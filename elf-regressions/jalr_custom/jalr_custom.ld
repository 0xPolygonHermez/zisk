/* Custom linker script for JALR test with non-standard entry point */
/* Entry point at 0xA8001000 instead of 0xA8000000 */

ENTRY(_start)

MEMORY
{
    ROM (rx)  : ORIGIN = 0xA8000000, LENGTH = 0x10000000
    RAM (rw)  : ORIGIN = 0xC0020000, LENGTH = 0x7FFE0000
}

SECTIONS
{
    . = 0xA8000000;  /* Start of ROM space */
    
    .header : ALIGN(4) {
        /* Reserve space for header - filled with zeros */
        KEEP(*(.header))
    } > ROM
    
    . = 0xA8001000;  /* Actual code starts here */
    .text : {
        *(.text.init)
        *(.text)
    } > ROM
    
    . = ALIGN(8);
    .rodata : {
        *(.rodata)
    } > ROM
    
    /* Switch to RAM address space for data sections */
    . = 0xC0020000;
    
    .data : {
        *(.data)
    } > RAM AT > ROM
    
    .bss : {
        *(.bss)
    } > RAM
    
    _end = .;
}