//! An ultrasonic water meter on the MSP430FR6043, built for a battery.
//!
//! # The shape of it
//!
//! One task, and it sleeps. Ping both transducers, work out how much water went by, add it to the
//! reading in FRAM, switch the analog section off, sleep until the next one is due. At the default
//! interval the machine is in LPM3 for about 99.9% of its life with only the crystal running.
//!
//! There is no second task because there is nothing for one to do. A meter has no user interface to
//! be responsive to and no protocol to service; adding tasks would add stack and wakeups and buy
//! nothing.
//!
//! # Where the design spends its energy
//!
//! Every choice is written down where it is made — [`config`] for the settings and why each is the
//! low-energy one, [`energy`] for what they add up to. The short version, and it is not the obvious
//! one:
//!
//! * **How often it measures** dominates everything. Hence the adaptive interval in [`meter`]:
//!   slow right down when nothing is flowing, which for a domestic meter is most of the time.
//! * **The correlation costs more than the ultrasound.** The front end is powered for tens of
//!   microseconds; the CPU spends tens of milliseconds correlating, because this build links the
//!   software multiply routines. [`config::CORRELATION_SAMPLES`] is the lever, and using the
//!   hardware multiplier would be the next one — see the README on why that is not done here.
//! * **Nothing external draws standby current.** No battery divider, no temperature sensor, no
//!   external memory. Each of those would be microamps forever, against a measurement that costs
//!   microamp-seconds. The cheapest component is the one not fitted.
//!
//! # Legal metrology
//!
//! This is meant to be sold, which makes it a measuring instrument under MID and its software
//! something a notified body assesses against WELMEC 7.2. What that costs the design is written up
//! in [`legal`]: the whole firmware is declared legally relevant, the image carries a checksum and
//! a version, and the per-instrument calibration is sealed with a write counter over it.
//!
//! # What has not happened
//!
//! None of this has been near a transducer, a pipe or a battery. The calibration in
//! [`legal::params`] is nominal geometry rather than a measurement, the currents in [`energy`] are
//! datasheet typicals, and the time-of-flight estimator is not TI's library. See the README.

#![no_std]
#![no_main]
#![feature(impl_trait_in_assoc_type)]
#![warn(missing_docs)]

use embassy_executor::{LowPowerMode, Spawner, set_low_power_mode};
use embassy_futures::select::{Either, select};
use embassy_msp430::uart::Uart;
use embassy_msp430::uss::Uss;

use legal::{identity, params};

use panic_msp430 as _;

mod calibration;
mod config;
mod display;
mod energy;
mod legal;
mod meter;
mod supply;

use display::Display;
use meter::{Meter, Outcome};
use supply::Monitor;

