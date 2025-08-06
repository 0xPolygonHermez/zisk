# Test case demonstrating custom entry point with program headers
# Program headers occupy 0x80000000-0x80000FFF (4096 bytes)
# "Actual" code starts at 0x80001000

.section .header
    # Program header/metadata starting at 0x80000000
    .word 0xDEADBEEF   # Magic number at 0x80000000
    .word 0x00000001   # Version
    .word 0x80001000   # Entry point address (where _start is)
    .word 0x00000100   # Program size

.section .text.init
.global _start

# This will be placed at 0x80001000 due to linker script
_start:
    # Add some code so that theres code
    li a0, 42
    li a1, 58
    add a2, a0, a1
    
    # Exit
    li a7, 93
    ecall
    
1:  j 1b