# Pinout — MSP430FR5043IRGC, 64-pin VQFN

Transcribed from **Figure 7-2** of SLASEF5B, the package drawing. Not from Table 7-1, which is laid
out in a way that already caused one wrong answer in this design — see the note at the end.

## The whole package

| | | | |
| ---: | --- | ---: | --- |
| 1 | AVCC1 | 33 | DVSS2 |
| 2 | P2.2/COUT/UCA0CLK/A14/C14 | 34 | P5.0/TB0.0/UCA2SIMO/UCA2TXD |
| **3** | **P1.0**/UCA1CLK/TA1.0/A0/C0/VREF− | 35 | P5.1/TB0.1/UCA2SOMI/UCA2RXD |
| 4 | P1.1/UCA1STE/TA4.0/A1/C1/VREF+ | 36 | P5.2/TB0.2/UCA2CLK |
| 5 | AVSS2 | 37 | P5.3/TB0.3/UCA2STE |
| **6** | **PJ.4/LFXIN** | 38 | P5.4/TA0.0/UCB1CLK/TA4.0 |
| **7** | **PJ.5/LFXOUT** | 39 | P5.5/TA4.1/UCB1SIMO/UCB1SDA |
| 8 | AVSS3 | 40 | P5.6/TB0OUTH/UCB1SOMI/UCB1SCL |
| 9 | PJ.6/HFXIN/USSXT_BOUT | 41 | P5.7/TA0.2/UCB1STE |
| 10 | PJ.7/HFXOUT | 42 | P6.0/TA0CLK/COUT |
| **11** | **TEST/SBWTCK** | 43 | P6.4/MCLK |
| **12** | **RST/NMI/SBWTDIO** | 44 | P6.5/SMCLK |
| 13 | PJ.0/UCA2CLK/…/TDO | 45 | P6.6/ACLK |
| 14 | PJ.1/UCA2STE/…/TDI/TCLK | 46 | P7.0/TA1.0/TA1.2/XPB0 |
| 15 | PJ.2/UCA2SIMO/…/TMS | 47 | P6.2/TB0CLK |
| 16 | PJ.3/UCA2SOMI/…/TCK | 48 | DVSS3 |
| **17** | **DVSS1** | **49** | **DVCC3** |
| **18** | **DVCC1** | 50 | P3.3/MCLK/TB0.3/XPB1 |
| **19** | **P1.2**/UCA1SIMO/UCA1TXD/TA1.0/A8/C8 | **51** | **P3.4**/SMCLK/DMAE0 |
| **20** | **P1.3**/UCA1SOMI/UCA1RXD/TA1.1/A9/C9 | **52** | **P3.5**/ACLK/COUT |
| 21 | P2.0/UCA1CLK/UCA3SIMO/**UCA3TXD** | **53** | **CH1_IN** |
| 22 | P2.1/UCA1STE/UCA3SOMI/UCA3RXD | **54** | **CH1_OUT** |
| **23** | **P1.6**/UCA3STE/UCB0SIMO/**UCB0SDA** | **55** | **PVSS** |
| **24** | **P1.7**/USSTRG/UCA3CLK/UCB0SOMI/**UCB0SCL** | **56** | **PVCC** |
| **25** | **P1.4**/TB0.4/UCB0STE/A2/C2 | **57** | **PVCC** |
| 26 | P1.5/TB0.5/UCB0CLK/A3/C3 | **58** | **PVSS** |
| 27 | P3.1/TA1CLK/TB0.1/MTIF_OUT_IN | **59** | **CH0_OUT** |
| 28 | P4.0/RTCCLK/TA4.1/MTIF_PIN_EN | **60** | **CH0_IN** |
| 29 | P4.1/UCA0CLK/TB0.4/UCA3SOMI | 61 | AVSS4 |
| 30 | P4.2/UCA0STE/TB0.5/UCA3SIMO | **62** | **USSXTIN** |
| **31** | **P4.3**/UCA0SIMO/**UCA0TXD** | **63** | **USSXTOUT** |
| **32** | **P4.4**/UCA0SOMI/**UCA0RXD** | 64 | AVSS1 |

Bold is what this board uses.

## What the board connects

| Use | Pin | Signal |
| --- | ---: | --- |
| Radio SPI CLK | 3 | P1.0 / UCA1CLK — primary |
| Radio SPI MOSI | 19 | P1.2 / UCA1SIMO — primary |
| Radio SPI MISO | 20 | P1.3 / UCA1SOMI — primary |
| Radio CS | 52 | P3.5 — GPIO |
| Display SDA | 23 | P1.6 / UCB0SDA — **secondary** |
| Display SCL | 24 | P1.7 / UCB0SCL — **secondary** |
| Display power | 51 | P3.4 — GPIO |
| **Button** | **25** | **P1.4 — GPIO** |
| Calibration TX | 31 | P4.3 / UCA0TXD — primary |
| Calibration RX | 32 | P4.4 / UCA0RXD — primary |
| Debug | 11, 12 | TEST/SBWTCK, RST/SBWTDIO |
| Timekeeping | 6, 7 | LFXIN, LFXOUT — Y1 32.768 kHz |
| Measurement | 62, 63 | USSXTIN, USSXTOUT — Y2 8 MHz |
| Transducer A | 59, 60 | CH0_OUT, CH0_IN |
| Transducer B | 54, 53 | CH1_OUT, CH1_IN |
| Supply | 1, 18, 49 | AVCC1, DVCC1, DVCC3 |
| Ultrasonic supply | 56, 57 | PVCC |
| Ground | 5, 8, 17, 33, 48, 61, 64 | AVSS2/3/4/1, DVSS1/2/3 |
| Ultrasonic ground | 55, 58 | PVSS |

## Three things the drawing settled

**The button cannot be on P2.0.** Note A on the drawing: *"On devices with UART BSL: P2.0 is BSLTX,
P2.1 is BSLRX."* This device has the UART bootloader, so the BSL drives P2.0 as an output. A button
shorting it to ground would fight that driver. Moved to **P1.4**, pin 25 — interrupt-capable, and
carrying nothing else this design uses.

**There is no `VCORE` pin on this package.** An earlier draft of the schematic had a 470 nF core
decoupling capacitor on one. It is gone.

**Note B confirms the I²C assignment** independently: *"On devices with I²C BSL: P1.6 is BSLSDA,
P1.7 is BSLSCL."* P1.6 is the data line and P1.7 the clock, which is what section 9.14 said and what
`embassy-msp430` now does.

## Why not Table 7-1

It lists each pin's signals as a block, with the port name floating inside the block and the
pin-number cell centred beside it. Read it as rows and every port comes out one pin off — which in
this design first made it look as though P1.0 and P1.2 were missing from the 64-pin package, and
then produced a wrong alternate-function number that would have held I²C SCL low on the finished
board.

Figure 7-2 has one name against one number. Section 9.14 adds the `PxSEL1`/`PxSEL0` encoding. Use
those two and nothing else.
