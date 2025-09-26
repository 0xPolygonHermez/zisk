/* Custom linker script for custom entry point */
/* Simpler approach without complex PHDRS */
ENTRY(_start)

MEMORY
{
    ROM (rx) : ORIGIN = 0x80000000, LENGTH = 0x10000000
    RAM (rw) : ORIGIN = 0xa0020000, LENGTH = 0x1FFE0000
}

SECTIONS
{
    . = 0x80000000;  /* Start of ROM space */
    
    .header : ALIGN(4) {
        /* Our custom header/metadata section */
        KEEP(*(.header))
    } > ROM
    
    . = 0x80001000;  /* Actual code starts here */
    .text : {
        *(.text.init)
        *(.text)
    } > ROM
    
    . = ALIGN(8);
    .rodata : {
        *(.rodata)
    } > ROM
    
    /* Switch to RAM for data sections */
    . = ORIGIN(RAM);
    
    .data : {
        *(.data)
    } > RAM AT > ROM
    
    .bss : {
        *(.bss)
    } > RAM
    
    _end = .;
}