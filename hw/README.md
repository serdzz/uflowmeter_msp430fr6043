# Board

Design documents for the meter's PCB, aimed at JLCPCB.

## What this is, and what it is not

**It is** a specified circuit: every net, a bill of materials with real LCSC part numbers checked
against JLCPCB's assembly library, and the layout constraints that decide whether it works.

**It is a board you can order.** Routed, DRC clean, with the Gerbers, drill files, BOM and
placement list in `fab/`. Whether it *works* is a different question — see the end of
[`LAYOUT.md`](LAYOUT.md) for what has still never been tested. Placing and
routing a mixed-signal board with an 868 MHz section is spatial work that has to be done on a canvas
and looked at; emitting coordinates blind would produce files that look orderable and give you a
radio transmitting into a mismatched trace. The RF section in particular is copied from TI's
reference layout, not invented.

So: three documents a person can lay out from, and a BOM they can order against.

| | |
| --- | --- |
| [`SCHEMATIC.md`](SCHEMATIC.md) | every net and pin, with what is unverified stated at the end |
| [`PINOUT.md`](PINOUT.md) | all 64 pins, transcribed from the package drawing |
| [`RF.md`](RF.md) | the radio, and why it is a module rather than a chip |
| [`uflowmeter.kicad_sch`](uflowmeter.kicad_sch) | the schematic, generated and checked against the netlist |
| [`uflowmeter.kicad_pcb`](uflowmeter.kicad_pcb) | the board |
| [`mknet.py`](mknet.py) | the netlist as source — everything else is built from it |
| `./build_board.sh` | netlist → schematic → board → routing → fabrication files |
| [`bom.csv`](bom.csv) | JLCPCB BOM — LCSC numbers, tiers, notes |
| [`LAYOUT.md`](LAYOUT.md) | stackup, the three sections, design rules |

## Three findings that changed the design

### The FR6043 cannot be assembled by JLCPCB. The FR5043 can.

JLCPCB's library has no MSP430FR6043 at all. It has **MSP430FR5043IRGCR** (`C1850128`, VQFN-64,
$5.26, stock 133).

This costs nothing. The FR5043 is the same part without the LCD controller, and the display here is
an OLED on I²C — that controller was never going to be used. The firmware already builds for both:
`embassy-msp430` gained FR5043 support earlier, and the two share their code.

**Stock is 133 units.** Fine for prototypes, a supply risk for production. Check it before
committing to the part.

### The plain ER34615 will not run the radio

The cell in the energy budget is a Li-SOCl₂ D cell, and the ordinary ER34615 is a **low-drain**
cell: high internal impedance, and a passivation layer that grows while it sits unused and has to be
broken down before it can deliver current. The radio wants 35 mA in a four-millisecond pulse. The
voltage would sag, and after a quiet month it would sag much worse.

Use the **ER34615H**, which carries a hybrid layer capacitor for exactly this: the HLC delivers the
pulse while the cell delivers the average. That, plus C2 close to the radio.

This is the kind of thing that works on the bench for a week and fails in the field in January.

### The radio is a module, and that removed the hardest part of the board

The CC1101 was on this board, with its crystal, its bias resistor and the whole filter balun. All of
it is gone; J6 carries eight pins to a ready-made module instead.

What that bought: TI's reference layout no longer has to be transplanted — it exists only as CADSTAR
and Gerbers, readable by a person and not by a script — and EN 300 220 no longer has to be proven by
**conducted** measurement, which an antenna connector on your own board obliges you to do.

What it costs: unit price at volume, and a dependency on a module that must be 868 MHz, must have no
power LED, and must have no regulator with a quiescent current worth speaking of. See
[`RF.md`](RF.md); an LED alone would be two hundred times this instrument's whole budget.

## What the assembly house places, and what you do

**22 parts on 13 lines** carry LCSC numbers and are assembled: the MCU, the MOSFET, both crystals,
the Schottky and every passive.

**7 parts are fitted by hand** — all five headers, the cell terminals and the button. The headers
are through-hole and normally hand-soldered anyway; the button's footprint is a CK KSC7xx, 5.8 × 4 mm
on four pads, and JLCPCB stocks 4.5 × 4.5 mm parts that will not sit on it. Pick a stocked switch at
order time and the footprint can be swapped to match.

Three things that will otherwise be found by the assembly house rather than by you, and which
`make_fab.sh` now handles before writing anything:

* **Fiducials must not appear in the placement file.** They are copper marks, not parts, and a house
  that is asked to place FID1 will write and ask what it is.
* **Every designator in the placement file needs a line in the BOM.** `BT1` lost its line when the
  BOM was rewritten for the radio module, and nothing noticed until the order was uploaded.
* **A paste layer with nothing on it does not belong in the archive.** Every part on this board is
  on the top, so the bottom paste gerber comes out as a bare header — and a house asked to make a
  stencil of nothing quite reasonably asks what you meant. Any paste layer with no apertures is now
  dropped.

## Cost

| | |
| --- | --- |
| Extended part types | 4 → **$12** one-time per order |
| Silicon | ~$7 (U1 $5.26, U2 $1.42, Q1 $0.04) |
| Crystals | ~$0.51 |

Y1 and Y2 are **basic** parts, which is luck worth keeping — they carry no loading fee.

## Before this can be ordered

In order:

1. **Read the FR5043 datasheet's package pinout** and turn the pin *names* in `SCHEMATIC.md` into
   pin *numbers*. Not done here.
2. **Check Table 7-1** for every peripheral pin. `embassy-msp430` carries its own note that its
   alternate-function assignments are derived rather than confirmed, so a wrong alternate means that
   peripheral silently does not appear on that pin.
3. **Specify the USS front end.** The transducer drive and ADC return pins are not in this
   schematic; they are the part of the circuit that measures water, and they are missing.
4. Download TI's **SWRR045** and copy the newest reference design — there are three versions and two
   of them are marked not to be used.
5. Draw the layout against `LAYOUT.md`.
6. DRC, then check 50 Ω against JLCPCB's calculator for the stackup actually ordered.

Steps 1–3 are firmware-adjacent and could be done from this repository. Steps 4–6 need a layout
tool and somebody to look at the result.