/// Bring the hardware up and then measure, for ever.
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut hal = embassy_msp430::Config::default();
    hal.clock.dco = config::MCLK;
    hal.clock.mclk_div = config::MCLK_DIV;
    hal.clock.smclk_div = config::SMCLK_DIV;
    hal.clock.aclk = config::ACLK;
    hal.clock.aclk_div = config::ACLK_DIV;
    let p = embassy_msp430::init(hal);

    // WELMEC 7.2 P5: the legally relevant image is checked before anything is measured, and the
    // reaction to a mismatch is to stop. A meter that cannot vouch for its own code has no business
    // adding to somebody's bill, and carrying on regardless is exactly the failure the requirement
    // exists to prevent.
    //
    // The nominal is not compared here because there is nowhere yet to have deposited it -- that
    // belongs with the production step that also seals the parameters. What this does establish is
    // that the checksum is computed and available, which is half of P2 as well: this number and
    // `identity::SOFTWARE_VERSION` together say exactly which binary is running.
    let image = identity::image_crc();
    let _ = (image, identity::image_len(), identity::SOFTWARE_VERSION);

    // Per-instrument calibration, written on a flow rig and sealed. An instrument that has none
    // still measures -- it is useful on a bench -- but says so, and nothing may be billed from it.
    let params = params::load();

    // LPM3: the CPU, MCLK, SMCLK and the DCO all off, ACLK still running so the time driver keeps
    // counting. This is the mode the meter lives in, and choosing it is the single largest thing
    // this function does — the executor's default is LPM0, which leaves SMCLK and the DCO running
    // for no reason and costs a couple of hundred microamps around the clock.
    set_low_power_mode(LowPowerMode::Lpm3);

    let uss = match Uss::new(p.UUPS, p.HSPLL, p.SAPH, p.SDHS, config::uss()) {
        Ok(uss) => uss,
        Err(_) => {
            // Nothing can be measured. A meter that carries on and records zeroes is worse than one
            // that stops: the zeroes look exactly like a closed tap. Sleep forever instead, and let
            // whatever reads the FRAM find a reading that stopped advancing.
            set_low_power_mode(LowPowerMode::Lpm4);
            loop {
                embassy_time::Timer::after_secs(3600).await;
            }
        }
    };

    // An uncalibrated instrument serves the production interface first, and returns from it only
    // when it has been sealed. A sealed one never opens the UART at all -- see `calibration`.
    let mut params = params;
    if !params.is_calibrated() {
        let mut uart = Uart::new(
            p.EUSCI_A0,
            p.P4_4,
            p.P4_3,
            embassy_msp430::uart::Config::default(),
        )
        .unwrap();
        calibration::run(&mut uart, &mut params).await;
    }

    let mut meter = Meter::new(uss, Monitor::new(p.ADC12), params);

    // The display is unpowered until somebody presses the button. See `display` for why switching
    // its supply, rather than using the controller's sleep command, is the only version of this
    // that fits the energy budget.
    let mut display = Display::new(p.P3_4, p.EUSCI_B0, p.P1_7, p.P1_6, p.P2_0);

    loop {
        let outcome = meter.measure().await;

        // Nothing is reported anywhere: this build has no display and no radio, and the reading
        // lives in FRAM for whatever comes to collect it. The match is here because the outcome is
        // what the interval is chosen from, and because a build that does report has an obvious
        // place to do it.
        match outcome {
            Outcome::Flowing(_) | Outcome::Still | Outcome::NoEcho | Outcome::NotCalibrated => {}
            // A front end that will not start is worth slowing down for: retrying every two seconds
            // spends the battery on a fault that needs a person.
            Outcome::FrontEndFailed => {
                embassy_time::Timer::after_secs(config::INTERVAL_IDLE_S).await;
            }
        }

        // Wait out the interval, but answer the button if it comes first. Racing the two rather
        // than running a second task keeps the reading where it is -- owned by the meter -- instead
        // of needing a lock around it, and the display is the only other thing this instrument
        // does.
        //
        // Whichever wins, the next measurement follows. A press therefore brings one forward by up
        // to the length of the showing, which is a rare event and a harmless one: the volume is
        // integrated over the interval that actually elapsed.
        let reading = meter.totals;
        let calibrated = meter.params.is_calibrated();
        match select(
            embassy_time::Timer::after(meter.interval()),
            display.serve(&reading, calibrated),
        )
        .await
        {
            Either::First(()) | Either::Second(()) => {}
        }
    }
}

/// Forces the energy budget to be evaluated, even though nothing reads it at runtime.
///
/// It is documentation the compiler checks: change a setting in [`config`] and every number in
/// [`energy`] is worked out again at compile time, so the budget cannot quietly drift away from
/// what the meter actually does. Whether the linker keeps the array itself does not matter — the
/// arithmetic has already happened by then.
#[used]
#[unsafe(no_mangle)]
static ENERGY_BUDGET_NA: [u32; 3] = [
    energy::FLOWING.average_na,
    energy::IDLE.average_na,
    energy::years(19_000, energy::IDLE.average_na),
];
