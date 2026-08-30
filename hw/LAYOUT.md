# Layout

**This is a specification, not a finished layout.** Nothing here has been placed or routed. The
constraints below are the ones that decide whether the board works; a layout that satisfies them is
still a layout somebody has to draw and check.

## The board file

Open [`uflowmeter.kicad_pcb`](uflowmeter.kicad_pcb) — 54 footprints placed on a 70 × 55 mm four-layer
outline, with filled pours on every layer. **DRC reports zero violations.**

| | |
| --- | ---: |
| Unconnected at first placement | 134 |
| After the pours were filled | 86 |
| After 16 stitching vias took `VCC` down to its plane | 70 |
| Now | **69** |

So ground and supply are done — through the planes, as they should be — and what is left is signal
routing.

[`uflowmeter.kicad_pro`](uflowmeter.kicad_pro) carries the net classes:

| Class | Track | Nets |
| --- | ---: | --- |
| RF | 0.30 mm | `RF_P`, `RF_N`, `RFA`…`RFE`, `ANT`, `RF_SHUNT` |
| Power | 0.50 mm | `VCC`, `GND`, `RADIO_VDD`, `VCC_DISP`, `BAT+` |
| Ultrasonic | 0.25 mm | `USSXTIN`, `USSXTOUT`, `CH0`, `CH1` |
| Default | 0.20 mm | everything else |

Clearance is 0.15 mm on every class, which is not laziness: a 0.5 mm pitch QFN has only 0.25 mm
between its own pads, so a wider clearance makes DRC flag the packages themselves.

[`uflowmeter.net`](uflowmeter.net) is the netlist the board was built from, kept so the board can be
regenerated or re-imported.

There is no drawn schematic to maintain — [`SCHEMATIC.md`](SCHEMATIC.md) is the record instead.

### What DRC found while this was being placed

Two things worth keeping. `PVSS` — U1 pins 55 and 58, the ultrasonic ground — **was missing from the
netlist entirely**, and only showed up as two pads with no net. And the transducer header's
courtyard runs 11 mm down the board, far past its own body, which is why the 8 MHz crystal sits to
the left of it rather than under it.

## Placement

Already done in the board file, in the order below, because each region constrains the next. It is a
**starting placement**: DRC-clean and clustered correctly, but drawn without a human eye on it, so
expect to move things.

1. **U1 and Y2 first.** The 8 MHz crystal is the measurement — put it against U1's pins 62/63 with
   a ground guard and its own vias, and let nothing else compete for that corner. Y1 goes near
   pins 6/7.
2. **The ultrasonic corner next**, along U1's pins 53–60. J4 comes straight out of the board edge
   from there. `PVCC` decoupling sits at pins 56/57; `PVSS` returns on its own path.
3. **The RF block last, and as a unit.** U2, Y3 and the whole L121…C126 chain, copied from
   `CC1101EM_868_915MHz_LAYOUT_3_0_0.pdf` component for component, then J2 at the board edge. Do
   not let this block be reshaped to fit — reshape everything else around it.

Then the cheap stuff: Q1 beside the display net, SW1 where a finger reaches it, J1 and J5 anywhere
convenient on the edge, C1 at the cell terminals.

**The right half is a front panel.** The display module hangs from J3 at the top (body x 38.3…65.3,
y 6…33) with the button directly below it (x 47.2…56.4, y 35.9…46.1). Everything tall was moved out
from under the module: the transducer header to the top left, the cell and the calibration header to
the right edge below the panel.

`U1` and its decoupling remain under the module, which is fine to build — the module clears them on
its header standoff — and awkward to rework, since probing the MCU means lifting the display.

## Stackup

**Four layers.** JLCPCB's JLC04161H-7628, 1.6 mm, 1 oz outer.

| Layer | Use |
| --- | --- |
| 1 | components, RF, short signal runs |
| 2 | **unbroken ground** |
| 3 | `VCC` and slow signals |
| 4 | components, ground pour |

Two layers would be cheaper by a few dollars and wrong. The RF section needs a continuous reference
plane directly under it, and the ultrasonic front end needs a quiet one. Neither survives a
ground plane with routing cut through it.

