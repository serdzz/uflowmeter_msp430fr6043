# Schematic

Every net, every pin. Pin **numbers** are in [`PINOUT.md`](PINOUT.md), read off the datasheet's
section 9.14 and quoted here for the RGC (VQFN-64) package the board uses.

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


## U1 — MSP430FR5043IRGCR, VQFN-64-EP 9×9

Not the FR6043. The FR6043 is not in JLCPCB's library at all; the FR5043 is the same die without the
LCD controller, and since the display is an OLED on I²C that controller was never going to be used.

### Supply and core

| Pin | Net | Part |
| --- | --- | --- |
| DVCC | `VCC` | C3 100 n `0402` |
| AVCC | `VCC` | C4 100 n `0402` |
| DVSS, AVSS | `GND` | |
| VCORE | EP (pad) | `GND` | at least 9 vias |

### Clocks

Three crystals. Two of them decide whether the instrument works at all.

| | Crystal | Load caps | Net |
| --- | --- | --- | --- |
| LFXT 32.768 kHz | Y1 | C6, C7 | `LFXIN`, `LFXOUT` |
| **USSXT 8 MHz** | Y2 | C8, C9 | `USSXTIN` (62), `USSXTOUT` (63) |

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

## The radio — a module on J6

Not a chip on this board. See [`RF.md`](RF.md) for what that removed and what the module has to be.

| J6 | Net | MCU |
| ---: | --- | --- |
| 1 | `GND` | |
| 2 | `VCC` | |
| 3 | GDO0 | not connected |
| 4 | `RADIO_CS` | P3.5 — pin 52 |
| 5 | `SPI_SCK` | P1.0 — pin 3 |
| 6 | `SPI_MOSI` | P1.2 — pin 19 |
| 7 | GDO2 | not connected |
| 8 | `SPI_MISO` | P1.3 — pin 20 |

* **C2** — 10 µF `0603` at J6. The radio's 35 mA pulse comes out of this, not down the cell leads.

The module runs from `VCC` directly, unswitched: the CC1101 has its own `SPWD` state at 200 nA,
which is below this meter's own sleep current, so there is nothing for a switch to save.

## Display

The SSD1306 module is bought as a module and plugged in, not assembled here.

| Net | From | To |
| --- | --- | --- |
| `DISP_GATE` | U1 P3.4 (pin 51) | Q1 gate, R2 |
| `VCC_DISP` | Q1 drain | J3 pin 2, R3, R4, C18 |
| `I2C_SCL` | U1 P1.7 (pin 24) | J3 pin 3, R3 |
| `I2C_SDA` | U1 P1.6 (pin 23) | J3 pin 4, R4 |

* **Q1** — IRLML6401, P-channel, `SOT-23`. Source to `VCC`, drain to `VCC_DISP`. **The gate is low
  to turn the display on.**
* **R2** — 1 MΩ, gate to `VCC`. **Not optional.** Between reset and the firmware's first write P3.4
  is an input, and a floating gate leaves the switch in a state nobody has decided.
* **R3, R4** — 4.7 k I²C pull-ups **to `VCC_DISP`, not to `VCC`**. Pull-ups above an unpowered chip
  push current through its protection diodes — invisible on a bench supply and fatal to a ten-year
  battery.
* **C18** — 1 µF `0603` on `VCC_DISP`.
* **J3** — 4-pin 2.54 mm, on the **front**, pins in a row at y = 6 mm: 1 `GND`, 2 `VCC_DISP`,
  3 `I2C_SCL`, 4 `I2C_SDA`.

  The module stands on this header and its 27 x 27 mm body hangs below it, over **x 38.3…65.3,
  y 6…33** — directly above the button, which sits at x 47.2…56.4, y 35.9…46.1. Three millimetres
  between them. That is the front panel: reading on top, button under it, both reachable through
  the same face of an enclosure.

  The board was rearranged around this rather than the header being dropped somewhere free. The
  transducer header moved to the top left, the cell and the calibration header to the right edge
  below the panel, and the display's own passives out from under the button — because a 27 mm
  module above a button needs a clear column, and nothing that protrudes may sit under it.

  **What does sit under it** is `U1` and its decoupling, all surface mount, which the module clears
  on its header standoff. Fine to build, awkward to rework: probing the MCU means lifting the
  display.

  **Check the mating before ordering.** The order `GND VCC SCL SDA` is the module's own, read off
  the silkscreen of the part in hand. It was `VCC GND SCL SDA` in the first draft, which would have
  put the supply across the module backwards; both orders exist on these breakouts, so this is
  worth checking against the part and not against habit.

