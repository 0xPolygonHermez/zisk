/* Custom linker script with program headers in .text section */
/* This is here because golang does this */
ENTRY(_start)

SECTIONS
{
    /* Text section starts at standard ROM address */
    . = 0x80000000;
    
    .text : {
        /* Program headers occupy first 4096 bytes (0x1000) */
        *(.header)
        
        /* Pad to 4KB to ensure code starts at 0x80001000 */
        . = ALIGN(0x1000);
        
        /* Entry point and code start here at 0x80001000 */
        *(.text.init)
        *(.text)
    }
    
    . = ALIGN(8);
    .data : {
        *(.data)
    }
    
    . = ALIGN(8);
    .bss : {
        *(.bss)
    }
    
    /* Stack at a fixed location */
    . = 0x80100000;
    _stack_top = .;
}