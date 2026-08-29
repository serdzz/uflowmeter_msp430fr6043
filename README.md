# uflowmeter — MSP430FR5043 / FR6043

An ultrasonic water meter, built for a battery.

Builds for either device unchanged. The **FR5043 is the default**: it is the FR6043 without the
segment LCD driver, which this meter has no use for, in a 64-pin package instead of 80. It costs
about a third less and, at the time of writing, is the one that can actually be bought — the FR6043
is out of stock at distributors with a 16-week factory lead time.

```sh
cargo build --release                                          # FR5043, the default
cargo build --release --no-default-features -F msp430fr6043    # FR6043
```

Both come to exactly the same size: nothing in the firmware touches the difference. Ping both transducers, work out how much water went
past, add it to a reading in FRAM, switch the analog section off, sleep.

The brief was minimum energy, so that is the axis every decision here is made along, and each one is
written down where it is made — `config.rs` for the settings, `energy.rs` for what they add up to.

## What it draws

`energy.rs` computes this from the settings, at compile time, so it cannot drift away from what the
firmware does:

| | Front end | Correlation | Display | Average |
| --- | ---: | ---: | ---: | ---: |
| Water moving, every 2 s | 1 100 µs | 14 000 µs | 1.35 µA | **11.1 µA** |
| Nothing moving, every 30 s | 1 100 µs | 14 000 µs | 1.35 µA | **3.4 µA** |

Two things in that table are not what one would guess.

**The correlation costs more than the ultrasound** — 11.2 µC of the 16.5. The analog front end that
does the measuring is powered for about a millisecond; the CPU then spends fourteen milliseconds
correlating, because this build links the software multiply routines and every multiply is a
subroutine call. `config::CORRELATION_SAMPLES` is the lever that controls it.

**The front end's millisecond is nearly all crystal start-up**, not capture. The capture itself is
100 µs. Shortening the capture further would save almost nothing; what would save something is
measuring less often, which is what the adaptive interval does.

### The display

An SSD1306 OLED, shown for 45 seconds when the button is pressed and **unpowered the rest of the
time** — a high-side switch on its supply, not the controller's sleep command. That distinction is
the whole design: a sleeping SSD1306 module draws about 26 µA, which is more than this meter and
more than the cell's own self-discharge, so a display that merely slept would be the instrument's
largest consumer while showing nothing.

While it is up the reading is **live**: the meter keeps measuring — once a second rather than its
usual interval — and the display is redrawn after each. A frozen snapshot would be no use for the
thing people actually stand in front of a meter to do, which is watch the last digits with
everything closed and see whether anything is running.

At four presses a day the display and that faster measuring together average 1.35 µA — under what
the meter itself draws. Continuously on it would be 630 µA, three hundred times the meter.

#### The switch

An **IRLML6401** P-channel MOSFET: source to the 3 V rail, drain to the display, gate to the MCU pin
with a **1 MΩ pull-up to the rail** beside it. The pin is therefore **low to turn the display on** —
stated once in `display::Power` so the inversion cannot be read the wrong way round elsewhere.

The pull-up is not optional. Between reset and the firmware's first write that pin is an input, and
a floating gate leaves the switch in a state nobody has decided.

At 3 V the part is fully on: `VGS(th)` is −0.95 V worst case and `RDS(on)` is 0.085 Ω at −2.5 V,
which across 630 µA is a drop of fifty nanovolts. On-resistance is simply not a consideration here.

**Leakage is**, and the datasheet does not answer it at 3 V: `IDSS` is quoted as −1.0 µA at −12 V
and 25 °C, and −25 µA at −9.6 V and 55 °C — both near the part's full rated voltage, where leakage
is at its worst. At 3 V it will be far lower, but "far lower than a microamp" is not a number, and
this meter's entire idle draw is 2.1 µA. **Measure it on the board, warm.** It is the one thing
about this switch that could quietly halve the battery life, and the only symptom would be cells
coming back early from the field years later.

#### The pull-ups

The I2C pull-ups belong on the *switched* rail. Pull-ups above an unpowered chip push current
through its protection diodes — a leak invisible on a bench supply and fatal to a ten-year battery.
The firmware does its half by dropping the I2C driver before cutting power, which returns SDA and
SCL to GPIO inputs.

## The number that matters more than any of these

A 19 Ah lithium thionyl chloride D cell self-discharges at something like one per cent a year. That
is 190 mAh a year, which is an average of **21.7 µA** — ten times what the meter draws when idle.

So the firmware is already an order of magnitude below the point where its own consumption decides
anything. Further optimisation here buys nothing; what decides how long this meter runs is the cell
and whatever else is on the board. That is worth knowing before spending a week shaving microamps.

## Legal metrology

This is meant to be sold, which makes it a measuring instrument under **MID 2014/32/EU, annex
MI-001**, and its software something a notified body assesses against **WELMEC Guide 7.2**. That
shapes the firmware, and retrofitting it later is expensive, so it is in from the start.

| WELMEC 7.2 | Where |
| --- | --- |
| P2, software identification | `legal::identity` — a version and a CRC32 of the image |
| P5, protection against accidental change | the same checksum, computed by the device's CRC32 hardware at every start-up |
| P7, parameter protection | `legal::params` — a seal, a write counter that only rises, a checksum |
| P4, influence via communication interfaces | `calibration` — the production interface does not run on a sealed instrument |
| S1, software separation | the `legal` module boundary, though separation is **not** claimed |

