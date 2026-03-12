# Test c.li instruction - compressed load immediate
# Tests various immediate values including edge cases

.section .text.init
.global _start

_start:
    # Test basic positive values
    c.li x1, 0
    c.li x2, 1
    c.li x3, 31
    
    # Test negative values (sign-extended)
    c.li x4, -1
    c.li x5, -32
    
    # Test boundary values
    c.li x6, 15
    c.li x7, -16
    
    # Test with different registers
    c.li t0, 10
    c.li t1, 20
    c.li a0, 5
    c.li a1, -5
    
    # Exit
    li a7, 93
    ecall
    
1:  j 1b