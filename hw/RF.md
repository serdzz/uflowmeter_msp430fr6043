# The radio

**A module on a header, not a chip on this board.** J6 carries eight pins:

```
1 GND   2 VCC   3 GDO0   4 CSN   5 SCK   6 MOSI   7 GDO2   8 MISO
```

`GDO0` and `GDO2` are left unconnected — the firmware polls `MARCSTATE` over SPI rather than
watching a pin, because a frame is four milliseconds and the CPU has nothing else to do in them.

## What this decision removed

Twenty parts and the two hardest problems on the board:

| Gone | |
| --- | --- |
| U2, Y3, R5, L2, C11–C17 | the CC1101 itself, its 26 MHz crystal, its bias resistor, its supply bead and decoupling |
| L121–L132, C121–C131 | the whole filter balun and the 699 MHz notch |
| J2 | the u.FL antenna connector |

With them went the requirement to transplant TI's reference layout — which exists only as CADSTAR
and Gerbers, readable by a person and not by a script — and the requirement to prove EN 300 220 by
**conducted** measurement, which is what an antenna connector on your own board obliges you to do.

The firmware did not change. It still speaks to a real CC1101 over the same four wires, and every
register value in `radio/cc1101.rs` still applies.

## What the module has to be

Three things, and the third is the one that quietly ruins a battery meter.

**868 MHz.** The same CC1101 silicon covers 315, 433, 868 and 915 MHz, but a module's matching
network and antenna do not. A 433 MHz module will accept every register write and radiate almost
nothing at 868.

**Without a power LED.** An indicator LED is a milliamp, continuously. This instrument's entire
budget is 5.7 µA. One LED is two hundred times the whole meter and would flatten the cell in weeks.
Most bare CC1101 modules have none — check the one in your hand rather than the photograph.

**Without a regulator, or with one whose quiescent current is nanoamps.** The board feeds the module
3.6 V directly, which the CC1101 takes natively. A module carrying an LDO in front of it adds that
LDO's quiescent current to the budget, permanently.

If the module is pre-certified to EN 300 220, that is worth more than its price difference: it moves
the radio compliance off your instrument entirely.

## What is still true from before

TI's Design Note DN017 measured this silicon at **35.0 mA transmitting**, and that is what
`energy.rs` prices. A module does not change the current; it changes who is responsible for the
matching network that current flows into.
