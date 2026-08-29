//! What the meter costs to run, worked out from the settings it is running.
//!
//! This is here because "minimum energy" is not a property you can inspect by reading code. Every
//! choice in [`crate::config`] carries a comment saying why it is the low-energy one, and comments
//! do not add up. This does: it turns the settings into an estimated average current, so a change
//! to any of them can be checked rather than believed.
//!
//! # These are datasheet typicals, not measurements
//!
//! Every current below comes from the device datasheet at room temperature and nominal supply.
//! Nothing here has been on a meter. Real figures will be worse, in the way real figures always
//! are — colder, hotter, an older cell, a transducer that needs more drive than the guess. Treat
//! the answer as a way of comparing two configurations, which it is good at, and not as a battery
//! life, which it is not.
//!
//! # Where it goes
//!
//! Run the numbers with the defaults and the shape is not what intuition suggests. Neither the
//! ultrasound nor the radio is the biggest item. The correlation is — because the CPU has no hardware multiply linked
//! and runs thousands of multiply-accumulates per measurement, for tens of milliseconds, while the
//! analog front end that does the actual measuring is on for fifty microseconds. That single fact
//! is why [`crate::config::CORRELATION_SAMPLES`] exists and why it is the first thing to reach for.

use crate::config;

/// LPM3 with the crystal and the RTC running, in nanoamps.
///
/// What the machine draws for very nearly all of its life, and therefore the floor: no measurement
/// interval, however long, gets below this.
const SLEEP_NA: u32 = 1_500;

/// The CPU and its memory, running, in microamps.
///
/// About a hundred microamps per megahertz for an FRAM part at 8 MHz with no wait states.
const ACTIVE_UA: u32 = 800;

/// The ultrasonic front end while it is measuring, in microamps.
///
/// Supply, PLL, pulse generator, amplifier and converter together. Milliamps, which is why it is
/// on for microseconds.
const USS_ACTIVE_UA: u32 = 4_000;

/// How long the crystal and PLL take to start, in microseconds.
///
/// Paid on every measurement, since the front end is powered down in between — and larger than the
/// capture it enables, which is worth knowing before shortening the capture any further.
const STARTUP_US: u32 = 1_000;

/// The OLED module while it is showing text, in microamps.
///
/// Measured on a 0.96-inch module at 3.3 V, not a datasheet figure: the controller's own numbers
/// are quoted with no panel attached, and the panel is what the current is. Roughly linear in how
/// many pixels are lit, so an all-white screen is thirty times this.
const DISPLAY_UA: u32 = 630;

/// The CC1101 while transmitting at +10 dBm, in microamps.
const RADIO_TX_UA: u32 = 30_000;

/// How long a frame takes, in microseconds.
///
/// Some thirty bytes at 100 kbps is 2.4 ms on air; the rest is the synthesiser calibrating and the
/// registers being written, since the radio is configured from scratch every time — it loses them
/// in power-down, which is where it spends its life.
const RADIO_FRAME_US: u32 = 4_000;

/// Cycles per multiply-accumulate in the correlation.
///
/// With the software multiply routines linked, a 16 by 16 multiply is a subroutine call and a shift
/// and add loop. Fifty cycles is an estimate on the optimistic side.
const CYCLES_PER_MAC: u32 = 50;

/// A rough division of a measurement's cost.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Budget {
    /// Charge the front end takes for one measurement, in nanocoulombs.
    pub front_end_nc: u32,
    /// Charge the correlation takes, in nanocoulombs.
    pub correlation_nc: u32,
    /// Average current over a measurement interval, in nanoamps, including sleep.
    pub average_na: u32,
    /// Microseconds the front end is powered for.
    pub front_end_us: u32,
    /// Microseconds the CPU spends correlating.
    pub correlation_us: u32,
    /// Average current the display costs, in nanoamps, spread over the day.
    pub display_na: u32,
    /// Average current the radio costs, in nanoamps.
    pub radio_na: u32,
}

