//! The CC1101, transmitting only.
//!
//! # It never receives
//!
//! Not a limitation — the design. Receiving costs 15.6 mA, and an instrument that listened even one
//! per cent of the time would draw 156 µA, seventy times everything else in this meter put
//! together. Transmitting a wM-Bus frame costs about 30 mA for the four milliseconds it is on air,
//! which spread over a minute is 2 µA. That asymmetry is why wM-Bus has a mode for meters that only
//! ever talk, and why this driver has no `receive`.
//!
//! Between frames the radio is in `SPWD`, its power-down state, which the datasheet gives as 200 nA
//! — below the meter's own sleep current.
//!
//! # The register settings
//!
//! From TI's own values for 868.95 MHz, 100 kbps 2-FSK — the wM-Bus C1 air interface. They are
//! reproduced here rather than computed because the synthesiser words are the output of TI's design
//! tool and there is no honest way to derive them in a comment.
//!
//! **None of this has been near an antenna.** A wrong word in the table below produces a radio that
//! transmits confidently on the wrong frequency, and nothing in the firmware could tell.

use embassy_msp430::gpio::Output;
use embassy_msp430::spi::Spi;

/// Header bit: read rather than write.
const READ: u8 = 0x80;
/// Header bit: burst, meaning the address auto-increments.
const BURST: u8 = 0x40;

/// Command strobes.
const SRES: u8 = 0x30;
const STX: u8 = 0x35;
const SIDLE: u8 = 0x36;
const SPWD: u8 = 0x39;
const SFTX: u8 = 0x3b;

/// The transmit FIFO.
const FIFO: u8 = 0x3f;
/// `PKTLEN`, which in fixed-length mode is how many bytes go out.
const PKTLEN: u8 = 0x06;
/// `MARCSTATE`, the state machine's own view of what it is doing.
const MARCSTATE: u8 = 0x35 | 0xc0;

/// Configuration for 868.95 MHz, 100 kbps 2-FSK, fixed length, CRC off.
///
/// The CRC is off because wM-Bus carries its own — see [`super::wmbus`] — and the CC1101's would be
/// a second, different one appended to a frame that has no room for it.
const CONFIG: [(u8, u8); 21] = [
    (0x02, 0x06), // IOCFG0: assert while the packet is being sent, deassert at the end
    (0x03, 0x07), // FIFOTHR
    (0x08, 0x00), // PKTCTRL0: fixed length, no CRC
    (0x07, 0x00), // PKTCTRL1: no address check, no status bytes
    (0x0b, 0x08), // FSCTRL1
    (0x0d, 0x21), // FREQ2  \
    (0x0e, 0x65), // FREQ1   > 868.95 MHz
    (0x0f, 0x6a), // FREQ0  /
    (0x10, 0x5c), // MDMCFG4 \
    (0x11, 0x04), // MDMCFG3  > 100 kbps
    (0x12, 0x05), // MDMCFG2: 2-FSK, 16/16 sync word
    (0x13, 0x22), // MDMCFG1
    (0x14, 0xf8), // MDMCFG0
    (0x15, 0x44), // DEVIATN: about 50 kHz
    (0x17, 0x30), // MCSM1: idle after transmitting
    (0x18, 0x18), // MCSM0: calibrate when leaving idle
    (0x21, 0x56), // FREND1
    (0x22, 0x10), // FREND0
    (0x23, 0xea), // FSCAL3
    (0x24, 0x2a), // FSCAL2
    (0x25, 0x00), // FSCAL1
];

/// Sync word for wM-Bus mode C, frame format A.
const SYNC: (u8, u8) = (0x54, 0x3d);

/// Output power, as a `PATABLE` entry.
///
/// `0xC0` is 12.0 dBm at 35.0 mA, per TI Design Note DN017 Table 5 -- **with wirewound matching
/// inductors**. The same setting with multilayer inductors gives 9.8 dBm and leaves no margin
/// against EN 300 220, so this constant and the board's bill of materials have to agree. See
/// `hw/SCHEMATIC.md`.
const PA_POWER: u8 = 0xc0;

/// The radio.
pub struct Cc1101<'d> {
    spi: Spi<'d>,
    cs: Output<'d>,
}

impl<'d> Cc1101<'d> {
    /// Take the bus and the chip select, and leave the radio powered down.
    ///
    /// Configuration happens in [`Cc1101::transmit`], not here: the CC1101 loses its registers in
    /// `SPWD`, which is the state it spends its life in, so they are written afresh each time. That
    /// costs twenty-odd SPI bytes per frame and saves the 200 µA that keeping it in idle would.
    pub fn new(spi: Spi<'d>, cs: Output<'d>) -> Self {
        let mut radio = Self { spi, cs };
        radio.strobe(SRES);
        radio.strobe(SPWD);
        radio
    }

    /// Chip select is active low, and the CC1101 wants it held across the whole transfer.
    fn select(&mut self) {
        self.cs.set_low();
    }

    fn deselect(&mut self) {
        self.cs.set_high();
    }

    /// Send a command strobe.
    fn strobe(&mut self, cmd: u8) {
        self.select();
        let _ = self.spi.blocking_write(&[cmd]);
        self.deselect();
    }

    /// Write one register.
    fn write(&mut self, addr: u8, value: u8) {
        self.select();
        let _ = self.spi.blocking_write(&[addr, value]);
        self.deselect();
    }

    /// Read one status register.
    fn read(&mut self, addr: u8) -> u8 {
        let mut buf = [addr | READ, 0];
        self.select();
        let _ = self.spi.blocking_transfer_in_place(&mut buf);
        self.deselect();
        buf[1]
    }

    /// Bring the radio out of power-down and write the whole configuration.
    fn configure(&mut self, length: u8) {
        self.strobe(SIDLE);
        for (addr, value) in CONFIG {
            self.write(addr, value);
        }
        // The sync word the receiver is listening for.
        self.write(0x04, SYNC.0);
        self.write(0x05, SYNC.1);
        self.write(PKTLEN, length);

        // PATABLE is a burst write of one entry.
        self.select();
        let _ = self.spi.blocking_write(&[0x3e | BURST, PA_POWER]);
        self.deselect();
    }

    /// Send `frame`, then put the radio back to sleep.
    ///
    /// Blocking, and deliberately: a frame is four milliseconds, the CPU has nothing else to do in
    /// them, and an async version would cost an interrupt handler and a waker for no saving.
    pub fn transmit(&mut self, frame: &[u8]) {
        if frame.is_empty() || frame.len() > 64 {
            return;
        }

        self.configure(frame.len() as u8);
        self.strobe(SFTX);

        self.select();
        let _ = self.spi.blocking_write(&[FIFO | BURST]);
        let _ = self.spi.blocking_write(frame);
        self.deselect();

        self.strobe(STX);

        // Wait for the state machine to leave transmit. Bounded, because a radio that never
        // finishes must not take the meter down with it -- the reading matters more than the frame.
        for _ in 0..20_000u32 {
            // MARCSTATE 0x13 is TX; anything else means it has finished or never started.
            if self.read(MARCSTATE) & 0x1f != 0x13 {
                break;
            }
        }

        self.strobe(SIDLE);
        self.strobe(SFTX);
        // 200 nA until the next frame, which is less than the meter's own sleep current.
        self.strobe(SPWD);
    }
}
