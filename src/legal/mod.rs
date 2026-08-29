//! The legally relevant software.
//!
//! Everything a notified body assesses when this instrument is put through MID conformity
//! assessment lives under here: what turns a captured waveform into litres, what those litres are
//! added to, the numbers that calibrate it, and the checks over both.
//!
//! # Why the boundary exists at all
//!
//! WELMEC 7.2 — the guide MID software is assessed against — asks for legally relevant software to
//! be identifiable and protected. It allows two approaches: separate the legally relevant part from
//! the rest and prove the separation (requirements S1 and S2), or declare the whole software
//! legally relevant and be done with it.
//!
//! **This instrument declares its whole software legally relevant.** That is the conservative
//! choice, and it is the right one here: proving separation costs more assessment effort than this
//! firmware would save, and the non-relevant part — a radio and some housekeeping — is small.
//!
//! The module boundary is kept anyway, for two reasons. It documents which code carries the
//! metrology, which is what an assessor asks first. And if the radio ever grows to the point where
//! declaring it legally relevant means re-approval every time it changes, the separation is already
//! drawn and only has to be argued.
//!
//! What that decision costs is worth stating plainly: **any change anywhere in this firmware
//! changes the legally relevant software**, so it needs a new version in
//! [`identity::SOFTWARE_VERSION`] and, depending on the change, a fresh look from the notified
//! body.
//!
//! # Which requirement is met where
//!
//! | | |
//! | --- | --- |
//! | P2, software identification | [`identity::SOFTWARE_VERSION`] and [`identity::image_crc`] |
//! | P5, protection against accidental change | [`identity::image_crc`], checked at every start-up |
//! | P7, parameter protection | [`params`] — the seal, the write counter, the checksum |
//! | S1, software separation | this module boundary, though separation is not claimed |
//!
//! P1 (documentation), P3 and P4 (influence via the user and communication interfaces), P6
//! (protection against deliberate change) and P8 (authentication of presented data) are not
//! addressed by this code alone: they depend on the instrument's physical sealing, its enclosure
//! and the protocol it eventually speaks, none of which exist yet.

pub mod flow;
pub mod fram;
pub mod identity;
pub mod params;
pub mod totals;
