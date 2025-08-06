/* Custom linker script for JALR test with non-standard entry point */
/* Entry point at 0x80001000 instead of 0x80000000 */

ENTRY(_start)

SECTIONS
{
    . = 0x80000000;  /* Start of ROM space */
    
    .header : {
        /* Reserve space for header - filled with zeros */
        . = . + 0x1000;  /* 4KB header space */
    }
    
    . = 0x80001000;  /* Actual code starts here */
    .text : {
        *(.text.init)
        *(.text)
    }
    
    .rodata : {
        *(.rodata)
    }
    
    .data : {
        *(.data)
    }
    
    .bss : {
        *(.bss)
    }
    
    _end = .;
}