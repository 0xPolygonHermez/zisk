ENTRY(nitro_dispatch)
PHDRS { text PT_LOAD FLAGS(1); rodata PT_LOAD FLAGS(4); data PT_LOAD FLAGS(6); }
SECTIONS {
  . = 0x97000000;
  .text : { *(.text .text.*) } :text
  . = ALIGN(0x1000);
  .rodata : { *(.rodata .rodata.* .srodata .srodata.*) } :rodata
  ASSERT(. <= 0x97e00000, "Nitro runtime ROM exceeds reservation")
  . = 0xa0600000;
  .data : { *(.data .data.* .sdata .sdata.*) } :data
  .bss (NOLOAD) : { *(.bss .bss.* .sbss .sbss.*) *(COMMON) } :data
  ASSERT(. <= 0xa0fd0000, "Nitro runtime RAM exceeds reserved region")
  __stylus_heap_start = 0xb7000000;
  __stylus_heap_end = 0xbefd0000;
  /DISCARD/ : { *(.comment .eh_frame .riscv.attributes) }
}
