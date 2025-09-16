# Test c.andi instruction - compressed AND immediate

.section .text.init
.global _start

_start:
    # Test basic AND with positive immediate
    li x8, 0xff
    c.andi x8, 0x0f      # x8 = 0xff & 0x0f = 0x0f
    
    # Verify result
    li t0, 0x0f
    bne x8, t0, error
    
    # Test AND with zero (clears all bits)
    li x9, 0x12345678
    c.andi x9, 0         # x9 = 0x12345678 & 0 = 0
    
    # Verify result
    bne x9, x0, error
    
    # Test AND with -1 (preserves all bits in range)
    li x10, 0x12345678
    c.andi x10, -1       # x10 = 0x12345678 & 0xffffffff = 0x12345678
    
    # Verify result (only low bits matter for c.andi)
    li t0, 0x12345678
    bne x10, t0, error
    
    # Test boundary immediate values
    li x11, 0xffffffff
    c.andi x11, 31       # x11 = 0xffffffff & 31 = 31
    
    # Verify result
    li t0, 31
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