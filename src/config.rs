//! Every number the meter runs on, and why each one is the one it is.
//!
//! This file exists because the brief was minimum energy, and on a battery meter almost every
//! design decision is an energy decision wearing a different hat. Rather than scatter those choices
//! through the code with a comment each, they are here together, so the trade behind any of them
//! can be found and argued with.
//!
//! # Where the energy goes
//!
//! Not where intuition puts it. A measurement is microseconds; a day is eighty-six thousand
//! seconds. So the numbers that decide battery life are, in order:
//!
//! 1. **How often it measures.** Everything else is a rounding error beside this.
//! 2. **How long the analog front end is powered for each measurement**, which is the capture
//!    length plus the crystal's start-up.
//! 3. **What the machine draws while asleep**, which is the sum of a lot of small things and is
//!    where an ill-chosen external component quietly costs more than the measuring does.
//!
//! The CPU's own consumption while computing barely registers, which is why this firmware is built
//! for size rather than speed.

use embassy_msp430::clock::{AclkSource, DcoFreq, Div};
use embassy_msp430::uss::{
    Channel, Config as UssConfig, ExcitationBias, Oversampling, PgaBias, Resolution,
};

// ---------------------------------------------------------------------------------------------
// Clocks
// ---------------------------------------------------------------------------------------------

/// MCLK, and with it SMCLK.
///
/// 8 MHz, which is the minimum-energy point for this part rather than a compromise. Above it the
/// FRAM needs wait states, so every instruction fetch costs an extra cycle and the energy per
/// instruction goes up; below it the work takes proportionally longer while the leakage and the
/// analog blocks carry on regardless. Run fast, finish, sleep.
pub const MCLK: DcoFreq = DcoFreq::_8MHz;

/// No division: see [`MCLK`]. Dividing MCLK would lengthen the active time without lowering the
/// energy per instruction.
pub const MCLK_DIV: Div = Div::_1;
/// Same, for the peripherals.
pub const SMCLK_DIV: Div = Div::_1;

/// ACLK, which is what runs while everything else is off.
///
/// The crystal, not the internal oscillator, and this is the one place where the lower-power option
/// is refused. The VLO would save a couple of hundred nanoamps and is accurate to tens of percent —
/// and ACLK is what times the measurement interval, which is what the flow is integrated over. A
/// clock twenty percent fast makes the totals twenty percent wrong, which is not a meter. Against
/// the milliamps the front end draws while measuring, the crystal's extra current does not appear
/// in the sum at all.
pub const ACLK: AclkSource = AclkSource::Lfxt;
/// Undivided, so the time driver gets its full 32768 Hz.
pub const ACLK_DIV: Div = Div::_1;

// ---------------------------------------------------------------------------------------------
// How often to measure
// ---------------------------------------------------------------------------------------------

/// Seconds between measurements while water is moving.
///
/// The single largest lever in the whole design. Every doubling of this halves the meter's energy,
/// and the only thing it costs is how quickly a change in flow is noticed — which for billing is
/// nothing, since the totals integrate either way.
pub const INTERVAL_FLOWING_S: u64 = 2;

/// Seconds between measurements when nothing has moved for a while.
///
/// A tap that is off stays off for hours at a time, and a meter that keeps pinging an unmoving pipe
/// is spending its battery to learn nothing. Coming back up to [`INTERVAL_FLOWING_S`] takes one
/// interval, so the most this can lose is one slow interval's worth of the very start of a draw.
pub const INTERVAL_IDLE_S: u64 = 30;

/// Consecutive still measurements before the meter slows down.
///
/// Not one: a single zero in the middle of a draw is more likely to be a missed echo than a closed
/// tap, and dropping to the slow rate on it would then delay noticing that the water is still
/// running.
pub const IDLE_MEASUREMENTS: u8 = 8;

/// Velocity below which the water counts as still, in micrometres per second.
///
/// Not zero. The measurement has noise, and around zero that noise is as likely to read positive as
/// negative — so a meter that integrated everything would accumulate a total from a closed tap,
/// wandering up or down depending on which way the noise leaned. Everything under this is treated
/// as no flow.
pub const ZERO_CUTOFF_UM_S: i32 = 4_000;

// ---------------------------------------------------------------------------------------------
// The acoustic path
// ---------------------------------------------------------------------------------------------
//
// Nothing here any more. The geometry, the calibration correction, the zero offset and the burst
// threshold are per instrument and live in `legal::params`, written on a flow rig and sealed. A
// constant here would be a number no calibration could reach.

// ---------------------------------------------------------------------------------------------
// The ultrasonic front end
// ---------------------------------------------------------------------------------------------

