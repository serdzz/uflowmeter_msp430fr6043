//! The measurement cycle.
//!
//! Ping both ways, work out the flow, add it to the reading, switch the front end off, sleep. The
//! sleeping is most of it: at two seconds between measurements the machine spends something like
//! 99.9% of its life in LPM3 with only the crystal running, and the whole design is arranged to
//! keep that fraction as close to one as it can be.
//!
//! # The adaptive interval
//!
//! A meter on a pipe where nothing is flowing learns nothing by asking often, and the asking is
//! what costs. So after [`config::IDLE_MEASUREMENTS`] consecutive still readings the interval
//! stretches from [`config::INTERVAL_FLOWING_S`] to [`config::INTERVAL_IDLE_S`], and the first
//! measurement that finds water moving pulls it straight back.
//!
//! What that costs is the start of a draw: at worst the meter is most of a slow interval late in
//! noticing a tap opening, and that fraction of the flow goes uncounted. Against a domestic pattern
//! — hours of nothing, then minutes of something — it is a very cheap trade, and it is the largest
//! single saving in the design.

use embassy_msp430::uss::{tof, Uss};
use embassy_time::Duration;

use crate::config;
use crate::legal::flow;
use crate::supply::{Health, Monitor};
use crate::legal::params::Params;
use crate::legal::totals::{self, Totals};

/// Everything one measurement cycle needs.
pub struct Meter<'d> {
    /// The ultrasonic front end.
    pub uss: Uss<'d>,
    /// The battery and temperature monitor.
    pub monitor: Monitor<'d>,
    /// The reading.
    pub totals: Totals,
    /// What this particular instrument was calibrated to. Read once at start-up: they cannot change
    /// while it is running, because the only path that writes them refuses a sealed instrument.
    pub params: Params,
    /// The last thing the monitor said.
    pub health: Health,
    /// Consecutive measurements that found no movement.
    still: u8,
    /// Measurements since the last housekeeping check.
    since_housekeeping: u16,
    /// Sample buffer. Both directions, back to back, because that is how the transfer controller
    /// fills it.
    buffer: [i16; config::SAMPLES * 2],
}

/// What one cycle came to, for whoever is reporting.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// Water is moving, at this many microlitres per second.
    Flowing(i32),
    /// Everything worked and nothing is moving.
    Still,
    /// No echo was found. Air in the pipe, or a transducer going deaf.
    NoEcho,
    /// The front end itself failed.
    FrontEndFailed,
    /// The instrument has not been calibrated and sealed, so nothing it measures may be billed
    /// from. It still counts, so that a bench instrument is useful, but the reading is marked.
    NotCalibrated,
}

impl<'d> Meter<'d> {
    /// Build a meter around a front end and a monitor, picking up whatever reading is in FRAM.
    pub fn new(uss: Uss<'d>, monitor: Monitor<'d>, params: Params) -> Self {
        Self {
            uss,
            monitor,
            totals: totals::load(),
            params,
            health: Health::default(),
            still: 0,
            since_housekeeping: 0,
            buffer: [0; config::SAMPLES * 2],
        }
    }

    /// How long to wait before the next measurement.
    pub fn interval(&self) -> Duration {
        if self.still >= config::IDLE_MEASUREMENTS {
            Duration::from_secs(config::INTERVAL_IDLE_S)
        } else {
            Duration::from_secs(config::INTERVAL_FLOWING_S)
        }
    }

    /// How many seconds the last interval was, which is what the volume is integrated over.
    fn interval_seconds(&self) -> u32 {
        if self.still >= config::IDLE_MEASUREMENTS {
            config::INTERVAL_IDLE_S as u32
        } else {
            config::INTERVAL_FLOWING_S as u32
        }
    }

    /// Run one measurement and fold it into the reading.
    pub async fn measure(&mut self) -> Outcome {
        // An instrument that has not been calibrated and sealed still measures -- that is what
        // makes it useful on a bench -- but the outcome says so, every time, so that no reading
        // from it can be mistaken for one that may be billed from.
        let uncalibrated = !self.params.is_calibrated();

        // The volume this measurement stands for covers the interval that has just elapsed, which
        // is the one the meter was on *before* this reading changes it.
        let seconds = self.interval_seconds();

        // Powering the front end down between measurements means coming back is the whole bring-up:
        // supply, crystal, PLL. About a millisecond, against seconds of everything switched off.
        if self.uss.restart().is_err() {
            return Outcome::FrontEndFailed;
        }

        let outcome = self.measure_once(seconds).await;

        // Off again before anything else happens, including the arithmetic — the correlation takes
        // tens of milliseconds on the CPU and there is no reason to hold the analog section up for
        // it.
        self.uss.power_down();

        self.totals.measurements = self.totals.measurements.saturating_add(1);
        if matches!(outcome, Outcome::NoEcho) {
            self.totals.missed = self.totals.missed.saturating_add(1);
        }

        // Written every measurement, which on flash would be unthinkable and on FRAM costs about a
        // hundred nanojoules. See `totals`.
        totals::store(&mut self.totals);

        self.since_housekeeping += 1;
        if self.since_housekeeping >= config::HOUSEKEEPING_EVERY {
            self.since_housekeeping = 0;
            self.health = self.monitor.read().await;
        }

        if uncalibrated {
            return Outcome::NotCalibrated;
        }
        outcome
    }

    /// The measurement proper, with the front end already up.
    async fn measure_once(&mut self, seconds: u32) -> Outcome {
        let config = *self.uss.config();

        let (delta_t_ps, t_up_ns, t_down_ns) = {
            let Ok((up, down)) = self
                .uss
                .capture_pair(config::FIRST_CHANNEL, &mut self.buffer)
                .await
            else {
                return Outcome::FrontEndFailed;
            };

            // The window is counted from the burst, not from the start of the capture. See
            // `config::CORRELATION_SAMPLES`: this is where most of the CPU energy in a measurement
            // is decided.
            let Some(m) = tof::analyse(
                up,
                down,
                &config,
                // From the instrument's own parameters: the transducers fitted to it decide what
                // counts as a burst, and that is a per-batch number.
                self.params.burst_threshold,
                crate::config::MAX_LAG,
                crate::config::CORRELATION_SAMPLES,
            ) else {
                self.still = self.still.saturating_add(1);
                return Outcome::NoEcho;
            };

            (
                m.delta_t_ps,
                tof::flight_time_ns(m.start_first, &config),
                tof::flight_time_ns(m.start_second, &config),
            )
        };

        let Some(f) = flow::compute(&self.params, delta_t_ps, t_up_ns, t_down_ns) else {
            self.still = self.still.saturating_add(1);
            return Outcome::NoEcho;
        };

        if !f.moving {
            self.still = self.still.saturating_add(1);
            return Outcome::Still;
        }
        self.still = 0;

        let volume = flow::volume_ul(f.rate_ul_s, seconds);
        if volume >= 0 {
            self.totals.add_forward(volume as u32);
        } else {
            self.totals.add_reverse(volume.unsigned_abs());
        }

        Outcome::Flowing(f.rate_ul_s)
    }
}
