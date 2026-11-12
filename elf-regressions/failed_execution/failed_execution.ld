/* Code at the ZisK ROM base, without the ELF headers in the loaded segment. */
ENTRY(_start)
SECTIONS { . = 0xA8000000; .text : { *(.text.init) *(.text*) } }