/// Samples captured per ping.
///
/// This is the second-largest energy lever, because the analog front end is powered for the whole
/// capture and the capture is this many samples long. It wants to be just long enough to contain
/// the burst with room either side for the correlation to slide — no longer. Two hundred samples
/// at 4 Msps is fifty microseconds.
pub const SAMPLES: usize = 200;

/// Which transducer transmits first. Only affects the sign of the answer.
pub const FIRST_CHANNEL: Channel = Channel::Ch0;

/// How many samples after the burst the correlation runs over.
///
/// The third energy lever, and an unobvious one. The correlation is the only part of a measurement
/// where the CPU does real work, and its cost is this times the number of lags — so shortening it
/// saves more than shortening the capture does, because the capture is analog time while this is
/// CPU time at eight megahertz with no hardware multiply linked.
///
/// Sixty-four samples covers the first several cycles of a 1 MHz burst at 4 Msps, which is where
/// the signal is strongest and most alike between the two directions. Correlating the tail as well
/// adds arithmetic and the noisiest part of the waveform.
pub const CORRELATION_SAMPLES: usize = 64;

/// How far the correlation searches, in samples.
///
/// A little more than the largest shift the plumbing can produce, and no more: the search is the
/// expensive part of the arithmetic, and every extra lag is another pass over the samples.
pub const MAX_LAG: i16 = 16;

/// The front end's own configuration.
///
/// The energy-relevant choices here:
///
/// * **Ten excitation pulses.** Fewer means less energy into the transducer and a weaker echo;
///   more means a longer ring-down that has to die away before the echo arrives. Ten is a common
///   starting point for a 1 MHz water transducer and wants checking against a real one.
/// * **Two stop pulses**, which damp the transmitter so it stops ringing sooner. That shortens the
///   capture window, which is energy.
/// * **Oversampling of 20**, the lowest that is likely to give usable noise on a 1 MHz burst. Every
///   step up quiets the signal and lengthens the capture proportionally.
/// * **The lowest biases** that should still drive the transducer and centre the amplifier. Both
///   are currents drawn for the whole measurement.
/// * **14-bit samples**, which cost nothing extra: the converter runs the same modulator either
///   way, and the resolution only changes how much of the result is kept.
pub const fn uss() -> UssConfig {
    // Start from the HAL's default and change what this meter wants. The struct is
    // `non_exhaustive`, so a literal is not allowed — and this reads better anyway, since only the
    // deliberate departures appear.
    let mut c = UssConfig::DEFAULT;
    c.pulses = 10;
    c.stop_pulses = 2;
    c.oversampling = Oversampling::_20;
    c.resolution = Resolution::_14Bit;
    // The lowest biases the part offers. Both are currents drawn for the whole of a measurement,
    // and both want raising if the echo turns out too small to see — which `uss_scope` in the HAL's
    // examples will show.
    c.excitation_bias = ExcitationBias::_0V2;
    c.pga_bias = PgaBias::_0V75;
    c
}

// ---------------------------------------------------------------------------------------------
// Housekeeping
// ---------------------------------------------------------------------------------------------

/// Measurements between checks of the battery and the temperature.
///
/// Rare on purpose. Both need the internal reference, which is a bias current that has to be
/// switched on and left to settle — tens of microseconds of it — and neither number changes on the
/// timescale of a measurement. Once every few minutes is generous.
pub const HOUSEKEEPING_EVERY: u16 = 128;

/// How long the display stays up after a press, in seconds.
///
/// Kept here rather than only in [`crate::display`] so that [`crate::energy`] can price it: this is
/// the number that decides whether the display is affordable, and it should be next to the other
/// numbers that decide the same thing.
pub const DISPLAY_SHOW_SECONDS: u32 = 45;

/// Seconds between measurements while somebody is watching the display.
///
/// Faster than either normal interval, and deliberately: the reason to stand in front of a meter
/// with nothing running is to see whether the last digits climb, and a reading that only moves
/// every two seconds is a poor instrument for that. Forty-five seconds of it costs about 34 nA
/// averaged over a day, which does not register.
pub const INTERVAL_WATCHED_S: u64 = 1;

/// How many times a day somebody is assumed to press the button.
///
/// Only used to price the display in [`crate::energy`]. Four is a guess about a household, and the
/// number is here rather than buried in that module because it is an assumption about people, not
/// about hardware — the kind that is worth being able to find and argue with.
pub const DISPLAY_VIEWS_PER_DAY: u32 = 4;

/// Millivolts below which the battery is called low.
///
/// A lithium thionyl chloride cell holds about 3.6 V until it is nearly finished and then falls
/// quickly, so this is set where there is still life to be collected rather than where the meter
/// would stop working.
pub const BATTERY_LOW_MV: u16 = 3_200;
