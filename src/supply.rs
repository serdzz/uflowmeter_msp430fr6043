//! The battery and the temperature — the two things the meter checks about itself.
//!
//! Both are read through the on-chip converter, and both are read rarely. The reason is the same
//! for each: they need the internal voltage reference, which is a bias current that has to be
//! switched on and left tens of microseconds to settle, and neither number changes on the timescale
//! of a measurement. Once every few minutes is generous, which is what
//! [`crate::config::HOUSEKEEPING_EVERY`] arranges.
//!
//! # Why nothing external
//!
//! The obvious way to watch a battery is a resistive divider to the converter. It is also the
//! wrong way on a meter that has to last a decade: a divider high enough not to load the cell is
//! high enough to be a poor source for a sampling converter, and one low enough to drive the
//! converter properly draws current continuously — a 1 MΩ pair across 3.6 V is 3.6 µA, which over
//! ten years is more charge than all the measuring.
//!
//! The device has a channel that reads half the supply internally, switched, drawing nothing
//! between conversions. That is the one this uses, and the missing external divider is a component
//! chosen for minimum energy by not existing.
//!
//! The temperature sensor is on-chip for the same kind of reason, though the accuracy argument
//! carries as much weight: [`crate::flow`] does not need temperature to work out velocity, so a
//! couple of degrees is fine and an external sensor would be current spent on precision nobody
//! uses.

use embassy_msp430::adc::{Adc, Channel, Config as AdcConfig, Reference, Resolution, SampleTime};
use embassy_msp430::peripherals::ADC12;
use embassy_msp430::Peri;

use crate::config;

/// What the meter knows about its own condition.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default)]
pub struct Health {
    /// Supply, in millivolts.
    pub supply_mv: u16,
    /// Die temperature, in tenths of a degree Celsius.
    ///
    /// Uncalibrated — see [`temperature_c10`]. Good for a trend and for the log, not for a
    /// correction that matters.
    pub temperature_c10: i16,
    /// Whether the battery has fallen below [`config::BATTERY_LOW_MV`].
    pub battery_low: bool,
}

/// Reads the battery and the die temperature.
pub struct Monitor<'d> {
    peri: Option<Peri<'d, ADC12>>,
}

impl<'d> Monitor<'d> {
    /// Take the converter, without powering it up.
    ///
    /// Deliberately not configured here. The converter and its reference are only switched on
    /// inside [`Monitor::read`] and switched off again on the way out, so between the few seconds a
    /// day they are needed they cost nothing.
    pub fn new(peri: Peri<'d, ADC12>) -> Self {
        Self { peri: Some(peri) }
    }

    /// Switch the converter on, take both readings, and switch it off.
    pub async fn read(&mut self) -> Health {
        let peri = match self.peri.take() {
            Some(peri) => peri,
            // Cannot happen: the peripheral is put back before this returns. Reporting zeroes
            // would be worse than reporting the previous state, so say nothing is known.
            None => return Health::default(),
        };

        // The 1.2 V reference: the lowest of the three, and the one whose generator draws least.
        // Nothing here needs the headroom of the others — half the supply of a 3.6 V cell is 1.8 V,
        // which is why `HalfAvcc` exists as a divided channel in the first place.
        let mut config = AdcConfig::default();
        config.reference = Reference::Internal1V2;
        config.resolution = Resolution::_12Bit;
        // A long sample time costs microseconds of converter, which against the reference's
        // settling time is nothing, and it is what makes an internal channel with a high source
        // impedance read correctly.
        config.sample_time = SampleTime::_256;

        let mut adc = Adc::new(peri, config);

        let supply_raw = adc.read_internal(Channel::HalfAvcc).await;
        let temperature_raw = adc.read_internal(Channel::TemperatureSensor).await;

        // The channel reads half the rail, so the answer doubles.
        let supply_mv = adc.to_millivolts(supply_raw).unwrap_or(0).saturating_mul(2);
        let temperature_c10 = temperature_c10(adc.to_millivolts(temperature_raw).unwrap_or(0));

        // Dropping the converter turns it and the reference back off.
        drop(adc);

        Health {
            supply_mv,
            temperature_c10,
            battery_low: supply_mv < config::BATTERY_LOW_MV,
        }
    }
}

/// Die temperature from the sensor's output voltage, in tenths of a degree.
///
/// **Uncalibrated.** The sensor is a diode whose voltage falls with temperature at a rate that is
/// consistent, from an offset that is not: part-to-part the offset varies by several degrees, which
/// is why TI stores a two-point calibration for each device in its `TLV` table. This does not read
/// that table, so the number is good for watching a trend and useless as an absolute.
///
/// Reading the table would fix it, and the reason not to is that nothing here needs it: the
/// velocity calculation has no temperature term at all, by construction. If a density correction is
/// ever added, this is the first thing to improve.
fn temperature_c10(millivolts: u16) -> i16 {
    // Typical figures for this family: about 690 mV at 30 °C, falling about 1.7 mV per degree.
    const MV_AT_30C: i32 = 690;
    const UV_PER_DEGREE: i32 = 1_700;

    let delta_uv = (millivolts as i32 - MV_AT_30C) * 1000;
    (300 + (delta_uv * 10) / UV_PER_DEGREE) as i16
}
