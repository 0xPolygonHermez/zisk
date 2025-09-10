# Basic compressed instructions

.section .text.init
.global _start

_start:
    c.li a0, 10         # Compressed load immediate (c.li)
    c.li a1, 20         # Compressed load immediate (c.li)  
    c.add a0, a1        # Compressed add (c.add a0, a0 + a1)
    
    # Exit using regular instruction
    li a7, 93           # Exit syscall number
    ecall
    
1:  j 1b                # Infinite loop