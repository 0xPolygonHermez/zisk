# Test c.add instruction - compressed add register to register

.section .text.init
.global _start

_start:
    # Test basic addition
    li x8, 10
    li x9, 20
    c.add x8, x9         # x8 = 10 + 20 = 30
    
    # Verify result
    li t0, 30
    bne x8, t0, error
    
    # Test with zero
    li x10, 42
    li x11, 0
    c.add x10, x11       # x10 = 42 + 0 = 42
    
    # Verify result
    li t0, 42
    bne x10, t0, error
    
    # Test negative addition
    li x12, -5
    li x13, 3
    c.add x12, x13       # x12 = -5 + 3 = -2
    
    # Verify result
    li t0, -2
    bne x12, t0, error
    
    # Success
    li a0, 0
    li a7, 93
    ecall

error:
    li a0, 1
    li a7, 93
    ecall

1:  j 1b