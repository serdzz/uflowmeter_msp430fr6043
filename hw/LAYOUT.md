# Layout

**This is a specification, not a finished layout.** Nothing here has been placed or routed. The
constraints below are the ones that decide whether the board works; a layout that satisfies them is
still a layout somebody has to draw and check.

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

## Before ordering

1. Run DRC clean.
2. Check the 50 Ω trace width against **JLCPCB's** impedance calculator for the exact stackup.
3. Confirm every U1 pin number against the FR5043 datasheet's package pinout. **This has not been
   done** — see `SCHEMATIC.md`.
4. Confirm the alternate-function table (Table 7-1) for every peripheral pin. **Also not done.**
