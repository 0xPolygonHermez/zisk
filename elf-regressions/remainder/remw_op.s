.section .text.init
.global _start

_start:
    li a0, -100         # negative dividend (32-bit signed)
    li a1, 7            # divisor (32-bit signed)
    remw a2, a0, a1     # 32-bit signed remainder: -100 % 7 = -2
    
    # Exit
    li a7, 93
    ecall
    
1:  j 1b