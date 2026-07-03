OUTPUT_FORMAT("elf64-littleriscv")
OUTPUT_ARCH("riscv")
ENTRY(_start)

MEMORY {
  rom   (xa) : ORIGIN = 0x80000000, LENGTH = 0x08000000
  ram   (wxa) : ORIGIN = 0xa0000000, LENGTH = 0x20000000
}

PHDRS {
  text PT_LOAD FLAGS(5);
  rodata PT_LOAD FLAGS(4);
  data PT_LOAD FLAGS(6);
  bss PT_LOAD FLAGS(6);
  output_data PT_LOAD FLAGS(6);
  general_registers_data PT_LOAD FLAGS(6);
  float_registers_data PT_LOAD FLAGS(6);
  stack_data PT_LOAD FLAGS(6);
}

_stack_size                  = 0x400000;  /* 4 MB reserved */
_general_registers_data_size = 0x8000;    /* 32 kB */
_float_registers_data_size   = 0x8000;    /* 32 kB */
_output_data_size            = 0x20000;   /* 128 KB reserved */
_float_ram_data_size         = 0x10000;   /* 64 KB reserved */

/*

0x4000_0000  ┌────────────────────┐← free input entry
             │    input data      │
             │       (1GB)        │
0x7FFF_FFFF  └────────────────────┘
0x8000_0000  ┌────────────────────┐
             │     ROM data       │             
             │      (128MB)       │
0x87FF_FFFF  └────────────────────┘
                       ↑
                    768 MB
                       ↓ 
0xA000_0000  ┌────────────────────┐← _init_stack_bottom
             │                    │
             │   stack ↑ 4MB      │
             │                    │
0xA040_0000  ├────────────────────┤← _init_stack_top
             │ .general_registers │  32 KB reserved (NOLOAD)
             │  0x8000 bytes      │  _general_registers_data_start / _end
0xA040_8000  ├────────────────────┤
             │ .float_registers   │  32 KB reserved (NOLOAD)
             │  0x8000 bytes      │  _float_registers_data_start / _end
0xA041_0000  ├────────────────────┤
             │ .output_data       │  128 KB reserved (NOLOAD)
             │  0x20000 bytes     │  _output_data_start / _end
0xA043_0000  ├────────────────────┤
             │   .data            │
             │   .bss             │
             ├────────────────────┤ ← _bss_end, _heap_bottom
             │   heap ↓           │
             │                    │
             │                    │
             │                    │
             │                    │
0xBFFF_0000  ├────────────────────┤ ← _heap_top 
             │ float ram          │
0xC000_0000  └────────────────────┘

*/

SECTIONS
{
  .text : { *(.text.init) *(.text .text.*)} >rom AT>rom :text

  . = ALIGN(8);
  PROVIDE(_global_pointer = .);
  .rodata : { *(.rodata .rodata.*)} >rom AT>rom :rodata

  /* C++ static constructors / destructors (run by _start before/after main),
     placed in ROM: the entries are link-time-constant function pointers, never
     written. KEEP prevents --gc-sections from dropping them. The init/fini array
     bound symbols are referenced (strongly) by the runtime, so this section MUST
     be present when linking against libziskos. Empty when the program has no
     static initializers (start == end), so it costs nothing. */
  . = ALIGN(8);
  .init_array : {
    PROVIDE(__init_array_start = .);
    KEEP(*(SORT_BY_INIT_PRIORITY(.init_array.*)))
    KEEP(*(.init_array))
    PROVIDE(__init_array_end = .);
  } >rom AT>rom :rodata

  .fini_array : {
    PROVIDE(__fini_array_start = .);
    KEEP(*(SORT_BY_INIT_PRIORITY(.fini_array.*)))
    KEEP(*(.fini_array))
    PROVIDE(__fini_array_end = .);
  } >rom AT>rom :rodata

  /* reserve stack at the beginning of RAM */
  . = ORIGIN(ram);

  /* reserved space for stack data */
  .stack_data (NOLOAD): {
  PROVIDE(_stack_start = .);
  . = . + _stack_size;
  PROVIDE(_init_stack_top = .); # reserve 4M bytes for the initialisation stack
  } >ram AT>ram :stack_data
  
  /* reserved space for general registers */

  .general_registers_data (NOLOAD) : {
    PROVIDE(_general_registers_data_start = .);
    . = . + _general_registers_data_size; 
    PROVIDE(_general_registers_data_end = .);
  } >ram AT>ram :general_registers_data

  /* reserved space for float registers */

  .float_registers_data (NOLOAD) : {
    PROVIDE(_float_registers_data_start = .);
    . = . + _float_registers_data_size; 
    PROVIDE(_float_registers_data_end = .);
  } >ram AT>ram :float_registers_data

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

  PROVIDE(_heap_bottom = .);
  PROVIDE(_heap_top = ORIGIN(ram) + LENGTH(ram) - _float_ram_data_size);
  PROVIDE(_heap_size = _heap_top - _heap_bottom);

  _end = .;
}
