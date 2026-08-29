//! An SSD1306 OLED, shown for a few seconds when somebody presses the button.
//!
//! # Why the power is switched and not the display
//!
//! The SSD1306 has a sleep command, and it is not enough. A measured 0.96-inch module in sleep
//! draws about 26 µA — more than this entire meter does while running, and more than the cell's own
//! self-discharge. A display that sleeps would be the largest consumer in the instrument while
//! showing nothing at all.
//!
//! So the module's supply is switched. [`Display::show`] turns it on, draws, waits, and turns it
//! off again; between presses the display is not powered and costs nothing.
//!
//! Showing it for [`SHOW_FOR`] four times a day works out at about **1.3 µA**, which is still under
//! what the meter itself draws. Leaving it on continuously would be 630 µA — three hundred times
//! the meter, and the end of a 19 Ah cell in about three years.
//!
//! # What the hardware has to be
//!
//! * **An IRLML6401 as a high-side switch on the module's VCC.** A P-channel MOSFET: source to the
//!   3 V rail, drain to the display, gate to [`Display`]'s power pin with a 1 MΩ pull-up to the
//!   rail beside it. Not the GPIO alone: the module's peak while its charge pump starts is beyond
//!   what an MSP430 pin should source.
//!
//!   **The gate pull-up is not optional.** Between reset and this module's first write the pin is
//!   an input, and a floating gate leaves the switch in a state nobody has decided. The pull-up
//!   makes that state "off".
//! * **The I2C pull-ups on the switched rail**, not on the permanent one. Pull-ups above an
//!   unpowered chip push current through its protection diodes into its own VCC — a leak that is
//!   invisible on a bench supply and fatal to a ten-year battery.
//!
//! The firmware does its half of the same problem: the I2C driver is constructed for the duration
//! of a showing and dropped afterwards, which returns SDA and SCL to GPIO inputs rather than
//! leaving the eUSCI driving into an unpowered display.
//!
//! # The one parameter to measure
//!
//! Not the on-resistance. The IRLML6401 is 0.085 Ω at the gate drive it gets here, which across
//! 630 µA is a drop of fifty nanovolts — it could be a hundred times worse and nothing would
//! notice.
//!
//! What matters is **leakage while off**, and the datasheet does not answer it at 3 V. `IDSS` is
//! given as −1.0 µA at −12 V and 25 °C, and −25 µA at −9.6 V and 55 °C. Both are at nearly the
//! part's full rated voltage, and leakage falls steeply below that, so the figure at 3 V will be
//! far smaller — but "far smaller" than a microamp is not a number, and this meter's whole idle
//! draw is 2.1 µA.
//!
//! So it wants measuring on the actual board, at the actual rail voltage, warm. It is the one thing
//! about this switch that could quietly halve the battery life, and the only symptom would be cells
//! coming back early from the field years later.
//!
//! # Two things this is not right for
//!
//! An OLED holds a static image poorly — a meter reading that never changes will burn in — and its
//! luminance halves after some tens of thousands of hours, against the fifteen years this
//! instrument is meant to last. Both are arguments for the display being momentary, which it is,
//! and against ever leaving it on.

use embassy_msp430::gpio::{Level, Output, Pull};
use embassy_msp430::i2c::{Config as I2cConfig, I2c};
use embassy_msp430::peripherals::{EUSCI_B0, P1_6, P1_7, P2_0};
use embassy_msp430::Peri;
use embassy_time::{Duration, Timer};

use crate::legal::totals::Totals;

/// The module's I2C address. `0x3D` on boards that strap SA0 high.
const ADDRESS: u8 = 0x3c;

/// Control byte marking the rest of the transfer as commands.
const COMMANDS: u8 = 0x00;
/// Control byte marking the rest of the transfer as display data.
const DATA: u8 = 0x40;

/// How long a press keeps the display up.
///
/// Forty-five seconds of a 630 µA display, four times a day, averages about 1.3 µA — still under
/// what the meter itself draws. It is the duty cycle that makes this affordable, not the duration:
/// leaving the display on permanently would be three hundred times the whole instrument.
const SHOW_FOR: Duration = Duration::from_secs(crate::config::DISPLAY_SHOW_SECONDS as u64);

/// How long the panel needs after its supply comes up, before it will accept commands.
const POWER_SETTLE_MS: u64 = 100;

/// Initialisation, straight from the datasheet's sequence for a 128×64 panel.
///
/// The one line worth knowing is `0x8D, 0x14`: it enables the internal charge pump, which is what
/// makes the panel work from a single 3.3 V rail instead of needing its own 7–15 V supply.
const INIT: [u8; 24] = [
    0xae, // display off while it is configured
    0xd5, 0x80, // clock divide and oscillator frequency
    0xa8, 0x3f, // multiplex ratio: 64 rows
    0xd3, 0x00, // no display offset
    0x40, // start line 0
    0x8d, 0x14, // charge pump on
    0x20, 0x00, // horizontal addressing
    0xa1, // columns left to right
    0xc8, // rows bottom to top -- with the above, the usual module orientation
    0xda, 0x12, // COM pin configuration for 128x64
    0x81, 0x7f, // contrast, mid scale
    0xd9, 0xf1, // pre-charge
    0xdb, 0x40, // VCOMH deselect
    0xa4, // follow the RAM, not all-on
    0xa6, // not inverted
];

