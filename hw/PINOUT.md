# Pinout — MSP430FR5043IRGCR (VQFN-64, RGC)

Read off **SLASEF5B**, the FR6043/FR5043 family datasheet, December 2021 revision. Both packages are
given because the firmware currently builds for the 80-pin FR6043 while the board uses the 64-pin
FR5043.

## Where these numbers come from, and where they do not

**Section 9.14's port tables**, not Table 7-1.

Table 7-1 lists each pin's signals in a block with the port name floating in the middle of it, and
the pin-number cell centred vertically beside the block. Read it quickly and the port name appears
to belong to the row above — which puts every port off by one pin and, worse, makes it look as if
P1.0 and P1.2 are absent from the 64-pin package. They are not.

Section 9.14 gives, per port pin, an explicit `PN 80` column, an `RGC 64` column, each function, and
the `PxSEL1`/`PxSEL0` encoding that selects it. It is unambiguous. Use it.

The ultrasonic and power pins are not in section 9.14 — they are not GPIO — so those come from
Table 7-1, where they are single-function rows with the number on the same line and no ambiguity.

## Function select

`PxSEL1:PxSEL0` — `00` GPIO, `01` primary, `10` secondary, `11` tertiary.

## Digital

| Use | Port | PN80 | **RGC64** | Function | Select |
| --- | --- | ---: | ---: | --- | --- |
| Radio SPI CLK | P1.0 | 4 | **3** | UCA1CLK | primary |
| Radio SPI MOSI | P1.2 | 25 | **19** | UCA1SIMO | primary |
| Radio SPI MISO | P1.3 | 26 | **20** | UCA1SOMI | primary |
| Display SDA | P1.6 | 29 | **23** | UCB0SDA | **secondary** |
| Display SCL | P1.7 | 30 | **24** | UCB0SCL | **secondary** |
| Button | P2.0 | 27 | **21** | GPIO | — |
| Display power | P3.4 | 65 | **51** | GPIO | — |
| Radio CS | P3.5 | 66 | **52** | GPIO | — |
| Calibration TX | P4.3 | 37 | **31** | UCA0TXD | primary |
| Calibration RX | P4.4 | 38 | **32** | UCA0RXD | primary |

**P1.7 is a trap.** Its signal list reads `P1.7/USSTRG/UCA3CLK/UCB0SOMI/UCB0SCL`, and counting along
that list makes `UCB0SCL` look like the third alternate. It is the second: Table 9-27 lists `USSTRG`
as an *independent function* with no select encoding of its own. Choosing the third alternate
selects the row Table 9-27 marks "N/A — internally tied to DVSS", which holds SCL low and means no
I²C transfer can ever start. `embassy-msp430` had exactly this wrong.

## Ultrasonic front end

Its own power domain, `PVCC`/`PVSS`, separate from `AVCC` and `DVCC`. Four pins of it.

| Signal | PN80 | **RGC64** | Type |
| --- | ---: | ---: | --- |
| CH1_IN | 67 | **53** | analog in, PVCC |
| CH1_OUT | 68 | **54** | analog out, PVCC |
| PVSS | 69 | 55 | power |
| PVCC | 70 | 56 | power |
| PVCC | 71 | 57 | power |
| PVSS | 72 | 58 | power |
| CH0_OUT | 73 | **59** | analog out, PVCC |
| CH0_IN | 74 | **60** | analog in, PVCC |
| AVSS4 | 77 | 61 | power |
| USSXTIN | 78 | **62** | analog in, **1.5 V** |
| USSXTOUT | 79 | **63** | analog out, **1.5 V** |
| AVSS1 | 80 | 64 | power |

`CH0_OUT`/`CH0_IN` and `CH1_OUT`/`CH1_IN` are the two transducers: the channel drives its
transducer on `_OUT` and returns the received echo on `_IN`.

### Two rules the datasheet states outright

Footnote 6 of Table 7-1, in its own words:

> Do not connect the USSXTIN and USSXTOUT pins to AVCC or DVCC. USSXTIN does not support bypass
> mode, so do not drive an external clock to USSXTIN pin.

So the 8 MHz reference **must be a crystal**, not an oscillator module, and the pins sit in a 1.5 V
domain of their own — treat them as analog, not as a logic clock input.

`PVCC` wants its own decoupling at the pins, and its return should reach the star ground separately
from the digital one. This is the supply that swings while a transducer is driven.
