# Schematic

Every net, every pin. The pin assignments are the ones the firmware claims — they are checked
against `embassy-msp430`'s pin traits, **not** against the datasheet's Table 7-1. See the caveat at
the end.

## Power

The cell drives everything directly. There is no regulator, and that is the point: an LDO's own
quiescent current would be a large fraction of the 5.4 µA this instrument draws, and both ICs run
from 1.8 V to 3.6 V natively.

| Net | From | To |
| --- | --- | --- |
| `BAT+` | BT1 pin 1 | D1 anode |
| `VCC` | D1 cathode | U1 DVCC/AVCC, U2 supply, Q1 source, R2 |
| `GND` | BT1 pin 2 | everything |

* **BT1** — ER34615**H**, 3.6 V Li-SOCl₂ D cell, **with hybrid layer capacitor**. See `README.md`;
  the plain ER34615 is a low-drain cell and will not carry the radio's pulses.
* **D1** — optional series Schottky, `SOD-123`. Fitted, it drops the fresh cell's 3.67 V open-circuit
  below the MCU's 3.6 V recommended maximum. Not fitted, link it out with a 0 Ω. The trade is
  headroom at end of life against operating slightly above TI's recommended conditions at the start.
* **C1** — 100 µF, 6.3 V, `0805`, across `VCC` at the cell terminals.
* **C2** — 10 µF `0603`, at U2. The radio's 35 mA pulse comes out of this, not out of the cell.

## U1 — MSP430FR5043IRGCR, VQFN-64-EP 9×9

Not the FR6043. The FR6043 is not in JLCPCB's library at all; the FR5043 is the same die without the
LCD controller, and since the display is an OLED on I²C that controller was never going to be used.

### Supply and core

| Pin | Net | Part |
| --- | --- | --- |
| DVCC | `VCC` | C3 100 n `0402` |
| AVCC | `VCC` | C4 100 n `0402` |
| DVSS, AVSS | `GND` | |
| VCORE | `VCORE` | C5 470 n `0402` to `GND` — required, not optional |
| EP (pad) | `GND` | at least 9 vias |

### Clocks

Three crystals. Two of them decide whether the instrument works at all.

| | Crystal | Load caps | Net |
| --- | --- | --- | --- |
| LFXT 32.768 kHz | Y1 | C6, C7 | `LFXIN`, `LFXOUT` |
| **USSXT 8 MHz** | Y2 | C8, C9 | `USSXTIN`, `USSXTOUT` |

**Y2 is the measurement.** Every sample interval and every excitation period is derived from the
80 MHz the HSPLL makes from it, so an error here is an error in every reading at once. Its
*absolute* accuracy matters less than it looks: the flight-time difference is measured in both
directions on the same clock, so a frequency error largely cancels. Its *stability over
temperature* is what does not cancel.

Load capacitor values follow each crystal's C\_L, not a habit — see `LAYOUT.md`.

### Reset and debug

| Net | Part |
| --- | --- |
| `RST/SBWTDIO` | R1 47 k to `VCC`, C10 100 n to `GND`, J1 pin 3 |
| `TEST/SBWTCK` | J1 pin 4 |

* **J1** — 4-pin 1.27 mm Spy-Bi-Wire header: 1 `VCC`, 2 `GND`, 3 `RST/SBWTDIO`, 4 `TEST/SBWTCK`.
  This is how firmware gets in. A board without it is a board you throw away.

## U2 — CC1101RGPR, QFN-20-EP 4×4, 868.95 MHz

| CC1101 | Net | MCU pin |
| --- | --- | --- |
| SI | `SPI_MOSI` | P1.2 (UCA1SIMO) |
| SO (GDO1) | `SPI_MISO` | P1.3 (UCA1SOMI) |
| SCLK | `SPI_SCK` | P1.0 (UCA1CLK) |
| CSn | `RADIO_CS` | P3.5 |
| XOSC_Q1/Q2 | Y3 26 MHz + C11, C12 | |
| AVDD ×3, DVDD | `VCC` | C13–C16 100 n `0402` each |
| DCOUPL | `RADIO_DCOUPL` | C17 100 n to `GND` |
| RF_P, RF_N | filter balun → J2 | |
| GDO0, GDO2 | not connected | |

`GDO0` and `GDO2` are left unconnected on purpose: the firmware polls `MARCSTATE` over SPI rather
than watching a pin, because a frame is four milliseconds and the CPU has nothing else to do in
them.

### The filter balun — copy TI's reference design, do not invent it

`RF_P`/`RF_N` are differential; the antenna is single-ended 50 Ω. The network between them is a
filter balun, and the impedance the chip wants to see toward the antenna is **Z = 86.5 + j43 Ω at
868 MHz**.

Designators, per TI Design Note **DN017 (SWRA168A)**: C121, C122, C123, C124, C125, L121, L122,
L123, L124, L131, L132, C131.

