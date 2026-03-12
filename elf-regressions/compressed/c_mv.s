# Test c.mv instruction - compressed move register to register

.section .text.init
.global _start

_start:
    # Test basic move
    li x1, 0x12345678
    c.mv x2, x1          # x2 = x1
    
    # Verify result
    bne x1, x2, error
    
    # Test move zero
    li x3, 0
    c.mv x4, x3          # x4 = 0
    
    # Verify result
    bne x3, x4, error
    
    # Test move negative
    li x5, -1
    c.mv x6, x5          # x6 = -1
    
    # Verify result
    bne x5, x6, error
    
    # Test chain of moves
    li x7, 0xdeadbeef
    c.mv x8, x7          # x8 = x7
    c.mv x9, x8          # x9 = x8 = x7
    
    # Verify chain
    bne x7, x8, error
    bne x8, x9, error
    
    # Success
    li a0, 0
    li a7, 93
    ecall

error:
    li a0, 1
    li a7, 93
    ecall

1:  j 1b