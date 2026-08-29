//! The parameters that make one instrument different from another, and the seal over them.
//!
//! # Why these are not constants
//!
//! Because every instrument is different. The acoustic geometry varies with the moulding, the
//! transducers vary with the batch, and what turns a difference in flight time into litres is
//! settled per meter on a flow rig — not in a header file. Firmware that hard-codes the geometry
//! can be a demonstration; it cannot be a product, because there is nowhere to put the number the
//! calibration produced.
//!
//! # WELMEC 7.2 P7, parameter protection
//!
//! Device-specific parameters have to be secured against unauthorised change. This does it three
//! ways, none of them cryptography and all of them things a notified body will recognise:
//!
//! * **A seal.** Once [`SEALED`] is set the write path refuses further changes, and the firmware
//!   offers no way to clear it.
//! * **A counter that only goes up.** Every accepted write increments [`Params::writes`], and it is
//!   reported alongside the reading. An instrument whose counter has moved since its certificate
//!   was issued has been interfered with, whether or not anything else shows it. That is the audit
//!   trail in its smallest honest form.
//! * **A checksum**, which catches the accidental half of the problem — a battery pulled mid-write
//!   — rather than the deliberate half.
//!
//! What this deliberately does not claim is protection against someone holding a programmer. That
//! is what the physical seal on the instrument is for, and pretending otherwise in software would
//! be worse than not trying.

/// Layout version, so a later firmware knows what it is looking at.
const VERSION: u16 = 1;

/// The value that counts as sealed.
///
/// Neither zero nor all-ones, so that blank or erased FRAM cannot read as a sealed instrument.
pub const SEALED: u16 = 0x5ea1;

/// Everything settled per instrument.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Params {
    /// Layout version.
    pub version: u16,
    /// Serial number, as it appears on the instrument and in every radio frame.
    pub serial: u32,

    /// Length of the acoustic path, in micrometres.
    ///
    /// Micrometres rather than tenths of a millimetre because this is now a measured quantity
    /// rather than a nominal one.
    pub path_um: u32,
    /// The part of that path lying along the pipe, in micrometres.
    pub axial_um: u32,
    /// Cross-sectional area of the bore, in hundredths of a square millimetre.
    pub bore_area_mm2_100: u32,

    /// Correction applied to the computed volume, in parts per million, signed.
    ///
    /// What a flow rig actually produces: run the instrument against a reference, find it reads
    /// 0.4% low, put +4000 here. Everything the geometry does not capture ends up in this number.
    pub calibration_ppm: i32,

    /// The difference in flight time this instrument reports at zero flow, in picoseconds, signed.
    ///
    /// Subtracted from every measurement. It is not zero and it is not stable: the two directions
    /// are never quite symmetric, and that asymmetry is the largest error in a meter of this kind.
    /// Measured with the pipe full and still.
    pub zero_offset_ps: i32,

    /// Excitation frequency the transducers fitted to this instrument want, in hertz.
    pub excitation_hz: u32,
    /// Amplitude that counts as the burst having arrived, for these transducers.
    pub burst_threshold: i16,

    /// Set to [`SEALED`] once the instrument is calibrated.
    pub seal: u16,
    /// How many times these parameters have been written. Only ever increases.
    pub writes: u16,
    /// Checksum over everything above.
    pub crc: u16,
}

impl Default for Params {
    /// What an uncalibrated instrument holds.
    ///
    /// The geometry is a nominal 50 mm path at 45 degrees through a 20 mm bore — a shape to start
    /// from, not a calibration. Such an instrument is unsealed, and [`Params::is_calibrated`] says
    /// so.
    fn default() -> Self {
        Self {
            version: VERSION,
            serial: 0,
            path_um: 50_000,
            axial_um: 35_400,
            bore_area_mm2_100: 31_400,
            calibration_ppm: 0,
            zero_offset_ps: 0,
            excitation_hz: 1_000_000,
            burst_threshold: 400,
            seal: 0,
            writes: 0,
            crc: 0,
        }
    }
}

impl Params {
    /// Whether this instrument has been calibrated and sealed.
    ///
    /// An instrument that has not been must not be billed from, and the firmware says so rather
    /// than quietly reporting numbers derived from nominal geometry.
    pub fn is_calibrated(&self) -> bool {
        self.seal == SEALED
    }