**The values are not reproduced here.** They live in TI's CC1101EM 868/915 MHz reference design
(SWRR045) and must be copied from it exactly, along with its layout. Two reasons this is not
pedantry:

1. **There are three versions of that reference design.** DN017 labels them "newest, recommended",
   "second version, not recommended", and "first version, should not be used". Copying the wrong one
   off a forum post is a real and common failure.
2. RF matching depends on board parasitics, so the values are only correct together with the
   layout they were characterised on.

### Two things that decide whether it can be sold in Europe

Both come out of DN017 and neither is obvious from the CC1101 datasheet.

**The inductors must be wirewound, not multilayer.** DN017's Table 5 measures the same circuit with
both:

| Inductors | PA setting | TX current | Output | 2nd harmonic | ETSI EN 300 220 |
| --- | --- | ---: | ---: | ---: | --- |
| All multilayer | 0xC0 | 33.6 mA | 9.8 dBm | −30.8 dBm | **no margin** |
| All multilayer | 0xC2 | 30.4 mA | 9.3 dBm | −37.3 dBm | passes |
| **All wirewound** | **0xC0** | **35.0 mA** | **12.0 dBm** | −34.8 dBm | **passes** |

Cheap multilayer inductors are the default choice and they are the wrong one here.

**The 699 MHz notch filter is required.** A CC1101 emits a spur above −54 dBm at 699 MHz, and
EN 300 220 requires below −54 dBm. Because this board uses an antenna *connector*, compliance is
proven by **conducted** measurement, which sees that spur. Three extra parts, values from DN017
Table 1:

| Part | Value |
| --- | --- |
| C125 | 12 pF |
| C126 | 47 pF |
| L125 | 3.3 nH |

(A board with an integrated antenna is assessed by radiated measurement instead and can leave the
notch out. That is a different board.)

* **J2** — u.FL / IPEX connector. An external, already-certified whip antenna is the cheapest way
  through RED for a first product.

## Display

The SSD1306 module is bought as a module and plugged in, not assembled here.

| Net | From | To |
| --- | --- | --- |
| `DISP_GATE` | U1 P3.4 | Q1 gate, R2 |
| `VCC_DISP` | Q1 drain | J3 pin 1, R3, R4, C18 |
| `I2C_SCL` | U1 P1.7 | J3 pin 3, R3 |
| `I2C_SDA` | U1 P1.6 | J3 pin 4, R4 |

* **Q1** — IRLML6401, P-channel, `SOT-23`. Source to `VCC`, drain to `VCC_DISP`. **The gate is low
  to turn the display on.**
* **R2** — 1 MΩ, gate to `VCC`. **Not optional.** Between reset and the firmware's first write P3.4
  is an input, and a floating gate leaves the switch in a state nobody has decided.
* **R3, R4** — 4.7 k I²C pull-ups **to `VCC_DISP`, not to `VCC`**. Pull-ups above an unpowered chip
  push current through its protection diodes — invisible on a bench supply and fatal to a ten-year
  battery.
* **C18** — 1 µF `0603` on `VCC_DISP`.
* **J3** — 4-pin 2.54 mm: 1 `VCC_DISP`, 2 `GND`, 3 `I2C_SCL`, 4 `I2C_SDA`.

## Button

| Net | Part |
| --- | --- |
| `BUTTON` | SW1 to `GND`, C19 100 n to `GND`, U1 P2.0 |

No pull-up resistor: the firmware enables the internal one only while waiting, so that it is not a
permanent leak. With that ~35 k pull-up, C19 gives about 3.5 ms of debounce.

## Transducers

* **J4** — 4-pin 2.54 mm to the two 1 MHz transducers.

**This is the part of the schematic that is least finished.** The USS front-end pins (`CH0`/`CH1`
drive and the ADC return) are dedicated analog pins whose numbers on the VQFN-64 package have not
been read off the datasheet — see below.

## Calibration

* **J5** — 3-pin 2.54 mm: 1 `GND`, 2 `UART_TX` (U1 P4.3), 3 `UART_RX` (U1 P4.4).

Only alive while the instrument is unsealed. Once sealed, the firmware never opens the UART again.

---

## What has not been verified

Read this before ordering anything.

1. **No pin *numbers* are in this document, only pin *names*.** Turning names into the 64 pads of a
   VQFN-64 needs the FR5043 datasheet's package pinout, which has not been opened here.
2. **The alternate-function assignments are unconfirmed.** `embassy-msp430` carries this note in its
   own source: *"the alternates are derived from the order of the functions in the datasheet's
   package pinout and want checking against Table 7-1."* If an alternate is wrong, that peripheral
   silently does not appear on that pin.
3. **P1.7 is also `USSTRG`.** It is used here as I²C SCL. If the USS front end ever needs its
   external trigger, these two want separating.
4. **The USS front-end connections are not specified**, per above.
5. **The RF section has no component values here** — by design, they come from TI's reference.
