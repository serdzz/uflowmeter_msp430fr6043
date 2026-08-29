//! Where the instrument's own data sits in FRAM.
//!
//! One place, because two modules each choosing an address is how blocks come to overlap: the slots
//! interleave — totals, parameters, totals shadow, parameters shadow — so a size assertion made in
//! one module against its own slot says nothing about whether it has run into the next one. Stated
//! together, the arithmetic is checkable and is checked below.
//!
//! The interleaving is deliberate. Each block is written as a pair, shadow first, so that a battery
//! pulled mid-write spoils at most the copy that is not yet trusted; putting the two copies of a
//! block at opposite ends of the region means a write that runs away from its slot damages the
//! other block rather than its own backup.

/// The region `memory.x` reserves. Nothing else may be placed here.
pub const BASE: u16 = 0x6000;
/// Its length.
pub const LEN: u16 = 0x0100;
/// Every block gets the same slot, which is what makes the arithmetic below simple enough to trust.
pub const SLOT: u16 = 0x40;

/// The meter reading.
pub const TOTALS: u16 = BASE;
/// The per-instrument calibration.
pub const PARAMS: u16 = BASE + SLOT;
/// Second copy of the reading.
pub const TOTALS_SHADOW: u16 = BASE + 2 * SLOT;
/// Second copy of the calibration.
pub const PARAMS_SHADOW: u16 = BASE + 3 * SLOT;

/// The four slots have to fit the region.
const _: () = assert!(4 * SLOT <= LEN);
