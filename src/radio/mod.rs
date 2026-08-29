//! Broadcasting the reading over wireless M-Bus.
//!
//! The instrument transmits and never listens. That is what makes a radio affordable here at all:
//! a frame costs about 30 mA for four milliseconds, which spread over a minute is 2 µA, while
//! receiving costs 15.6 mA continuously — seventy times the whole meter if it listened even one per
//! cent of the time.
//!
//! What that rules out is a downlink: no remote configuration, no acknowledgement, no over-the-air
//! update. For a water meter in a stairwell that is the right trade, and wM-Bus has a mode for it.

pub mod cc1101;
pub mod wmbus;

use embassy_msp430::gpio::{Level, Output};
use embassy_msp430::spi::{Config as SpiConfig, Spi};
use embassy_msp430::peripherals::{EUSCI_A1, P1_0, P1_2, P1_3, P3_5};
use embassy_msp430::Peri;

use crate::legal::totals::Totals;
use cc1101::Cc1101;

/// The radio and the pins it needs.
pub struct Radio {
    spi: Peri<'static, EUSCI_A1>,
    sck: Peri<'static, P1_0>,
    mosi: Peri<'static, P1_2>,
    miso: Peri<'static, P1_3>,
    cs: Peri<'static, P3_5>,
    /// Counts up on every frame, so a receiver can tell a repeat from a fresh reading and see a gap.
    access: u8,
}

impl Radio {
    /// Take the pins. Nothing is powered until something is transmitted.
    pub fn new(
        spi: Peri<'static, EUSCI_A1>,
        sck: Peri<'static, P1_0>,
        mosi: Peri<'static, P1_2>,
        miso: Peri<'static, P1_3>,
        cs: Peri<'static, P3_5>,
    ) -> Self {
        Self { spi, sck, mosi, miso, cs, access: 0 }
    }

    /// Broadcast the reading.
    ///
    /// The SPI driver is built and dropped around the transmission, as the display's I2C is: the
    /// eUSCI has no business holding three pins in peripheral mode for the fifty-nine seconds
    /// between frames.
    pub fn broadcast(&mut self, totals: &Totals, serial: u32, status: u8) {
        let Ok(spi) = Spi::new(
            self.spi.reborrow(),
            self.sck.reborrow(),
            self.mosi.reborrow(),
            self.miso.reborrow(),
            SpiConfig::default(),
        ) else {
            return;
        };

        // Chip select idles high.
        let cs = Output::new(self.cs.reborrow(), Level::High);
        let mut radio = Cc1101::new(spi, cs);

        let reading = wmbus::Reading {
            serial,
            litres: totals.litres,
            access: self.access,
            status,
        };
        self.access = self.access.wrapping_add(1);

        let mut frame = [0u8; wmbus::MAX_FRAME];
        let len = wmbus::frame(&reading, &mut frame);
        radio.transmit(&frame[..len]);
    }
}
