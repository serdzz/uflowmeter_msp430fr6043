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

**7. The transducers.** This is the part of the design that does not exist yet: what sits between
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
