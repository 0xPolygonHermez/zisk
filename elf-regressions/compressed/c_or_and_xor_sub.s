# Test c.or, c.and, c.xor, c.sub instructions - compressed ALU operations

.section .text.init
.global _start

_start:
    # Test c.or
    li x8, 0xf0f0f0f0
    li x9, 0x0f0f0f0f
    c.or x8, x9          # x8 = 0xf0f0f0f0 | 0x0f0f0f0f = 0xffffffff
    
    # Verify result
    li t0, 0xffffffff
    bne x8, t0, error
    
    # Test c.and
    li x10, 0x12345678
    li x11, 0xff00ff00
    c.and x10, x11       # x10 = 0x12345678 & 0xff00ff00 = 0x12005600
    
    # Verify result
    li t0, 0x12005600
    bne x10, t0, error
    
    # Test c.xor
    li x12, 0xaaaaaaaa
    li x13, 0x55555555
    c.xor x12, x13       # x12 = 0xaaaaaaaa ^ 0x55555555 = 0xffffffff
    
    # Verify result
    li t0, 0xffffffff
    bne x12, t0, error
    
    # Test c.sub
    li x14, 100
    li x15, 30
    c.sub x14, x15       # x14 = 100 - 30 = 70
    
    # Verify result
    li t0, 70
    bne x14, t0, error
    
    # Success
    li a0, 0
    li a7, 93
    ecall

error:
    li a0, 1
    li a7, 93
    ecall

1:  j 1b