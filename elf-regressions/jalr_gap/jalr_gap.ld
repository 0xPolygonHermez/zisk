/* Simple linker script - just start at 0x80001000 instead of 0x80000000 */

ENTRY(_start)

SECTIONS
{
    . = 0x80001000;  /* Start at 0x80001000 instead of default 0x80000000 */
    
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