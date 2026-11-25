/* Simple linker script - just start at 0xA8001000 instead of 0xA8000000 */

ENTRY(_start)

SECTIONS
{
    . = 0xA8001000;  /* Start at 0xA8001000 instead of default 0xA8000000 */
    
    .text : {
        *(.text)
    }
    
    . = ALIGN(8);
    .rodata : {
        *(.rodata)
    }
    
    . = ALIGN(8);
    .data : {
        *(.data)
    }
    
    . = ALIGN(8);
    .bss : {
        *(.bss)
    }
}