/// How long one capture takes, in microseconds.
const fn capture_us(pll_hz: u32, oversampling: u32, samples: u32) -> u32 {
    // Two pings, each of `samples` samples, each sample taking `oversampling` modulator cycles.
    let cycles = 2 * samples * oversampling;
    // Rounded up, so a capture is never counted as free.
    (cycles as u64 * 1_000_000).div_ceil(pll_hz as u64) as u32
}

/// How long the correlation takes, in microseconds.
const fn correlation_us(window: u32, max_lag: u32, mclk_hz: u32) -> u32 {
    // One pass per lag across the search, plus the two either side of the peak for the parabola.
    let passes = 2 * max_lag + 1 + 2;
    let macs = passes * window;
    (macs as u64 * CYCLES_PER_MAC as u64 * 1_000_000).div_ceil(mclk_hz as u64) as u32
}

/// Work the budget out for the settings in [`crate::config`].
///
/// `interval_s` is which of the two measurement intervals to price — the meter uses both, so the
/// honest answer is a pair of numbers rather than one.
pub const fn budget(interval_s: u32) -> Budget {
    let uss = config::uss();
    let mclk_hz = 8_000_000;

    let front_end_us =
        STARTUP_US + capture_us(uss.pll_hz, uss.oversampling.ratio(), config::SAMPLES as u32);
    let correlation_us = correlation_us(
        config::CORRELATION_SAMPLES as u32,
        config::MAX_LAG as u32,
        mclk_hz,
    );

    // Microamps times microseconds is picocoulombs; a thousand of those is a nanocoulomb.
    let front_end_nc = (USS_ACTIVE_UA * front_end_us) / 1_000;
    // The CPU is running for the whole of the front end's time too, plus the correlation.
    let correlation_nc = (ACTIVE_UA * (correlation_us + front_end_us)) / 1_000;

    // The display is priced per day rather than per measurement, because that is how it is used:
    // a few presses, unrelated to how often the water is measured. The measuring that goes on while
    // it is up is counted too -- it runs faster than usual then, and paying for it here is what
    // keeps this budget describing the instrument rather than a simplified version of it.
    let display_seconds_per_day = config::DISPLAY_SHOW_SECONDS * config::DISPLAY_VIEWS_PER_DAY;
    let watched_measurements = display_seconds_per_day / config::INTERVAL_WATCHED_S as u32;
    let display_na = (DISPLAY_UA * display_seconds_per_day * 1000) / 86_400
        + (watched_measurements * (front_end_nc + correlation_nc)) / 86_400;

    // The radio has an interval of its own, unrelated to how often the water is measured.
    let radio_nc = (RADIO_TX_UA / 1000) * RADIO_FRAME_US;
    let radio_na = radio_nc / config::BROADCAST_INTERVAL_S as u32;

    // Charge per measurement, spread over the interval, plus what sleeping costs.
    let per_measurement_nc = front_end_nc + correlation_nc;
    let average_na = SLEEP_NA + per_measurement_nc / interval_s + display_na + radio_na;

    Budget {
        front_end_nc,
        correlation_nc,
        average_na,
        front_end_us,
        correlation_us,
        display_na,
        radio_na,
    }
}

/// The budget while water is moving.
pub const FLOWING: Budget = budget(config::INTERVAL_FLOWING_S as u32);
/// The budget while nothing is moving, which is what a domestic meter does almost all the time.
pub const IDLE: Budget = budget(config::INTERVAL_IDLE_S as u32);

/// Years a cell of `capacity_mah` would last at `average_na`, ignoring self-discharge.
///
/// Ignoring self-discharge is a real omission: a lithium thionyl chloride cell loses something like
/// a per cent a year on its own, which at these currents is a serious fraction of the total. So
/// this is an upper bound, and a generous one.
pub const fn years(capacity_mah: u32, average_na: u32) -> u32 {
    if average_na == 0 {
        return 0;
    }
    // mAh to nAh is a factor of a million; hours to years is 8766.
    ((capacity_mah as u64 * 1_000_000) / (average_na as u64 * 8766)) as u32
}
