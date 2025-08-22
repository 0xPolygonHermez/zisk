# Test c.lw and c.sw instructions - compressed load/store word

.section .data
test_data:
    .word 0x12345678
    .word 0x9abcdef0
    .space 32

.section .text.init
.global _start

_start:
    # Load base address (must use compressed register)
    la x8, test_data
    
    # Test basic c.sw and c.lw
    li x9, 0x11223344
    c.sw x9, 8(x8)       # Store at offset 8
    c.lw x10, 8(x8)      # Load back from offset 8
    
    # Verify result
    bne x9, x10, error
    
    # Test different offsets
    li x11, 0x55667788
    c.sw x11, 12(x8)     # Store at offset 12
    c.lw x12, 12(x8)     # Load back
    
    # Verify result
    bne x11, x12, error
    
    # Test maximum offset (124 = 31 * 4)
    li x13, 0xaabbccdd
    c.sw x13, 124(x8)    # Store at max offset
    c.lw x14, 124(x8)    # Load back
    
    # Verify result
    bne x13, x14, error
    
    # Test loading existing data
    c.lw x15, 0(x8)      # Load first word (0x12345678)
    
    # Verify result
    li t0, 0x12345678
    bne x15, t0, error
    
    # Success
    li a0, 0
    li a7, 93
    ecall

error:
    li a0, 1
    li a7, 93
    ecall

1:  j 1b