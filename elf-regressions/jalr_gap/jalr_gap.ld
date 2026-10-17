/* Simple linker script - just start at 0x90001000 instead of 0x90000000 */

ENTRY(_start)

SECTIONS
{
    . = 0x90001000;  /* Start at 0x90001000 instead of default 0x90000000 */
    
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