    /// `L² / (2·D)` in micrometres — the geometry folded into the one constant the flow equation
    /// needs.
    ///
    /// Computed rather than stored, so that it cannot come to disagree with the two lengths it is
    /// made of.
    pub fn geometry_um(&self) -> u32 {
        if self.axial_um == 0 {
            return 0;
        }
        ((self.path_um as u64 * self.path_um as u64) / (2 * self.axial_um as u64)) as u32
    }

    /// CRC-16/CCITT over the fields, excluding the checksum itself.
    fn checksum(&self) -> u16 {
        let mut crc = 0xffffu16;
        let mut feed = |value: u32, bytes: usize| {
            for i in 0..bytes {
                crc ^= (((value >> (8 * i)) & 0xff) as u16) << 8;
                for _ in 0..8 {
                    crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
                }
            }
        };
        feed(self.version as u32, 2);
        feed(self.serial, 4);
        feed(self.path_um, 4);
        feed(self.axial_um, 4);
        feed(self.bore_area_mm2_100, 4);
        feed(self.calibration_ppm as u32, 4);
        feed(self.zero_offset_ps as u32, 4);
        feed(self.excitation_hz, 4);
        feed(self.burst_threshold as u16 as u32, 2);
        feed(self.seal as u32, 2);
        feed(self.writes as u32, 2);
        crc
    }
}

/// Where the parameters live, inside the FRAM the instrument reserves for its own data. Two copies,
/// as for the totals: a battery pulled mid-write can spoil at most one.
const PRIMARY: *mut Params = super::fram::PARAMS as *mut Params;
/// The fallback. See [`PRIMARY`].
const SHADOW: *mut Params = super::fram::PARAMS_SHADOW as *mut Params;

/// One slot each. See the same assertion in `totals`: the slots interleave, so outgrowing one
/// lands on the other block rather than on this one's backup.
const _: () = assert!(core::mem::size_of::<Params>() as u16 <= super::fram::SLOT);

fn valid(p: &Params) -> bool {
    p.version == VERSION && p.crc == p.checksum()
}

/// Read the instrument's parameters.
///
/// Falls back to the shadow, then to uncalibrated defaults. Defaults are not a silent failure:
/// [`Params::is_calibrated`] is false, and the meter is expected to act on that.
pub fn load() -> Params {
    // SAFETY: a fixed address in the FRAM region `memory.x` reserves, which the linker puts nothing
    // else in. A struct of integers has no invalid bit pattern; whether the contents mean anything
    // is what the checksum decides.
    let primary = unsafe { PRIMARY.read_volatile() };
    if valid(&primary) {
        return primary;
    }
    // SAFETY: as above.
    let shadow = unsafe { SHADOW.read_volatile() };
    if valid(&shadow) {
        return shadow;
    }
    Params::default()
}

/// Why a write was refused.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WriteError {
    /// The instrument is sealed. That is the point of the seal, and there is no override.
    Sealed,
    /// The write counter has saturated, which means something has written to this instrument tens
    /// of thousands of times and it should not be trusted.
    CounterExhausted,
}

/// Write new parameters, if the instrument is not sealed.
///
/// The counter is incremented here rather than by the caller, so that no path through the firmware
/// can change a parameter without the change being counted.
pub fn store(new: &Params) -> Result<(), WriteError> {
    let current = load();
    if current.is_calibrated() {
        return Err(WriteError::Sealed);
    }
    let writes = current.writes.checked_add(1).ok_or(WriteError::CounterExhausted)?;

    let mut out = *new;
    out.version = VERSION;
    out.writes = writes;
    out.crc = out.checksum();

    critical_section::with(|_| {
        // SAFETY: the same reserved FRAM as `load`. FRAM is written in place, byte addressable,
        // with no erase and no unlock since the MPU is left disabled. The critical section covers
        // the pair, so an interrupt cannot leave both copies mid-write.
        unsafe {
            SHADOW.write_volatile(out);
            PRIMARY.write_volatile(out);
        }
    });
    Ok(())
}
