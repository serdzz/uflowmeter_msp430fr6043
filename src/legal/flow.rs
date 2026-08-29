//! Turning a difference in flight time into a volume.
//!
//! # The equation, and why this one
//!
//! For a path of length `L` whose axial component is `D`, the mean velocity along the pipe is
//!
//! ```text
//!         L²         Δt
//!  v =  ------  ·  ---------
//!        2 · D      t↑ · t↓
//! ```
//!
//! The important thing about this form is what is *not* in it: the speed of sound. The obvious
//! formulation needs it, and then the meter needs to know the water temperature accurately, and
//! then it needs a temperature sensor good to a fraction of a degree — an external part, excited
//! by a current, read through a divider, all of it drawing power forever. This form divides that
//! problem out. The two flight times carry the speed of sound and it cancels.
//!
//! That is an energy decision as much as an accuracy one, and it is why [`crate::supply`] reads the
//! on-chip temperature sensor rather than anything better: temperature is wanted for the density
//! correction and the log, not for the velocity, so a couple of degrees is fine.
//!
//! # Arithmetic on a machine with no divider
//!
//! Everything here is integers. The one 64-bit multiply is unavoidable — the numerator genuinely
//! spans more than 32 bits — but it happens once per measurement, a couple of times a second at
//! most, so its cost does not appear in the energy budget. Everything else is arranged to stay
//! inside 32 bits.

use super::params::Params;

/// A measurement, once the arithmetic has been done.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Flow {
    /// Mean velocity along the pipe, in micrometres per second. Signed: negative is backwards.
    pub velocity_um_s: i32,
    /// Volumetric rate, in cubic millimetres per second — which is microlitres per second.
    pub rate_ul_s: i32,
    /// Whether this counts as water moving at all. See [`config::ZERO_CUTOFF_UM_S`].
    pub moving: bool,
}

/// Work out the flow from a difference in flight time and the two absolute flight times.
///
/// `delta_t_ps` is the precise quantity, from the correlation. The two flight times are the coarse
/// ones from the threshold, and coarse is all they need to be: they appear only as a product in the
/// denominator, where a per-cent error is a per-cent error in the scale factor — which is what
/// calibration is for — while the same error in `delta_t_ps` would be the whole reading.
pub fn compute(params: &Params, delta_t_ps: i32, t_up_ns: u32, t_down_ns: u32) -> Option<Flow> {
    // A flight time of zero means the burst was found at the very start of the capture, which means
    // it was not really found. Dividing by it would be worse than saying so.
    if t_up_ns == 0 || t_down_ns == 0 {
        return None;
    }

    // The instrument's own asymmetry comes off first. Everything downstream is the flow; this is
    // not, and leaving it in would make a closed tap read as a steady trickle in whichever
    // direction the hardware happens to lean.
    let delta_t_ps = delta_t_ps - params.zero_offset_ps;

    let denominator = t_up_ns as i64 * t_down_ns as i64;
    let numerator = 1_000_000i64 * params.geometry_um() as i64 * delta_t_ps as i64;
    let velocity_um_s = (numerator / denominator) as i32;

    let moving = velocity_um_s.unsigned_abs() >= crate::config::ZERO_CUTOFF_UM_S as u32;

    // A micrometre per second through a square millimetre is a thousandth of a cubic millimetre per
    // second, so the area multiplies and a thousand divides. The area is in hundredths of a square
    // millimetre, which puts another hundred under the line.
    let rate_ul_s = if moving {
        let raw = (velocity_um_s as i64 * params.bore_area_mm2_100 as i64) / 100_000;
        // The calibration correction, in parts per million. This is where the flow rig's verdict
        // lands, and it is applied last so that it corrects everything above it at once.
        (raw + (raw * params.calibration_ppm as i64) / 1_000_000) as i32
    } else {
        0
    };

    Some(Flow {
        velocity_um_s,
        rate_ul_s,
        moving,
    })
}

/// How much water went past in `seconds` at `rate_ul_s`.
///
/// Rectangular integration, which is the right shape here: the meter does not know what happened
/// between measurements, and pretending it was a straight line between two samples would be an
/// invention. Making the interval short when water is moving is what keeps this honest, and is why
/// [`config::INTERVAL_FLOWING_S`] is what it is.
pub fn volume_ul(rate_ul_s: i32, seconds: u32) -> i32 {
    rate_ul_s.saturating_mul(seconds as i32)
}
