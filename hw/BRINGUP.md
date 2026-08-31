# Bringing the first board up

Nothing in this design has been near hardware. The order below is not a preference: each step is
what makes the next one interpretable, and skipping one turns a clear failure into a puzzle.

## Before any power

**Look at U1 under magnification.** A 0.5 mm pitch QFN with hand-fitted headers around it — a solder
bridge there is the likeliest fault on the board, and it is far cheaper to find with eyes than with
current.

**Continuity, cell terminal to U1's supply pins**, and **no continuity from either to ground**. The
board is mostly ground plane; a bridge to it is easy to make and silent.

## First power — bench supply, not the cell

**3.3 V, current limited to 10 mA.** If it draws milliamps, stop and look again: this instrument's
whole budget is 5.7 µA, so anything above tens of microamps means a fault, not a surprise.

Expect roughly **1.5 µA** with no firmware running — the MSP430 in reset.

## Then, in this order

**1. Spy-Bi-Wire.** Connect a programmer to J1 and read the device ID. That one step validates the
supply, `RST`, `TEST`, and the core — and `RST` is worth naming, because it reached J1 only by
accident until late in the layout and is now routed explicitly.

### The MSP-FET430UIF works, and is the better tool here

TI's *MSP Debuggers* guide (SLAU647O, Table 1) marks the MSP-FET430UIF as supporting **all
programmable MSP430 and CC430 devices**, over 4-wire JTAG and 2-wire Spy-Bi-Wire, with an
adjustable target supply. The FR5043 is inside that.

Do not confuse it with the **MSP-FET430PIF**, the parallel-port one, which the same table footnotes
as *"legacy device support only — does not support any devices released after 2011."*

**Its linear regulator is a reason to prefer it over the current MSP-FET**, not a sign of its age.
The guide says so directly: the MSP-FET and eZ-FET use a DC-DC converter that puts 5–50 mV of
ripple at 1–50 kHz on the target supply, and for sensitive analog and RF circuits it recommends an
emulator with an integrated linear regulator — naming the MSP-FET430UIF. This board has an
ultrasonic front end measuring picoseconds and a radio on the same rail.

| MSP-FET 14-pin | | J1 |
| --- | --- | --- |
| 11 `SBWTDIO` | → | 3 `RST` |
| 13 `SBWTCK` | → | 4 `TEST` |
| 9 `GND` | → | 2 `GND` |
| 4 `VCC_TARGET` | → | 1 `VCC` |

Use pin 4 (sense) with the board on a bench supply. Pin 2 (`VCC_TOOL`) would power the board from
the probe, which is convenient later and wrong for a first power-up — a fault should show as a
current limit, not as smoke.

**What it does not have**, and what that costs here:

* **No backchannel UART.** The calibration interface on J5 is a plain 3.3 V UART, and a current
  MSP-FET would carry it over the same cable. With the UIF you need a separate USB-serial adapter.
* **No EnergyTrace.** Not much of a loss: its resolution would not settle a 5.7 µA budget anyway.
  The leakage measurement in step 5 wants a source-meter, not a debug probe.
* **No BSL mode.** Irrelevant while J1 is fitted.

Two practical notes from the same guide. **Update its firmware through a direct USB port, not a
hub** — the update can fail through one, and a UIF this old will almost certainly ask for an update
the first time a current CCS sees it. And **never pull the USB or JTAG cable during a live debug
session**; terminate it first, or the target can be left drawing current.

**2. The 32.768 kHz crystal.** Confirm it starts. If it does not, the load capacitors are the first
suspect: `C6`/`C7` are 18 pF, chosen from a nominal 12.5 pF `C_L`, and want confirming against the
crystal actually fitted. See `LAYOUT.md`.

**3. The 8 MHz USSXT crystal.** Same check, `C8`/`C9`. This one is the measurement — every sample
interval derives from it — so a crystal that starts but runs off frequency is worse than one that
does not start at all.

**4. I²C to the display.** Scope `SCL` and `SDA` at J3 and confirm the module answers at `0x3C`.
**This is the step most likely to fail**, and for a known reason: `P1.7`'s `UCB0SCL` is the *second*
alternate function, not the third, and `embassy-msp430` had it wrong until it was checked against
the datasheet. If SCL never toggles, that is where to look first.

Check the module's silkscreen against J3 before plugging it in: **`GND VCC SCL SDA`**. Both orders
exist on these breakouts and the supply reversed will not do it any good.

**5. The display's power switch, and its leakage.** With the module unplugged and `P3.4` high — the
switch off — measure the current into `VCC_DISP`. **This is the measurement that decides battery
life**, and it is the one number the IRLML6401's datasheet does not give at 3 V: it quotes −1.0 µA
at −12 V and −25 µA at 55 °C, both near the part's rated voltage. At 3 V it will be far lower, but
"far lower than a microamp" is not a number and this meter's entire idle draw is 5.7 µA. Measure it
warm.

**6. The radio module.** Before fitting it, confirm three things by looking at it: **868 MHz**, **no
power LED**, and **no regulator**. An indicator LED is a milliamp — two hundred times this
instrument's whole budget, and it would flatten the cell in weeks. Then check `MARCSTATE` reads over
SPI, which validates all four wires at once.

**7. The calibration interface.** J5 is a plain UART at **115 200 8N1** — `Config::default()` in the
HAL, which the firmware does not override. Open a terminal and press Enter:

```
uflowmeter calibration
```

`I` reports the software version and the image checksum, `P` the parameters, `W <n> <value>` writes
one, `S` seals the instrument.

**Only 3.3 V adapters.** The board runs from a 3.6 V cell and an MSP430 pin tolerates `VCC + 0.3 V`.
Most FT232R breakouts carry a 5 V / 3.3 V jumper and ship on 5 V; in that position it drives 5 V
into P4.4, which is past the absolute maximum. Check the jumper before plugging anything in. An
FT230X or FT231X has no such trap — it takes its level from `VCCIO`.

Note that the FTDI **FT200XD** and **FT201X** are USB-to-**I²C** bridges, not UART, and will not
talk to J5 at all. (They would talk to the display, which is occasionally useful.)

| J5 | Adapter |
| --- | --- |
| 1 `GND` | GND |
| 2 `TX` — the board transmits | **RXD** |
| 3 `RX` — the board receives | **TXD** |

Crossed, as usual. **Do not let the adapter power the board** — it has its own supply, and two
sources on one rail is a way to find out which one wins.

**Do not send `S` until a flow rig says so.** It is irreversible: the firmware never opens this
UART again afterwards, and there is no command to unseal. That is the point of it.

**8. The transducers.** This is the part of the design that does not exist yet: what sits between
`CH0`/`CH1` and the transducers — matching, bias, protection — was never specified, because it
depends on transducers nobody had chosen. J4 brings the pins to a header so it can be worked out on
the bench. Run `uss_scope` from the HAL's examples first: it prints the captured waveform, which is
the only honest way to choose a burst threshold, a gain and a capture window.

## What you fit yourself

Six through-hole headers: `J1` (1.27 mm), `J3`, `J4`, `J5`, `J6` and `BT1`. No SMT service places
these, and they are the easiest parts on the board to solder.

## What the board still cannot tell you

No instrument is calibrated. `legal::params` holds nominal geometry — a 50 mm path at 45° through a
20 mm bore — and an instrument holding those is unsealed, reports `NotCalibrated` on every
measurement, and must not be billed from. Calibration is a flow rig and the `W`/`S` commands on J5.