## Button

| Net | Part |
| --- | --- |
| `BUTTON` | SW1 to `GND`, C19 100 n to `GND`, U1 P2.0 (pin 21) |

No pull-up resistor: the firmware enables the internal one only while waiting, so that it is not a
permanent leak. With that ~35 k pull-up, C19 gives about 3.5 ms of debounce.

## Ultrasonic front end

Its own power domain. Do not merge it into `AVCC`.

| Net | Pin (RGC64) | To |
| --- | ---: | --- |
| `CH0_OUT` | 59 | J4 pin 1 — transducer A drive |
| `CH0_IN` | 60 | J4 pin 2 — transducer A echo |
| `CH1_OUT` | 54 | J4 pin 3 — transducer B drive |
| `CH1_IN` | 53 | J4 pin 4 — transducer B echo |
| `PVCC` | 56, 57 | `VCC`, with C20 and C21 100 n `0402` one at each pin |
| `PVSS` | 55, 58 | star ground, on its own return |
| `USSXTIN` | 62 | Y2 |
| `USSXTOUT` | 63 | Y2 |

### How little there is to it

The front end is integrated to an unusual degree, which is the reason to choose this part at all.
`SAPH_A` drives the transducer, and it also owns the bias switches and the input multiplexer — so
the excitation bias and the amplifier bias are generated on-chip and switched by the sequencer,
without the CPU and without external components. The firmware sets them as numbers
(`excitation_bias`, `pga_bias` in `config.rs`), not as resistors.

So each channel is one node: `CHx_OUT` drives the transducer and `CHx_IN` listens to the same
terminal, with the transducer's other terminal at `PVSS`. Ten excitation pulses go out, two stop
pulses damp the ringing, and the echo comes back on the same wire.

* **J4** — 4-pin 2.54 mm: 1 `CH0_OUT`+`CH0_IN`, 2 `PVSS`, 3 `CH1_OUT`+`CH1_IN`, 4 `PVSS`.

**What is not settled**, and cannot be until a transducer is chosen: whether `CHx_OUT` and `CHx_IN`
tie directly at the pins or want a series element between drive and sense; whether the drive needs
current limiting; and what protection the cable into a wet meter body needs. Those depend on the
transducer's impedance and on the mechanics, and this is the one part of the board that should be
laid out with test points and the freedom to fit or omit series parts.

**Y2 must be a crystal, not an oscillator module.** The datasheet is explicit: *"Do not connect the
USSXTIN and USSXTOUT pins to AVCC or DVCC. USSXTIN does not support bypass mode, so do not drive an
external clock to USSXTIN pin."* Those two pins are a 1.5 V analog domain of their own — treat them
as analog, not as a logic clock input.

`PVCC` is the supply that swings while a transducer is driven, so its decoupling goes at the pins
and its return reaches the star ground separately from the digital one.

## Calibration

* **J5** — 3-pin 2.54 mm: 1 `GND`, 2 `UART_TX` (P4.3, pin 31), 3 `UART_RX` (P4.4, pin 32).

Only alive while the instrument is unsealed. Once sealed, the firmware never opens the UART again.

---

## What has not been verified

Read this before ordering anything.

1. **P1.7 is also `USSTRG`.** It is used here as I²C SCL. If the USS front end ever needs its
   external trigger, these two want separating.
2. **The RF section has no component values here** — by design, they come from TI's reference.
3. **The firmware still builds for the 80-pin FR6043**, while this board is the 64-pin FR5043. Every
   pin it uses exists on both and carries the same function, so the change is a feature flag rather
   than a rework — but it has not been made or built yet.
4. **The transducer interface is specified but not designed.** Which pins go where is now known;
   what sits between them and the transducers — matching, bias, protection — is not, and depends on
   transducers nobody has chosen.

Previously listed here and now done: the pin numbers (see [`PINOUT.md`](PINOUT.md)), the
alternate-function check against the datasheet (which found a real bug — P1.7's `UCB0SCL` is the
second alternate, not the third, and `embassy-msp430` had it wrong), and the USS front-end pins.
