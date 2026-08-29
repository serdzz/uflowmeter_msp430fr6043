# uflowmeter — MSP430FR6043

An ultrasonic water meter, built for a battery. Ping both transducers, work out how much water went
past, add it to a reading in FRAM, switch the analog section off, sleep.

The brief was minimum energy, so that is the axis every decision here is made along, and each one is
written down where it is made — `config.rs` for the settings, `energy.rs` for what they add up to.

## What it draws

`energy.rs` computes this from the settings, at compile time, so it cannot drift away from what the
firmware does:

| | Front end | Correlation | Per measurement | Average |
| --- | ---: | ---: | ---: | ---: |
| Water moving, every 2 s | 1 100 µs | 14 000 µs | 16.5 µC | **9.7 µA** |
| Nothing moving, every 30 s | 1 100 µs | 14 000 µs | 16.5 µC | **2.1 µA** |

Two things in that table are not what one would guess.

**The correlation costs more than the ultrasound** — 11.2 µC of the 16.5. The analog front end that
does the measuring is powered for about a millisecond; the CPU then spends fourteen milliseconds
correlating, because this build links the software multiply routines and every multiply is a
subroutine call. `config::CORRELATION_SAMPLES` is the lever that controls it.

**The front end's millisecond is nearly all crystal start-up**, not capture. The capture itself is
100 µs. Shortening the capture further would save almost nothing; what would save something is
measuring less often, which is what the adaptive interval does.

## The number that matters more than any of these

A 19 Ah lithium thionyl chloride D cell self-discharges at something like one per cent a year. That
is 190 mAh a year, which is an average of **21.7 µA** — ten times what the meter draws when idle.

So the firmware is already an order of magnitude below the point where its own consumption decides
anything. Further optimisation here buys nothing; what decides how long this meter runs is the cell
and whatever else is on the board. That is worth knowing before spending a week shaving microamps.

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

Current size: **14 140 bytes of flash and 1 256 of RAM**, against about 40 kB of reachable FRAM and
4 kB of RAM. Most of the RAM is the sample buffer.

## What has not happened

None of this has been near a transducer, a pipe, or a battery.

* **The geometry in `config.rs` is a placeholder** — a 50 mm path at 45 degrees through a 20 mm
  bore. It has to come from the meter body the firmware ends up in, and the reading is directly
  proportional to it.
* **The burst threshold is a guess.** Run `uss_scope` from the HAL's examples first: it prints the
  captured waveform, which is the only way to choose a threshold, a gain and a capture window.
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