/// A 5×8 font, in the order of [`glyph`].
///
/// Only the characters this instrument shows. A full ASCII font would be a kilobyte of FRAM to
/// display nothing that is ever needed.
const FONT: [[u8; 5]; 18] = [
    [0x3e, 0x51, 0x49, 0x45, 0x3e], // 0
    [0x00, 0x42, 0x7f, 0x40, 0x00], // 1
    [0x42, 0x61, 0x51, 0x49, 0x46], // 2
    [0x21, 0x41, 0x45, 0x4b, 0x31], // 3
    [0x18, 0x14, 0x12, 0x7f, 0x10], // 4
    [0x27, 0x45, 0x45, 0x45, 0x39], // 5
    [0x3c, 0x4a, 0x49, 0x49, 0x30], // 6
    [0x01, 0x71, 0x09, 0x05, 0x03], // 7
    [0x36, 0x49, 0x49, 0x49, 0x36], // 8
    [0x06, 0x49, 0x49, 0x29, 0x1e], // 9
    [0x00, 0x00, 0x00, 0x00, 0x00], // space
    [0x00, 0x60, 0x60, 0x00, 0x00], // .
    [0x7f, 0x40, 0x40, 0x40, 0x40], // L
    [0x7f, 0x49, 0x49, 0x49, 0x41], // E
    [0x7c, 0x08, 0x04, 0x04, 0x08], // r
    [0x38, 0x44, 0x44, 0x44, 0x38], // o
    [0x7c, 0x04, 0x18, 0x04, 0x78], // m
    [0x00, 0x36, 0x36, 0x00, 0x00], // :
];

/// Index into [`FONT`], or a space for anything not in it.
fn glyph(c: u8) -> usize {
    match c {
        b'0'..=b'9' => (c - b'0') as usize,
        b'.' => 11,
        b'L' => 12,
        b'E' => 13,
        b'r' => 14,
        b'o' => 15,
        b'm' => 16,
        b':' => 17,
        _ => 10,
    }
}

