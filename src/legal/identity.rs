//! Software identification and the integrity check over it.
//!
//! Two of the requirements a measuring instrument's software has to meet under WELMEC 7.2, which
//! is the guide a notified body assesses MID software against:
//!
//! * **P2, software identification.** The legally relevant software has to be identifiable, and the
//!   identification has to be presentable on demand. Here that is a version string fixed at build
//!   time plus the checksum below, which together say exactly which binary is running.
//! * **P5, protection against accidental or unintentional changes.** The guide asks for the
//!   checksum to be recomputed periodically and compared against a deposited nominal value, with a
//!   reaction adequate to the instrument if it does not match. Here the reaction is to stop
//!   measuring: a meter that cannot vouch for its own code has no business adding to somebody's
//!   bill.
//!
//! # What is covered
//!
//! Everything from the start of the program region to the end of the `.data` initialisers — that
//! is, all the code and all the constants, the whole image bar the vector table. The device
//! parameters have their own checksum in [`super::params`], because they legitimately change when
//! a meter is calibrated and the code does not.
//!
//! # Why the hardware does it
//!
//! The device has a CRC32 module. It is used here rather than a loop for the reason everything else
//! in this firmware is the way it is: the check runs on every wake-up over some fourteen kilobytes,
//! and doing that in software would cost more energy than the measurement it protects.

use core::ptr::addr_of;

/// Identification of this build, as P2 requires.
///
/// The version is set here and must be changed whenever the legally relevant software changes —
/// which, since this instrument declares its whole software legally relevant, means any change at
/// all. A notified body will check that it does.
pub const SOFTWARE_VERSION: &str = "1.0";

/// The CRC32 module.
const CRC32: u16 = 0x0980;
const CRC32DIW0: u16 = 0x00;
const CRC32INIRESW0: u16 = 0x08;
const CRC32INIRESW1: u16 = 0x0a;

unsafe extern "C" {
    /// Start of the program region, from the linker script.
    static _legal_start: u8;
    /// Where the `.data` initialisers live in ROM — the last thing in the image.
    static _sidata: u8;
    /// Start of `.data` in RAM.
    static _sdata: u8;
    /// End of `.data` in RAM.
    static _edata: u8;
}

#[inline]
fn write(offset: u16, value: u16) {
    // SAFETY: a volatile write to a CRC32 register.
    unsafe { ((CRC32 + offset) as *mut u16).write_volatile(value) }
}

#[inline]
fn read(offset: u16) -> u16 {
    // SAFETY: a volatile read of a CRC32 register.
    unsafe { ((CRC32 + offset) as *mut u16).read_volatile() }
}

/// Checksum of the legally relevant image.
///
/// Deterministic for a given binary, which is what makes it usable as half of the software
/// identification: two instruments running the same approved software produce the same number, and
/// any change to the code changes it.
pub fn image_crc() -> u32 {
    let start = addr_of!(_legal_start) as u16;
    // The image ends where the `.data` initialisers end, and their length is `.data`'s length.
    let data_len = addr_of!(_edata) as u16 - addr_of!(_sdata) as u16;
    let end = addr_of!(_sidata) as u16 + data_len;

    // IEEE 802.3 CRC32, which is what the module implements and what the seed selects.
    write(CRC32INIRESW0, 0xffff);
    write(CRC32INIRESW1, 0xffff);

    let mut addr = start;
    while addr < end {
        // SAFETY: reading this device's own program memory, a word at a time, between two addresses
        // the linker placed. FRAM reads like RAM here.
        let word = unsafe { (addr as *const u16).read_volatile() };
        write(CRC32DIW0, word);
        addr = addr.wrapping_add(2);
    }

    ((read(CRC32INIRESW1) as u32) << 16) | read(CRC32INIRESW0) as u32
}

/// Size of the checked image, in bytes. Reported alongside the checksum so that a truncated image
/// cannot pass by accident.
pub fn image_len() -> u16 {
    let data_len = addr_of!(_edata) as u16 - addr_of!(_sdata) as u16;
    (addr_of!(_sidata) as u16 + data_len) - addr_of!(_legal_start) as u16
}