## The three sections, kept apart

### RF — copy TI, do not improvise

Copy the CC1101EM 868/915 MHz reference layout (SWRR045) **component for component and trace for
trace**, including its ground via placement. The matching values are only valid together with the
parasitics of that layout.

* 50 Ω controlled impedance from the balun to J2. On JLC04161H-7628, a microstrip over layer 2 is
  about **0.30 mm wide** — confirm against JLCPCB's own impedance calculator for the stackup you
  order, do not take that number from here.
* Ground vias flanking the RF trace, every 2–3 mm.
* No routing on layer 2 anywhere under the RF section.
* Y3 (26 MHz) as close to U2 as the footprints allow, with its own ground vias.

### Ultrasonic front end — the quiet corner

* Y2 (8 MHz) **as close to U1 as physically possible**, with a ground guard and its own vias. This
  crystal is the measurement; noise coupled into it is noise in every reading.
* Keep the transducer runs to J4 short, matched in length to each other, and away from the radio
  and the display's switched rail.
* No digital switching under the analog return path.

### Digital and display

The display's switched rail carries 630 µA and is unremarkable. The only rule that matters:

* **R3 and R4 connect to `VCC_DISP`, never to `VCC`.** This is the single easiest mistake to make on
  this board and its symptom is a battery that dies years early with nothing else wrong.

## Crystal load capacitors

Do not copy a value out of a reference design. For each crystal:

```
C_load = 2 x (C_L - C_stray)      C_stray ~ 3 pF for a short SMD trace
```

Y1 (Epson Q13FC13500004) is C\_L = 12.5 pF, so C6/C7 ≈ 2 × (12.5 − 3) = **19 pF** → fit 18 pF and
check the frequency. The MSP430's LFXT also has selectable internal load capacitance; if that is
used, C6/C7 come off the board entirely. Decide which, and write it down.

## Power

* C1 (100 µF) at the cell terminals, C2 (10 µF) at U2. **The radio's 35 mA pulse must come out of
  C2**, not down the cell leads.
* Star the ground: analog return, digital return and the radio's return meet at one point near the
  cell negative.
* Keep the cell leads short and twisted, or the 35 mA pulse edge radiates.

## Mechanical

Unspecified — it depends on the meter body, which does not exist yet. What the layout has to know
before it starts: where the transducers enter, where the display faces, and where the button is
reachable through the enclosure.

## Design rules

JLCPCB's standard capability is enough; nothing here needs their advanced process.

| | |
| --- | --- |
| Min track / gap | 0.127 mm (5 mil) |
| Min via | 0.3 mm hole / 0.6 mm pad |
| Min annular ring | 0.13 mm |

## What is still missing

**Signal routing.** Ground and the supplies are carried by the planes and are connected; the 69
remaining items are signals.

They were left rather than forgotten. Routing them by straight line between pads was tried and
refused itself: of eleven two-pad nets, ten were blocked, because a straight run from a crystal to
its `QFN` pin necessarily passes over the neighbouring pins. Real routing escapes from the package
first and changes layer, which is a router's job or a person's, not a script's.

**The RF chain was deliberately not routed at all.** Its geometry has to be lifted from
`CC1101EM_868_915MHz_LAYOUT_3_0_0.pdf`, and a generic trace between those pads would be worse than
no trace, because it would look finished.

One thing to know before routing: the top layer already carries a filled ground pour. KiCad refills
around new tracks, so this is harmless, but it is the reverse of the usual order and can be
surprising. That is the part
that has to be done on a canvas with somebody looking at it, and the RF section in particular is
copied geometry rather than anything to be improvised.

## Before ordering

1. Route it.
2. Run DRC clean.
3. Check the 50 Ω trace width against **JLCPCB's** impedance calculator for the exact stackup
   ordered.
4. Confirm U1's exposed-pad size against TI's RGC package drawing — the netlist uses KiCad's
   generic `EP5.45x5.45mm`, which is a starting point rather than a checked figure.
