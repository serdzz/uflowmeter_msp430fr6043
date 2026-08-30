# Board

Design documents for the meter's PCB, aimed at JLCPCB.

## What this is, and what it is not

**It is** a specified circuit: every net, a bill of materials with real LCSC part numbers checked
against JLCPCB's assembly library, and the layout constraints that decide whether it works.

**It is not a board you can order.** There are no Gerbers, because there is no layout. Placing and
routing a mixed-signal board with an 868 MHz section is spatial work that has to be done on a canvas
and looked at; emitting coordinates blind would produce files that look orderable and give you a
radio transmitting into a mismatched trace. The RF section in particular is copied from TI's
reference layout, not invented.

So: three documents a person can lay out from, and a BOM they can order against.

| | |
| --- | --- |
| [`SCHEMATIC.md`](SCHEMATIC.md) | every net and pin, with what is unverified stated at the end |
| [`PINOUT.md`](PINOUT.md) | pin numbers and mux encodings, read off the datasheet |
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

### Two conditions for selling it in Europe

Both from TI Design Note DN017, neither obvious from the CC1101 datasheet.

**The matching inductors must be wirewound.** With multilayer inductors the second harmonic is
−30.8 dBm and TI's own measurement of EN 300 220 compliance is "no margin". With wirewound, it
passes. Multilayer is the cheaper default and the wrong choice.

**A 699 MHz notch filter is required.** The CC1101 emits a spur there above the −54 dBm EN 300 220
allows. Because this board uses an antenna connector, compliance is proven by conducted measurement,
which sees it. Three parts: 12 pF, 47 pF, 3.3 nH.

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
