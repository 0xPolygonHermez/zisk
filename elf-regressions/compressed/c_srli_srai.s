# Test c.srli and c.srai instructions - compressed shift right logical and arithmetic

.section .text.init
.global _start

_start:
    # Test c.srli (logical right shift)
    li x8, 0x80000000
    c.srli x8, 1         # x8 = 0x80000000 >> 1 = 0x40000000 (logical)
    
    # Verify result
    li t0, 0x40000000
    bne x8, t0, error
    
    # Test c.srli with data
    li x9, 0x12345678
    c.srli x9, 4         # x9 = 0x12345678 >> 4 = 0x01234567
    
    # Verify result
    li t0, 0x01234567
    bne x9, t0, error
    
    # Test c.srai (arithmetic right shift) with positive number
    li x10, 0x12345678
    c.srai x10, 4        # x10 = 0x12345678 >> 4 = 0x01234567 (same as logical)
    
    # Verify result
    li t0, 0x01234567
    bne x10, t0, error
    
    # Test c.srai with negative number (sign extension)
    li x11, 0x80000000
    c.srai x11, 1        # x11 = 0x80000000 >> 1 = 0xc0000000 (sign extended)
    
    # Verify result
    li t0, 0xc0000000
    bne x11, t0, error
    
    # Success
    li a0, 0
    li a7, 93
    ecall

error:
    li a0, 1
    li a7, 93
    ecall

1:  j 1b