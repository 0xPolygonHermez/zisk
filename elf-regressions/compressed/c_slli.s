# Test c.slli instruction - compressed shift left logical immediate

.section .text.init
.global _start

_start:
    # Test basic left shift
    li x1, 1
    c.slli x1, 1         # x1 = 1 << 1 = 2
    
    # Verify result
    li t0, 2
    bne x1, t0, error
    
    # Test larger shift
    li x2, 1
    c.slli x2, 8         # x2 = 1 << 8 = 256
    
    # Verify result
    li t0, 256
    bne x2, t0, error
    
    # Test shift with data
    li x3, 0x12345678
    c.slli x3, 4         # x3 = 0x12345678 << 4 = 0x23456780
    
    # Verify result
    li t0, 0x23456780
    bne x3, t0, error
    
    # Test maximum shift
    li x4, 1
    c.slli x4, 31        # x4 = 1 << 31 = 0x80000000
    
    # Verify result
    li t0, 0x80000000
    bne x4, t0, error
    
    # Success
    li a0, 0
    li a7, 93
    ecall

error:
    li a0, 1
    li a7, 93
    ecall

1:  j 1b