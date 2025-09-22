/* Linker script for FIPS markers test */
ENTRY(_start)

SECTIONS
{
    /* Standard ROM address */
    . = 0x80000000;
    
    /* Create a separate section for FIPS start marker */
    .fipsstart : {
        PROVIDE(_fipsstart = .);
        *(.fipsstart)
    }
    
    /* Main text section with the actual code */
    .text : {
        *(.text.start)  /* Entry point code */
        *(.text)        /* Other text */
    }
    
    /* Create a separate section for FIPS end marker */
    .fipsend : {
        PROVIDE(_fipsend = .);
        *(.fipsend)
    }
    
    /* Data sections */
    . = ALIGN(8);
    .data : {
        *(.data)
    }
    
    . = ALIGN(8);
    .bss : {
        *(.bss)
    }
    
    /* Stack */
    . = 0x80100000;
    _stack_top = .;
}