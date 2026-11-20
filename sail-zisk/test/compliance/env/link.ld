/* The RISC-V architectural tests, linked for ZisK's memory map
 * (core/src/mem.rs): code and read-only data in the program ROM, at
 * ROM_ADDR; the signature and the rest of the data in RAM, below SYS_ADDR.
 * From ziskof's zisk/env/link.ld, with the ROM moved to its current address.
 */
OUTPUT_ARCH( "riscv" )
ENTRY(rvtest_entry_point)

SECTIONS
{
  . = 0x90000000;              /* ROM_ADDR */
  .text.init : { *(.text.init) }
  . = ALIGN(0x1000);
  .text : { *(.text) }
  .rodata : { *(.rodata .rodata.*)}
  . = 0xa0010000;              /* RAM_ADDR + 64K, below SYS_ADDR (0xa0400000) */
  . = ALIGN(0x1000);
  .tohost : { *(.tohost) }
  . = ALIGN(0x1000);
  .data : { *(.data) }
  .data.string : { *(.data.string)}
  .bss : { *(.bss) }
  _end = .;
}
