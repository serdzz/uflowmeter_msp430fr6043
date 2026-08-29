//! Wireless M-Bus frames, EN 13757-4, mode C1.
//!
//! # Why C1 and not T1
//!
//! T1 is the mode most European water meters still use, and it encodes every byte as 3-out-of-6
//! symbols — a table, an encoder, and half again as many bits on air. C1 sends the bytes as they
//! are. It is the mode OMS now recommends for battery meters, the frame is shorter for the same
//! payload, and the flash the encoder would have cost is flash this instrument does not have.
//!
//! # What is on air
//!
//! A preamble and sync word the radio generates, then Format A: a length, a control field, who is
//! transmitting, and the data — with a CRC after the first ten bytes and after each sixteen
//! after that. The CRC is EN 13757's own, polynomial 0x3D65, which is not the CRC-16 anything else
//! uses.
//!
//! # What this is not
//!
//! **Unencrypted.** A real deployment runs OMS security profile A or B — AES-128 in mode 5 or
//! mode 7 — and a meter transmitting its reading in clear is both a privacy problem and something
//! no head-end will accept. The device has an AES256 accelerator sitting unused; wiring it up is
//! the next thing this module needs and it is not a small one, because the key management is the
//! hard part rather than the cipher.
//!
//! **Untested against a receiver.** Every field below is built from the standard as documented;
//! none of it has been read back by anything.

/// Manufacturer code, three letters packed into fifteen bits.
///
/// `const fn` so that a flat-code change is a one-line edit rather than a table lookup: each letter
/// is its position in the alphabet, five bits each, most significant first. A real product needs a
/// code allocated by the DLMS User Association; `LVA` is a placeholder and will collide with
/// somebody.
const fn manufacturer(a: u8, b: u8, c: u8) -> u16 {
    (((a - 64) as u16) << 10) | (((b - 64) as u16) << 5) | (c - 64) as u16
}

/// This instrument's manufacturer field. **A placeholder** — see [`manufacturer`].
const MANUFACTURER: u16 = manufacturer(b'L', b'V', b'A');

/// Device type 0x07: water meter.
const DEVICE_WATER: u8 = 0x07;
/// Version of this instrument's own frame layout, reported in the address field.
const GENERATION: u8 = 0x01;

/// C-field 0x44: SND-NR, a send with no reply expected.
///
/// The whole reason a battery meter can afford a radio: it says its piece and never listens.
const C_SND_NR: u8 = 0x44;

/// CI-field 0x7A: a short transport header, followed by data records.
const CI_SHORT_HEADER: u8 = 0x7a;

/// EN 13757's CRC: polynomial 0x3D65, initial value zero, final complement.
///
/// Not any of the common CRC-16s. Getting this wrong produces frames that look perfectly well
/// formed and that every receiver silently discards, which is a bad afternoon to debug.
fn crc(data: &[u8]) -> u16 {
    let mut crc = 0u16;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x3d65 } else { crc << 1 };
        }
    }
    !crc
}

/// The reading, as it goes on air.
pub struct Reading {
    /// The instrument's serial number, which becomes the address field.
    pub serial: u32,
    /// Volume in litres.
    pub litres: u32,
    /// Increments on every transmission, so a receiver can tell a repeat from a fresh reading and
    /// spot a gap.
    pub access: u8,
    /// Status byte: zero when all is well.
    pub status: u8,
}

/// Status bit: the instrument has not been calibrated, so nothing here may be billed from.
pub const STATUS_NOT_CALIBRATED: u8 = 0x04;
/// Status bit: the battery is low.
pub const STATUS_BATTERY_LOW: u8 = 0x08;
/// Status bit: no echo was found on the last measurements — air in the pipe, or a failing
/// transducer.
pub const STATUS_NO_ECHO: u8 = 0x10;

/// The longest frame this builds.
pub const MAX_FRAME: usize = 32;

/// Build a Format A frame into `out`, returning how much of it is used.
///
/// The layout, in order: the length, then a first block of ten bytes with its CRC, then one block
/// of data with its own. Two blocks is all this instrument needs — the reading is one record.
pub fn frame(reading: &Reading, out: &mut [u8; MAX_FRAME]) -> usize {
    // First block: everything that identifies the instrument.
    let head = [
        C_SND_NR,
        MANUFACTURER as u8,
        (MANUFACTURER >> 8) as u8,
        reading.serial as u8,
        (reading.serial >> 8) as u8,
        (reading.serial >> 16) as u8,
        (reading.serial >> 24) as u8,
        GENERATION,
        DEVICE_WATER,
        CI_SHORT_HEADER,
    ];

    // Second block: the short transport header, then one data record.
    //
    // DIF 0x0C is eight BCD digits; VIF 0x13 is volume in litres. Together they say "the following
    // four bytes are a volume in litres, packed two decimal digits per byte" -- which is what a
    // water meter reports and what any head-end will already know how to read.
    let mut body = [0u8; 12];
    body[0] = reading.access;
    body[1] = reading.status;
    // Configuration word: no encryption. This is the field that says so, and it is the field that
    // will change first when the AES accelerator is put to work.
    body[2] = 0x00;
    body[3] = 0x00;
    body[4] = 0x0c;
    body[5] = 0x13;
    let bcd = to_bcd8(reading.litres);
    body[6] = bcd as u8;
    body[7] = (bcd >> 8) as u8;
    body[8] = (bcd >> 16) as u8;
    body[9] = (bcd >> 24) as u8;
    let body_len = 10;

    // The L-field counts everything after itself except the CRCs.
    out[0] = (head.len() + body_len) as u8;

    let mut n = 1;
    out[n..n + head.len()].copy_from_slice(&head);
    let head_crc = crc(&head);
    n += head.len();
    out[n] = (head_crc >> 8) as u8;
    out[n + 1] = head_crc as u8;
    n += 2;

    out[n..n + body_len].copy_from_slice(&body[..body_len]);
    let body_crc = crc(&body[..body_len]);
    n += body_len;
    out[n] = (body_crc >> 8) as u8;
    out[n + 1] = body_crc as u8;
    n + 2
}

/// Eight decimal digits packed two to a byte, least significant byte first.
///
/// What M-Bus calls BCD and what the DIF above promises. Values that do not fit saturate at
/// 99 999 999 rather than wrapping: a meter that has counted past the field it reports in should
/// read as stuck at the top, not as having gone back to zero.
fn to_bcd8(mut value: u32) -> u32 {
    if value > 99_999_999 {
        value = 99_999_999;
    }
    let mut out = 0u32;
    let mut shift = 0;
    while shift < 32 {
        out |= (value % 10) << shift;
        value /= 10;
        shift += 4;
    }
    out
}
