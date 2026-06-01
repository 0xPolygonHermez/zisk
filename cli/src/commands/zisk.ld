OUTPUT_FORMAT("elf64-littleriscv")
OUTPUT_ARCH("riscv")
ENTRY(_start)

MEMORY {
  rom   (xa) : ORIGIN = 0x80000000, LENGTH = 0x10000000
  ram   (wxa) : ORIGIN = 0xa0020000, LENGTH = 0x1FFE0000
}

PHDRS {
  text PT_LOAD FLAGS(5);
  rodata PT_LOAD FLAGS(4);
  data PT_LOAD FLAGS(6);
  bss PT_LOAD FLAGS(6);
  output_data PT_LOAD FLAGS(6);
}

_stack_size           = 0x400000;  /* 4 MB reserved */
_output_data_size     = 0x10000;   /* 64 KB reserved */
_float_ram_data_size  = 0x10000;   /* 64 KB reserved */

/*

0xA0000000  ┌────────────────────┐
            │ .general_registers │  64 KB reserved (NOLOAD)
            │  0x10000 bytes     │  _general_registers_start / _end
0xA0010000  ├────────────────────┤
            │ .float_registers   │  64 KB reserved (NOLOAD)
            │  0x10000 bytes     │  _float_registers_start / _end
0xA001FFFF  └────────────────────┘ ← _kernel_heap_top
0xA0020000  ┌────────────────────┐
            │ .output_data       │  64 KB reserved (NOLOAD)
            │  0x10000 bytes     │  _output_data_start / _end
0xA0030000  ├────────────────────┤
            │   .data            │
            │   .bss             │
            ├────────────────────┤ ← _bss_end
            │   stack ↑ 4MB      │
            ├────────────────────┤ ← _init_stack_top = _kernel_heap_bottom
            │   heap ↓           │
            │                    │
0xBFFEFFFF  │                    │
0xBFFF0000  ├────────────────────┤ ← _kernel_heap_top 
            │ float ram          │
0xC0000000  └────────────────────┘ 

*/

SECTIONS
{
  .text : { *(.text.init) *(.text .text.*)} >rom AT>rom :text

  . = ALIGN(8);
  PROVIDE(_global_pointer = .);
  .rodata : { *(.rodata .rodata.*)} >rom AT>rom :rodata

  /* reserved space for output data */

  .output_data (NOLOAD) : {
    PROVIDE(_output_data_start = .);
    . = . + _output_data_size; 
    PROVIDE(_output_data_end = .);
  } >ram AT>ram :output_data

  .data : { *(.data .data.* .sdata .sdata.*) } >ram AT>ram :data

  .bss : {
    PROVIDE(_bss_start = .);
    *(.bss .bss.*);
    PROVIDE(_bss_end = .); # ... and one at the end
  } >ram AT>ram :bss

  . = ALIGN(8);
  PROVIDE(_init_stack_top = . + _stack_size); # reserve 4M bytes for the initialisation stack

  PROVIDE(_kernel_heap_bottom = _init_stack_top);
  PROVIDE(_kernel_heap_top = ORIGIN(ram) + LENGTH(ram) - _float_ram_data_size);
  PROVIDE(_kernel_heap_size = _kernel_heap_top - _kernel_heap_bottom);

  _end = .;
}