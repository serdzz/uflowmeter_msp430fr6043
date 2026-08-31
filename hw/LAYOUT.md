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
| After routing the signal nets | **34** |

Of those 34, seventeen are the RF chain, left alone on purpose. The other seventeen are signals on
fifteen nets the routers could not get through.

Ground and the supplies go through the planes. 355 tracks, 60 vias, DRC clean. Rebuild with:

```
python3 route_maze.py    uflowmeter.kicad_pcb fat   # A* on a 0.05 mm grid, ~2.5 min
python3 route_cleanup.py uflowmeter.kicad_pcb       # drop whatever DRC objects to
python3 route_escape.py  uflowmeter.kicad_pcb       # cruder pass for what is left
python3 route_cleanup.py uflowmeter.kicad_pcb
```

using KiCad's own Python, which is the only one with `pcbnew`.

`route_cleanup.py` exists because the routers model clearance as inflated rectangles on a grid,
which is close to KiCad's computation but not identical. Where they disagree KiCad is right, so it
drops the offending tracks and leaves those connections for a person — better than shipping a board
that fails its own rule check.

[`uflowmeter.kicad_pro`](uflowmeter.kicad_pro) carries the net classes:

| Class | Track | Nets |
| --- | ---: | --- |
| RF | 0.30 mm | `RF_P`, `RF_N`, `RFA`…`RFE`, `ANT`, `RF_SHUNT` |
| Power | 0.50 mm | `VCC`, `GND`, `RADIO_VDD`, `VCC_DISP`, `BAT+` |
| Ultrasonic | 0.25 mm | `USSXTIN`, `USSXTOUT`, `CH0`, `CH1` |
| **Default** | **0.15 mm** | everything else |

### Why the default track is 0.15 mm and not 0.25

Because at 0.25 mm nothing can leave a QFN pin at all, and it takes a picture of the grid to see
why. Around `U1` pin 25, with a 0.25 mm track's keep-out painted:

```
y301-309   4X5555X OOOO X5555555     the pin, flanked by its neighbours
y310-312   XXXXXXXXXXXXXX55555       a solid band, claimed by two nets at once
y313+      ...................       open board
```

`O` is the pin, `5` a neighbour, `X` a cell two nets both need and neither may have. The neighbours
sit 0.5 mm away and are 0.3 mm wide, so their copper edge is 0.35 mm from this pin's axis. A
0.25 mm track needs 0.275 mm of room from its centreline; add the clearance both sides and the two
keep-outs meet over the escape corridor and close it.

At 0.15 mm the track needs 0.225 mm, the keep-outs stop short of each other, and the corridor
opens. This is why fine-pitch escapes are routed thin and then widened once clear of the package —
and it is worth doing that widening by hand on the power-carrying runs.

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

### The radio — a header, and nothing to get wrong

J6 is eight pins to a module. There is no matching network on this board, no controlled impedance
and no antenna connector, because there is no RF on it: the module carries all of that.

* C2 (10 µF) at J6, so the 35 mA transmit pulse comes out of a capacitor rather than down the cell
  leads.
* Keep the module's footprint clear of the ultrasonic corner. It is still a radio, and the front end
  is still measuring picoseconds.

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

**Seventeen unconnected items are signals**, on fifteen nets: `CH0`, `I2C_SCL`, the `SPI` group,
`XOSC_Q1`/`Q2`, `USSXTIN`/`USSXTOUT`, `RADIO_CS`, `RADIO_DCOUPL`, `RADIO_RBIAS`, `TEST`, `UART_TX`.
They want the interactive router, which can push and shove what is already there; these only ever
add.

Most of them are the last pins of the two QFNs, which is where the room runs out first.

**The rest is the RF chain, deliberately untouched.** Its geometry has to be lifted from
`CC1101EM_868_915MHz_LAYOUT_3_0_0.pdf`, and a generic trace between those pads would be worse than
no trace, because it would look finished.

### The two passes, and which tracks came from which

**`route_maze.py`** is a grid router: A* on both layers, eight directions, a via costed at about
five millimetres of track. Octile movement is what gives it the shape — its runs come out
horizontal, vertical or at 45°, the way a person draws them.

**`route_escape.py`** is the cruder pass that follows: escape each pad to a via, then take whatever
straight or L-shaped run is clear. Its tracks are the long arbitrary diagonals, and they are the
ones worth redoing by hand.

Two things the grid router taught, both worth keeping:

* **A cell claimed by two nets belongs to neither.** On a 0.5 mm pitch package the neighbouring
  pins' keep-outs overlap, and letting the first claimant own the cell is exactly how a track ends
  up crossing its neighbour's pad — 67 violations' worth, the first time.
