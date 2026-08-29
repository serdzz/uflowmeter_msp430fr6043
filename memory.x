/* MSP430FR5043 and FR6043, as far as a 16-bit target can see it.
 *
 * One map for both: the two devices are register-identical bar the segment LCD driver, and their
 * memory maps are the same down to the byte.
 *
 * 64 kB of FRAM, of which 0x6000 to 0xFF7F is reachable with 16-bit addressing -- about 40 kB. The
 * 24 kB above 0x10000 needs the 20-bit addressing Rust's msp430 target does not have.
 *
 * TOTALS is the meter's own FRAM. It is a region of its own only so the linker cannot put code
 * there; unlike flash it needs no erase and no unlock, so the totals are written in place, a word
 * at a time, as often as the meter likes. That is the whole reason this part suits a battery
 * meter: a flash device would need a sector erase for the same job, costing about a thousand times
 * the energy and a finite number of cycles.
 *
 * The 8 kB at 0x4000-0x5FFF is the LEA accelerator's memory and is left out. The 16 bytes at
 * 0xFF80 are the JTAG, BSL and IPE signatures: writing them by accident locks the part.
 */
MEMORY
{
  RAM     : ORIGIN = 0x1C00, LENGTH = 0x1000  /* 0x1C00 ..= 0x2BFF, 4 kB */
  TOTALS  : ORIGIN = 0x6000, LENGTH = 0x0100  /* 0x6000 ..= 0x60FF, 256 B of FRAM */
  ROM     : ORIGIN = 0x6100, LENGTH = 0x9E80  /* 0x6100 ..= 0xFF7F */
  VECTORS : ORIGIN = 0xFF92, LENGTH = 0x006E  /* 54 interrupt vectors and the reset vector */
}

/* Bounds of the legally relevant image, for the periodic checksum WELMEC 7.2 P5 requires.
 *
 * The end is not a symbol the runtime provides, so it is worked out at run time instead: the
 * initialisers for .data are the last thing in ROM, and `_sidata` says where they start. See
 * `legal::identity`.
 */
_legal_start = ORIGIN(ROM);
