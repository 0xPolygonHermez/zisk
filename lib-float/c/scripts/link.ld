OUTPUT_FORMAT("elf64-littleriscv")
OUTPUT_ARCH("riscv")
ENTRY(_zisk_float)

MEMORY {
  rom   (xa) : ORIGIN = 0x87F00000, LENGTH = 0x100000
  ram   (wxa) : ORIGIN = 0xBFFF0000, LENGTH = 0x10000
}

PHDRS {
  text PT_LOAD FLAGS(1);
  rodata PT_LOAD FLAGS(4);
  data PT_LOAD FLAGS(6);
  bss PT_LOAD FLAGS(6);
}

SECTIONS
{
  .text : { *(.text.init) *(.text .text.*)} >rom AT>rom :text

  . = ALIGN(8);
  PROVIDE(_global_pointer = .);
  . = ALIGN(8);
  .rodata : { *(.rodata .rodata.*)} >rom AT>rom :rodata

  .data : { *(.data .data.* .sdata .sdata.*) } >ram AT>ram :data

  . = ALIGN(8);
  .bss : {
    PROVIDE(_bss_start = .);
    *(.bss .bss.*);
    PROVIDE(_bss_end = .);
  } >ram AT>ram :bss

  . = ALIGN(8);
  PROVIDE(_init_stack_top = . + 0x100000);

  . = ALIGN(8);
  _end = .;
}