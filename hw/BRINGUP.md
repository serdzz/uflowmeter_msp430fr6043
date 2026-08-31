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
switch off — measure the current into `VCC_DISP`. The IRLML6401's datasheet does not give this at
3 V: it quotes −1.0 µA at −12 V and −25 µA at 55 °C, both near the part's rated voltage. At 3 V it
will be far lower, but "far lower than a microamp" is not a number. Measure it warm.

Measure it to catch a **faulty part or a bad joint**, not because it threatens the battery — see the
section on the cell below, which is the correction to what an earlier draft of this file claimed.

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

**8. The transducers, with `uss_scope`.** This is the part of the design that does not exist yet:
what sits between `CH0`/`CH1` and the transducers — matching, bias, protection — was never
specified, because it depends on transducers nobody had chosen. J4 brings the pins to a header so it
can be worked out on the bench.

**Run `uss_scope` before the meter firmware, and before believing any number a meter produces.**
It is in the HAL's examples, it speaks over the same J5 header at the same 115 200, and it prints
the samples the receiving transducer actually produced — one per line, 200 of them, every five
seconds, so a capture pastes straight into anything that plots a column of numbers.

What the plot answers, in the order the questions matter:

* **Is the transducer ringing at all?** A flat line means the excitation is not reaching it — look at
  J4, the wiring, and whether `PVCC` is present, before suspecting anything subtle.
* **Where in the capture does the burst arrive?** If it is at the very edge or absent, the window is
  looking at the wrong time and no threshold will fix that.
* **How big is it?** That sets the gain and the threshold.
* **Has the transmitter stopped ringing before the echo lands?** If the two overlap, the stop pulses
  need adjusting — `config.rs` sends two.

If the USS will not start at all it says so: *"USS did not start: check the crystal and the supply"*.
That points at Y2 and `PVCC`, which is step 3's business, not the transducers'.

### Clamp-on transducers: yes for the bench, no for the instrument

Clamping a pair to any pipe is the quickest way to get a first waveform — no spool piece, no
plumbing, and you can move them about while watching the plot. For bringing the front end up it is
the right first step.

What has to change, all of it in the driver's `Config`:

| | |
| --- | --- |
| `pga_gain` | the signal crosses two pipe walls, so it arrives 20–40 dB weaker |
| `burst_threshold` | lower to match — `W 7` over J5 |
| `SAMPLES` (200) | a longer window: sound travels through the wall as well as the water |
| `pulses` (10) | more energy into the transducer |

You also need **wedges** — usually acrylic — to set the refraction angle, and couplant. Without the
wedges the beam does not enter the water at a useful angle at all.

**But not for a billing instrument, and the reason is not signal quality.** This firmware is built
for MID and WELMEC 7.2, where the instrument must be sealed and its metrological characteristics
verifiable. A clamp-on measurement depends on things that are not part of the instrument and cannot
be sealed with it:

* the pipe's wall thickness and material, which set the sound speed in the wall and the geometry;
* the couplant, which dries, creeps and ages;
* where the wedges sit, which nobody seals.

Slide the wedges a centimetre and the reading changes, and **the seal on the instrument will not
notice**. The write counter, the image checksum and `zero_offset_ps` all protect what is inside the
case, while the measurement is decided by what is outside it. That is why clamp-on meters are rare
in custody transfer — not because they measure badly, but because they cannot be sealed whole.

That is reasoning from the framework this firmware is built in, not a quotation from it. If you are
seriously considering clamp-on for billing, it is the first question for the notified body, and the
answer is likely to be no.

There is a subtler problem too. `zero_offset_ps` is the largest error in a meter of this kind. With
wetted transducers it comes from the transducers' own asymmetry and holds steady, which is why it
can be calibrated once and sealed. With clamp-on it also comes from the couplant, and **the couplant
changes with temperature and with age**. So the one parameter the calibration exists to pin down
drifts after calibration — and the firmware will not notice: it subtracts the old value and reports
a confident wrong number.

### Choosing the threshold

`uss_scope` finishes each pair of captures with a line like:

```
# peaks 812 774, try threshold 387
```

That suggestion is **half the smaller of the two peaks** — comfortably above the noise, and below
both. The *smaller* one matters: a threshold that catches one direction earlier than the other
biases the flight-time difference, and the flight-time difference is the entire measurement.

Then write it into the instrument, over J5:

```
W 7 387
```

Parameter 7 is `burst_threshold`. It defaults to **400**, which is a guess written when there was no
hardware — replace it with what the scope actually measured. It is per instrument for a reason: the
transducers vary batch to batch, and this is the number that decides when the firmware believes an
echo has arrived.

Two things about the threshold that a single capture will not show:

* **Too low and it triggers on the transmitter's own ring-down** rather than the echo, which reads
  as a flight time far too short.
* **It moves with temperature**, because the signal weakens as the transducers warm. A threshold
  chosen at 20 °C wants checking at the extremes of the range the meter is meant to work over.

## The cell, and what actually limits it

The design is **not capacity-limited**, and knowing that redirects effort away from things that do
not matter.

| | Circuit | + self-discharge | Years |
| --- | ---: | ---: | ---: |
| Idle, no radio fitted | 3.4 µA | 25.1 | **86** |
| Water moving, no radio | 11.1 | 32.8 | 66 |
| Idle, radio fitted | 5.7 | 27.4 | **79** |
| Water moving, radio fitted | 13.4 | 35.1 | 62 |

An ER34615 is 19 Ah, and a bobbin Li-SOCl₂ cell self-discharges at roughly **1 % a year** — 190 mAh,
which is **21.7 µA if you write it as a current**.

**That is more than the whole instrument draws.** Four times the idle figure without a radio, twice
the worst case with one. So the limit on this design is the cell's shelf life and its passivation,
not its capacity, and every number above is four to six times the ten to fifteen years a meter of
this kind is built for.

Two things follow.

**The MOSFET's leakage matters far less than an earlier draft of this file said.** At 0.5 µA the
answer is 78 years instead of 79; at 5 µA it is 67. For leakage to halve the life it would have to
reach about 25 µA, which is not leakage but a dead transistor. Measure it to find a fault, not to
save the battery.

**Without the radio fitted, a plain ER34615 will do.** The hybrid layer capacitor in the `H` version
exists for the radio's 35 mA pulse; with no radio the largest draw is 4 mA for 1.1 ms from the
ultrasonic front end, which a bobbin cell handles. Fit the `H` anyway if it is to hand — after
months idle the passivation layer raises the internal impedance and even 4 mA can pull the voltage
down — but it is insurance now rather than a requirement.

## What you fit yourself

Six through-hole headers: `J1` (1.27 mm), `J3`, `J4`, `J5`, `J6` and `BT1`. No SMT service places
these, and they are the easiest parts on the board to solder.

## What the board still cannot tell you

No instrument is calibrated. `legal::params` holds nominal geometry — a 50 mm path at 45° through a
20 mm bore — and an instrument holding those is unsealed, reports `NotCalibrated` on every
measurement, and must not be billed from. Calibration is a flow rig and the `W`/`S` commands on J5.
