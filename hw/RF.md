# The 868 MHz section

Values from **TI's CC1101EM 868/915 MHz reference design, revision 3.0.0** (SWRR045) — the version
DN017 labels newest and recommended. Two older versions exist and are marked "not recommended" and
"should not be used"; this is neither of them.

## Topology

```
RF_P (12) ──L121 12n──┬───────────┬──C122 1.5p──┬──L123 12n──┬──L124 12n──┬──C125 12p──┬── ANT
                      │           │             │            │            │            │
                     C121        L122          (node C)     C123         C126 47p      │
                     1.0p        18n                        3.3p            │          │
                      │           │                          │           L125 3.3n     │
RF_N (13) ──L131 12n──┴──┬──L132──┘             GND ─────────┘              └──────────┘
                         │   18n
                       C131 1.5p            C124 100p from L122 to GND
                         │
                        GND
```

`C126` and `L125` sit in series *across* `C125`. `C125` is part of the matching either way — only
those two come out if the notch is not wanted.

## Bill of materials

| Ref | Value | Package | Type |
| --- | --- | --- | --- |
| L121, L123, L124, L131 | 12 nH | 0402 | **wirewound**, Murata LQW15A |
| L122, L132 | 18 nH | 0402 | **wirewound**, Murata LQW15A |
| L125 | 3.3 nH | 0402 | **wirewound** — notch |
| C121 | 1.0 pF | 0402 | NP0, ±0.25 pF, 50 V |
| C122, C131 | 1.5 pF | 0402 | NP0, ±0.25 pF, 50 V |
| C123 | 3.3 pF | 0402 | NP0, ±0.25 pF, 50 V |
| C124 | 100 pF | 0402 | NP0, ±5%, 50 V |
| C125 | 12 pF | 0402 | NP0, ±5%, 50 V |
| C126 | 47 pF | 0402 | NP0, ±5%, 50 V — notch |
| **R171** | **56 kΩ** | 0402 | 1% — `RBIAS`, pin 17 |
| C81 | 12 pF | 0402 | NP0, 50 V — X1 load |
| C101 | 15 pF | 0402 | NP0, 50 V — X1 load |
| X1 | 26.000 MHz | | |
| C41…C151 | 100 nF | 0402 | X5R, 10 V — one per supply pin |
| C1 | 1 µF | 0805 | X7R, 16 V |
| L1 | ferrite bead, 1 kΩ @ 100 MHz | 0402 | in the VDD feed |

Three of these were missing from the first draft of `SCHEMATIC.md` and would each have cost a board
spin:

* **R171, the `RBIAS` resistor.** The CC1101 sets its internal bias current through it. Without it
  the radio does not work at all.
* **The crystal load caps are not equal** — 12 pF and 15 pF. Fitting a matched pair is the obvious
  thing to do and is not what TI specifies.
* **The ferrite bead in the supply feed**, which is what keeps the radio's own switching out of the
  rest of the board.

## Wirewound, and specifically these

The reference schematic says it in a note on the drawing: *"Wire Wound inductors have been used in
the balun and LC filter. Murata LQW15A series."*

DN017's Table 5 is why it matters. Same circuit, multilayer inductors, `PA = 0xC0`: second harmonic
−30.8 dBm and EN 300 220 compliance recorded as **"no margin"**. Wirewound: −34.8 dBm, passes, and
12.0 dBm out at 35.0 mA. Multilayer 0402 inductors are the cheaper default and the wrong part.

## The notch

`C126` + `L125` attenuate a spurious emission at carrier − 169 MHz — 699 MHz for an 868 MHz carrier,
where EN 300 220 allows −54 dBm and the bare CC1101 exceeds it.

Needed here, because this board brings the antenna out to a connector and compliance is then proven
by **conducted** measurement, which sees the spur. A board with an integrated antenna is assessed
radiated and can leave `C126` and `L125` unmounted.

## Layout

Copy the reference layout, `CC1101EM_868_915MHz_LAYOUT_3_0_0.pdf`, component for component. The
values above are only correct together with the parasitics they were characterised on. The archive
also carries the Gerbers and the PADS `.pcb`, so the geometry can be lifted rather than redrawn.
