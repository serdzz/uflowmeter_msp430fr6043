//! The meter's reading, kept in FRAM.
//!
//! # Why this is three lines of code and not a journal
//!
//! On a flash part, storing a number that changes every two seconds is a real problem. Flash cannot
//! rewrite a word in place; it needs a sector erase first, which takes milliseconds, holds the CPU,
//! costs a few millijoules, and wears the sector out after some ten thousand goes. The usual answer
//! is a journal — append each new record after the last, erase only when the sector fills — which
//! is a few hundred lines of careful code and still burns the flash out eventually.
//!
//! FRAM has none of that. A write is a write: byte-addressable, in place, no erase, about a
//! hundred nanojoules, and rated for something like 10^15 cycles — which at one write every two
//! seconds is longer than the universe has been around. So the totals live in a plain struct at a
//! fixed address and are updated by assigning to it.
//!
//! This is the single clearest reason an FRAM part suits a battery meter, and it is worth being
//! explicit that the simplicity here is the *point* rather than a shortcut.
//!
//! # What is still needed
//!
//! A checksum, because a write interrupted by the battery being pulled leaves half a value, and a
//! meter that comes back with a plausible-looking wrong reading is worse than one that admits it
//! lost count. And a shadow copy, so there is something to fall back to: the two are written one
//! after the other, so a power cut can spoil at most one of them.

/// Layout version, so a later firmware can tell it is looking at an older shape rather than
/// misreading the bytes.
const VERSION: u16 = 1;

/// What the meter has counted.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct Totals {
    /// Layout version.
    pub version: u16,
    /// Whole litres delivered.
    pub litres: u32,
    /// Microlitres on top of that, always under a million.
    ///
    /// Kept separately rather than as one big number so that the small volumes a single measurement
    /// contributes are not lost to rounding — two seconds of a slow trickle is a few hundred
    /// microlitres, and a total in litres alone would throw every one of them away.
    pub microlitres: u32,
    /// Litres that went backwards, counted separately rather than subtracted.
    ///
    /// Subtracting would hide it. Reverse flow through a domestic meter means a fault or a theft,
    /// and it is exactly what somebody would want to see.
    pub reverse_litres: u32,
    /// How many measurements have been made, for working out what the battery has been spent on.
    pub measurements: u32,
    /// Measurements where no echo was found.
    ///
    /// A rising count here is the earliest sign of air in the pipe or a failing transducer, long
    /// before the readings themselves look wrong.
    pub missed: u32,
    /// Checksum over everything above.
    pub crc: u16,
}

impl Default for Totals {
    fn default() -> Self {
        Self {
            version: VERSION,
            litres: 0,
            microlitres: 0,
            reverse_litres: 0,
            measurements: 0,
            missed: 0,
            crc: 0,
        }
    }
}

/// A litre, in microlitres.
const UL_PER_LITRE: u32 = 1_000_000;

impl Totals {
    /// Add `microlitres` to the reading, carrying into litres.
    pub fn add_forward(&mut self, microlitres: u32) {
        self.microlitres += microlitres;
        while self.microlitres >= UL_PER_LITRE {
            self.microlitres -= UL_PER_LITRE;
            self.litres = self.litres.saturating_add(1);
        }
    }

    /// Count water that went the wrong way.
    ///
    /// Only whole litres, because the interesting question about reverse flow is whether it is
    /// happening at all, not exactly how much.
    pub fn add_reverse(&mut self, microlitres: u32) {
        self.reverse_litres = self
            .reverse_litres
            .saturating_add(microlitres / UL_PER_LITRE);
    }

    /// CRC-16/CCITT over the fields, excluding the checksum itself.
    fn checksum(&self) -> u16 {
        let mut crc = 0xffffu16;
        let mut feed = |value: u32, bytes: usize| {
            for i in 0..bytes {
                crc ^= (((value >> (8 * i)) & 0xff) as u16) << 8;
                for _ in 0..8 {
                    crc = if crc & 0x8000 != 0 {
                        (crc << 1) ^ 0x1021
                    } else {
                        crc << 1
                    };
                }
            }
        };
        feed(self.version as u32, 2);
        feed(self.litres, 4);
        feed(self.microlitres, 4);
        feed(self.reverse_litres, 4);
        feed(self.measurements, 4);
        feed(self.missed, 4);
        crc
    }
}

/// Where the two copies live.
///
/// Fixed addresses inside the region `memory.x` set aside, rather than a `link_section`. A named
/// section would have to be woven into `msp430-rt`'s linker script, and the whole point of the
/// region is that nothing else is allowed there — so an address is both simpler and exactly as
/// safe.
///
/// Two copies, written one after the other, so that a battery pulled mid-write can spoil at most
/// one.
const PRIMARY: *mut Totals = 0x6000 as *mut Totals;
/// The fallback. See [`PRIMARY`].
const SHADOW: *mut Totals = 0x6080 as *mut Totals;

/// The region is 256 bytes and holds two of these; if the struct ever outgrows that the build
/// should stop rather than let the copies overlap.
const _: () = assert!(core::mem::size_of::<Totals>() <= 0x80);

/// Whether a stored copy can be believed.
fn valid(t: &Totals) -> bool {
    t.version == VERSION && t.crc == t.checksum()
}

/// Read the meter's reading back.
///
/// Takes the primary if it is intact, the shadow if it is not, and a fresh zero if neither is —
/// which is what a meter that has never been switched on looks like, and is indistinguishable from
/// one whose FRAM has been wiped. Both want the same thing: start counting.
pub fn load() -> Totals {
    // SAFETY: `PRIMARY` and `SHADOW` are inside the FRAM region `memory.x` reserves, which the
    // linker puts nothing else in. FRAM reads like RAM, and a struct of integers has no invalid
    // bit pattern, so whatever is there is a `Totals` — possibly a nonsensical one, which is what
    // the checksum is for.
    let primary = unsafe { PRIMARY.read_volatile() };
    if valid(&primary) {
        return primary;
    }
    // SAFETY: as above.
    let shadow = unsafe { SHADOW.read_volatile() };
    if valid(&shadow) {
        return shadow;
    }
    Totals::default()
}

/// Write the reading down.
///
/// The shadow first, then the primary. That order is what makes the pair useful: [`load`] prefers
/// the primary, so the copy written last is the one that is trusted, and a power cut during the
/// primary's write leaves the shadow holding the previous good value.
pub fn store(totals: &mut Totals) {
    totals.crc = totals.checksum();

    critical_section::with(|_| {
        // SAFETY: the same region as `load`. FRAM is written like RAM on this part — byte
        // addressable, in place, no erase and no unlock, since the MPU is left disabled. The
        // critical section is not for the hardware but for the pair: an interrupt between the two
        // writes would leave a window where neither copy is the previous good one.
        unsafe {
            SHADOW.write_volatile(*totals);
            PRIMARY.write_volatile(*totals);
        }
    });
}
