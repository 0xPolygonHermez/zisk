/* Custom linker script for JALR test with non-standard entry point */
/* Entry point at 0x80001000 instead of 0x80000000 */

ENTRY(_start)

MEMORY
{
    ROM (rx)  : ORIGIN = 0x80000000, LENGTH = 0x10000000
    RAM (rw)  : ORIGIN = 0xa0020000, LENGTH = 0x1FFE0000
}

SECTIONS
{
    . = 0x80000000;  /* Start of ROM space */
    
    .header : ALIGN(4) {
        /* Reserve space for header - filled with zeros */
        KEEP(*(.header))
    } > ROM
    
    . = 0x80001000;  /* Actual code starts here */
    .text : {
        *(.text.init)
        *(.text)
    } > ROM
    
    .rodata : {
        *(.rodata)
    } > ROM
    
    /* Switch to RAM address space for data sections */
    . = 0xa0020000;
    
    .data : {
        *(.data)
    } > RAM AT > ROM
    
    .bss : {
        *(.bss)
    } > RAM
    
    _end = .;
}