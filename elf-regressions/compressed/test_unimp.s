# Test case to reproduce 16 bit zeroes

.section .text.init
.global _start

_start:
    unimp        # Assembler generates 0x0000 (in RVC mode) 
    
    li a7, 93
    ecall

1:  j 1b                # Infinite loop