# Simple addition test - Just to check that the scripts work
# If this fails, there is likely something wrong with the build-environment
# or the code.

.section .text.init
.global _start

_start:
    li a0, 42           # Load first number
    li a1, 58           # Load second number
    add a2, a0, a1      # a2 = 42 + 58 = 100
 
    # Exit
    li a7, 93           # Exit syscall number
    ecall
    
1:  j 1b                # Infinite loop