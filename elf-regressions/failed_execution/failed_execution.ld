/* Code at the ZisK ROM base, without the ELF headers in the loaded segment. */
ENTRY(_start)
SECTIONS { . = 0x80000000; .text : { *(.text.init) *(.text*) } }