* **The keep-out cannot be generous.** It is painted as a rectangle, so a margin much over the
  0.275 mm the rules need closes the one way out of a QFN pin, which is straight outward along the
  pin's own axis. At 0.38 mm nothing escaped at all.

Treat all of it as a first pass that saves the tedious part, not as a finished layout. `USSXTIN`
and `USSXTOUT` in particular want redoing by hand — that crystal is the measurement — and so does
the I²C pair, which should not take a long unshielded run past the radio.

One thing to know: the top layer carries a filled ground pour. KiCad refills around new tracks, so
this is harmless, but it is the reverse of the usual order and can be surprising. That is the part
that has to be done on a canvas with somebody looking at it, and the RF section in particular is
copied geometry rather than anything to be improvised.

## Before ordering

1. Route it.
2. Run DRC clean.
3. Check the 50 Ω trace width against **JLCPCB's** impedance calculator for the exact stackup
   ordered.
4. Confirm U1's exposed-pad size against TI's RGC package drawing — the netlist uses KiCad's
   generic `EP5.45x5.45mm`, which is a starting point rather than a checked figure.

## Manufacturing files

`./make_fab.sh` writes everything JLCPCB asks for into `fab/`:

* `uflowmeter-jlcpcb.zip` — the four copper layers, both masks, both silkscreens, both paste
  layers, the outline, and the plated and non-plated drill files
* `uflowmeter-bom.csv` — Comment / Designator / Footprint / LCSC Part #
* `uflowmeter-cpl.csv` — Designator / Mid X / Mid Y / Layer / Rotation

`finish_board.py` adds the three things a bare board needs and a router will not: fiducials in an
asymmetric L so the assembly machine can find the board and tell which way round it is — a QFN-64
on 0.5 mm pitch is placed from those, not from the outline — and a legend on both sides.

Regenerate the whole board from the netlist with:

```
python3 mkboard.py       uflowmeter.net uflowmeter.kicad_pcb
python3 route_maze.py    uflowmeter.kicad_pcb fat
python3 route_cleanup.py uflowmeter.kicad_pcb
python3 route_escape.py  uflowmeter.kicad_pcb
python3 route_cleanup.py uflowmeter.kicad_pcb
python3 route_rf.py      uflowmeter.kicad_pcb
python3 finish_board.py  uflowmeter.kicad_pcb
./make_fab.sh
```

using KiCad's own Python, which is the only one carrying `pcbnew`.

## It is routed

**Zero DRC violations, zero unconnected items, ERC clean.** 32 footprints, 222 tracks, 89 vias, on
70.1 × 55.1 mm of four-layer board.

Getting the last connections in took a rule that is worth stating once, because it caused every
failure at the end:

> A 0.6 mm via needs **0.525 mm** of clear board to a foreign track — its own radius, plus the
> clearance, plus the other track's half width. Escapes from adjacent pins of a 0.5 mm pitch
> package run 0.5 mm apart. That is always a hair too close.

So a via never goes directly behind a pin. One of each adjacent pair fans sideways first — which is
the standard fine-pitch escape, and here it was needed four times: `I2C_SCL`/`BUTTON`,
`DISP_GATE`/`RADIO_CS`, `UART_TX`/`UART_RX`, and `USSXTIN`/`USSXTOUT`.

The nets the grid router could not reach are in [`route_by_hand.py`](route_by_hand.py) as explicit
waypoints, under a discipline that makes crossings impossible rather than unlikely: **verticals on
the top layer, horizontals on the bottom, a via at every corner, a lane of its own for each net.**
Two verticals cannot meet — their x differs. Two horizontals cannot meet — their y differs. A
vertical and a horizontal are on different layers. Drawn by eye instead, the same routes produced
seven crossings.

### What still has to be true before it works

Ordering it is not the same as it working. Unchanged from the rest of this repository:

* **Nothing has been near a transducer, a pipe or a battery.** No instrument has been calibrated;
  `legal::params` still holds nominal geometry.
* **What sits between `CH0`/`CH1` and the transducers is not designed.** The pins and their power
  domain are specified; matching, bias and protection are not, and depend on transducers nobody has
  chosen. J4 brings them to a header so that can be worked out on the bench.
* **The radio module must meet the three conditions in [`RF.md`](RF.md)** — 868 MHz, no power LED,
  no regulator with a quiescent current worth speaking of.
* **The FR5043's stock at JLCPCB was 133 pieces.** Check it before committing to an order.