**The whole software is declared legally relevant.** Proving separation costs more assessment effort
than this firmware would save. The price is that any change anywhere needs a new version in
`identity::SOFTWARE_VERSION` and, depending on what changed, another look from the notified body.

### Calibration is data, not code

The change that matters most for a product: the geometry, the zero offset and the correction factor
are **per instrument**, written on a flow rig into FRAM and sealed. Firmware that hard-codes them
can be a demonstration; it cannot be a product, because there is nowhere to put what the calibration
produced.

`zero_offset_ps` is the one to watch. The two directions are never quite symmetric, and that
asymmetry — not the ultrasound, not the arithmetic — is the largest error in a meter of this kind.

### The production interface

A line-based ASCII protocol on `eUSCI_A0`, served **only while the instrument is unsealed**. Once
`S` succeeds the firmware never opens the UART again, which answers P4 and the energy question at
once: an instrument in the field has no listening interface.

```
I              software version, image checksum, image length
P              the current parameters
W <n> <value>  set parameter n          (refused once sealed)
S              seal. There is no unseal command
```

## Where the energy went, in order

1. **How often it measures.** Everything else is a rounding error beside it. Hence the adaptive
   interval in `meter.rs`: two seconds while water moves, thirty when it has not for a while, and
   straight back on the first measurement that finds movement. What it costs is being late to
   notice a tap opening; against a domestic pattern of hours of nothing then minutes of something,
   it is the cheapest trade in the design.
2. **The correlation window**, for the reason in the table above.
3. **Nothing external draws standby current.** No battery divider — the internal half-supply channel
   does the same job and draws nothing between conversions, where a 1 MΩ divider would be 3.6 µA
   forever. No temperature sensor: the velocity equation in `flow.rs` is the form that cancels the
   speed of sound, so temperature is not needed to measure, only to log, and the on-chip sensor is
   plenty for that. No external memory: see below. The cheapest component is the one not fitted.
4. **LPM3, not the executor's default of LPM0.** One line in `main.rs`, and LPM0 would leave SMCLK
   and the DCO running around the clock for nothing.
5. **8 MHz, no FRAM wait states.** Faster needs wait states, so the energy per instruction rises;
   slower just lengthens the active time. This is the minimum, not a compromise.
6. **The crystal, not the internal oscillator**, and this is the one place the lower-power option is
   refused: ACLK times the interval the flow is integrated over, and a clock 20% fast makes the
   totals 20% wrong. Its couple of hundred nanoamps do not appear in the sum.

## Why FRAM changes the design

The reading is written every measurement — every two seconds — to a plain struct at a fixed address.
On a flash part that would be unthinkable: no in-place rewrite, a sector erase costing milliseconds
and millijoules, and ten thousand cycles before the sector dies. The usual answer is a wear-levelled
journal, a few hundred lines of careful code that still wears out.

FRAM writes in place, byte by byte, with no erase, for about a hundred nanojoules, ten to the
fifteenth times. So `totals.rs` is a struct and two writes. Two copies, written in a fixed order
with a checksum, so a battery pulled mid-write costs at most the newer one.

This is the clearest reason the part suits the job.

## Building

```sh
cargo build --release
```

Needs a nightly toolchain and TI's `msp430-elf-gcc`; see `embassy-msp430`'s README. The HAL comes
from a checkout of [serdzz/embassy](https://github.com/serdzz/embassy) on `dev/msp430`, expected as
a sibling directory.

Current size: **26 066 bytes of flash and 1 542 of RAM**, on either device, against about 40 kB of
reachable FRAM and 4 kB of RAM. Most of the RAM is the sample buffer. About 14 kB of FRAM is left,
which is what the radio has to fit into.

## What has not happened

None of this has been near a transducer, a pipe, or a battery.

* **No instrument has been calibrated.** `legal::params` defaults to nominal geometry — a 50 mm
  path at 45 degrees through a 20 mm bore — and an instrument holding those is unsealed, reports
  `NotCalibrated` on every measurement, and must not be billed from. The reading is directly
  proportional to the geometry, so this is not a detail.
* **The burst threshold is a guess.** Run `uss_scope` from the HAL's examples first: it prints the
  captured waveform, which is the only way to choose a threshold, a gain and a capture window.
* **Nothing is deposited to compare the image checksum against.** P5 asks for the computed value to
  be checked against a nominal; there is nowhere to have put one yet. That belongs with the
  production step that also seals the parameters.
* **The time-of-flight estimator is not TI's library** and will not match its accuracy. It has no
  answer for zero-flow drift, which is the error that decides whether a meter is billable. See the
  `uss` module docs.
* **The currents in `energy.rs` are datasheet typicals.** Good for comparing two configurations,
  which is what they are for. Not a battery life.
* **The temperature reading is uncalibrated** — the per-device two-point calibration in the `TLV`
  table is not read. It is good for a trend, not as an absolute. Nothing depends on it.

## The next energy lever, and why it is not pulled

The correlation would be several times faster using the hardware multiplier, which this device has.
`.cargo/config.toml` links the software routines instead (`-mhwmult=none`), because the multiplier
is a peripheral with shared registers: a multiply inside an interrupt handler corrupts one it
interrupted, and this build cannot promise that no handler in the HAL multiplies.

Making that promise — auditing the handlers that this firmware actually enables, and pinning it with
a test — is worth roughly ten times the saving of any remaining setting. It is also, per the
self-discharge argument above, worth nothing at all in battery life.