/// The display and the pins it needs.
///
/// Holds the peripherals rather than a live driver, because the driver only exists while the
/// display is powered.
/// The high-side switch.
///
/// A type of its own for one reason: the switch is a P-channel MOSFET, so the pin is **low to turn
/// the display on**. That inversion is exactly the kind of thing that gets written the wrong way
/// round once and then read as correct forever, so it is stated here and nowhere else, and the rest
/// of this module says `on()` and `off()`.
struct Power(Output<'static>);

impl Power {
    /// Pulling the gate down against the source puts the MOSFET into conduction.
    fn on(&mut self) {
        self.0.set_low();
    }

    /// Gate at the rail: no gate-source voltage, no conduction.
    fn off(&mut self) {
        self.0.set_high();
    }
}

pub struct Display {
    power: Power,
    i2c: Peri<'static, EUSCI_B0>,
    scl: Peri<'static, P1_7>,
    sda: Peri<'static, P1_6>,
    button: Peri<'static, P2_0>,
}

impl Display {
    /// Take the pins, leaving the display unpowered.
    pub fn new(
        power: Peri<'static, embassy_msp430::peripherals::P3_4>,
        i2c: Peri<'static, EUSCI_B0>,
        scl: Peri<'static, P1_7>,
        sda: Peri<'static, P1_6>,
        button: Peri<'static, P2_0>,
    ) -> Self {
        Self {
            // High is off for a P-channel high-side switch, and off is where the display spends
            // very nearly all of its life.
            power: Power(Output::new(power, Level::High)),
            i2c,
            scl,
            sda,
            button,
        }
    }

    /// Wait for the button, then show `totals` for [`SHOW_FOR`].
    ///
    /// The waiting is what the meter does almost always, and it costs nothing: the pin's edge
    /// interrupt wakes the executor out of LPM3.
    pub async fn serve(&mut self, totals: &Totals, calibrated: bool) {
        {
            // Pulled up, so the button shorts it to ground and the press is a falling edge. An
            // external pull-up would be a permanent leak; the internal one is switched off with
            // the pin between presses.
            let mut button = embassy_msp430::gpio::Input::new(self.button.reborrow(), Pull::Up);
            button.wait_for_falling_edge().await;
        }

        self.show(totals, calibrated).await;
    }

    /// Power the display, draw, wait, and power it down again.
    async fn show(&mut self, totals: &Totals, calibrated: bool) {
        self.power.on();
        Timer::after_millis(POWER_SETTLE_MS).await;

        // The driver lives only as long as this block. Dropping it puts SDA and SCL back to GPIO
        // before the supply is cut, so nothing is left driving an unpowered chip.
        {
            let mut i2c = match I2c::new(
                self.i2c.reborrow(),
                self.scl.reborrow(),
                self.sda.reborrow(),
                I2cConfig::default(),
            ) {
                Ok(i2c) => i2c,
                // A display that will not configure is not worth reporting anywhere -- there is
                // nowhere to report it to. Cut the power and carry on metering.
                Err(_) => {
                    self.power.off();
                    return;
                }
            };

            if Self::draw(&mut i2c, totals, calibrated).await.is_ok() {
                Timer::after(SHOW_FOR).await;
            }
        }

        self.power.off();
    }

    /// Send the initialisation sequence and the two lines of text.
    ///
    /// Free of `self` on purpose: the bus driver handed in already borrows the peripheral this
    /// display owns, so anything reaching for `self` as well would be a second borrow of it.
    async fn draw(i2c: &mut I2c<'_>, totals: &Totals, calibrated: bool) -> Result<(), ()> {
        let mut init = [0u8; INIT.len() + 1];
        init[0] = COMMANDS;
        init[1..].copy_from_slice(&INIT);
        i2c.write(ADDRESS, &init).await.map_err(|_| ())?;

        Self::clear(i2c).await?;

        // The reading, as the dial would show it.
        let mut line = [b' '; 16];
        let n = format_reading(totals, &mut line);
        Self::text(i2c, 2, &line[..n]).await?;

        // An uncalibrated instrument says so, in the place a reading would otherwise be trusted
        // from. It is the same statement `Outcome::NotCalibrated` makes to the radio.
        if !calibrated {
            Self::text(i2c, 5, b"Error 0").await?;
        }

        // Only now is the panel turned on, so the first thing anybody sees is the finished screen
        // rather than the previous contents of its RAM.
        i2c.write(ADDRESS, &[COMMANDS, 0xaf]).await.map_err(|_| ())
    }

    /// Blank every page.
    async fn clear(i2c: &mut I2c<'_>) -> Result<(), ()> {
        let blank = [0u8; 33];
        for page in 0..8 {
            Self::seek(i2c, page, 0).await?;
            // 128 columns in chunks of 32, which keeps the transfer buffer small.
            for _ in 0..4 {
                let mut chunk = blank;
                chunk[0] = DATA;
                i2c.write(ADDRESS, &chunk).await.map_err(|_| ())?;
            }
        }
        Ok(())
    }

    /// Point the controller at a page and column.
    async fn seek(i2c: &mut I2c<'_>, page: u8, column: u8) -> Result<(), ()> {
        i2c.write(
            ADDRESS,
            &[
                COMMANDS,
                0xb0 | (page & 0x07),
                column & 0x0f,
                0x10 | (column >> 4),
            ],
        )
        .await
        .map_err(|_| ())
    }

    /// Draw `text` at the start of `page`.
    async fn text(i2c: &mut I2c<'_>, page: u8, text: &[u8]) -> Result<(), ()> {
        Self::seek(i2c, page, 0).await?;
        // One character per transfer: six bytes of glyph and spacing, plus the control byte. Slower
        // than batching, and the display is up for ten seconds.
        for &c in text {
            let g = FONT[glyph(c)];
            let out = [DATA, g[0], g[1], g[2], g[3], g[4], 0x00];
            i2c.write(ADDRESS, &out).await.map_err(|_| ())?;
        }
        Ok(())
    }
}

/// Write the reading into `out` the way a water meter's dial shows it, and return its length.
///
/// Cubic metres with three decimals, which is what a domestic meter is read in and what the bill is
/// written in. The instrument counts litres, so the whole part is thousands of them and the
/// decimals are the litres under that — a reading of 1234 litres shows as `1.234 m3`.
fn format_reading(totals: &Totals, out: &mut [u8; 16]) -> usize {
    let mut i = 0;
    let push = |v: u8, out: &mut [u8; 16], i: &mut usize| {
        if *i < out.len() {
            out[*i] = v;
            *i += 1;
        }
    };

    let cubic_metres = totals.litres / 1000;
    let litres = totals.litres % 1000;

    // Whole cubic metres, most significant digit first, without leading zeroes.
    let mut divisor = 10_000u32;
    let mut started = false;
    while divisor > 0 {
        let digit = (cubic_metres / divisor) % 10;
        if digit != 0 || started || divisor == 1 {
            push(b'0' + digit as u8, out, &mut i);
            started = true;
        }
        divisor /= 10;
    }

    push(b'.', out, &mut i);
    push(b'0' + ((litres / 100) % 10) as u8, out, &mut i);
    push(b'0' + ((litres / 10) % 10) as u8, out, &mut i);
    push(b'0' + (litres % 10) as u8, out, &mut i);
    push(b' ', out, &mut i);
    push(b'm', out, &mut i);
    push(b'3', out, &mut i);
    i
}
