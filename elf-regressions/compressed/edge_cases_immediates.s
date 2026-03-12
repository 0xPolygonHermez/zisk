# Edge cases for compressed instruction immediates
# Tests boundary values, overflow conditions, and special immediate encodings

.section .text.init
.global _start

_start:
    # c.li edge cases - 6-bit signed immediate (-32 to 31)
    c.li x1, 31          # Maximum positive immediate
    c.li x2, -32         # Maximum negative immediate  
    c.li x3, 0           # Zero immediate
    c.li x4, 1           # Minimum positive
    c.li x5, -1          # Maximum magnitude negative
    
    # Verify sign extension
    li t0, 31
    bne x1, t0, error
    li t0, -32
    bne x2, t0, error
    li t0, -1
    bne x5, t0, error
    
    # c.addi edge cases - 6-bit signed immediate
    li x6, 100
    c.addi x6, 31        # Add maximum positive
    li t0, 131
    bne x6, t0, error
    
    li x7, 100  
    c.addi x7, -32       # Add maximum negative
    li t0, 68
    bne x7, t0, error
    
    # Test zero addition (should be no-op)
    li x8, 0x12345678
    c.addi x8, 0
    li t0, 0x12345678
    bne x8, t0, error
    
    # Test overflow with c.addi
    li x9, 0x7fffffff    # Max positive 32-bit
    c.addi x9, 1         # Should overflow to negative
    li t0, 0x80000000
    bne x9, t0, error
    
    li x10, 0x80000000   # Min negative 32-bit
    c.addi x10, -1       # Should underflow to positive  
    li t0, 0x7fffffff
    bne x10, t0, error
    
    # c.lui edge cases - 6-bit immediate for upper 20 bits
    # Note: c.lui encodes different than c.li/c.addi
    c.lui x11, 1         # Minimum non-zero value
    li t0, 0x1000        # Should be 1 << 12
    bne x11, t0, error
    
    c.lui x12, 31        # Maximum positive in 6-bit field
    li t0, 0x1f000       # Should be 31 << 12
    bne x12, t0, error
    
    # c.lui with different values
    c.lui x13, 16        # Another positive value
    # Test completed
    
    # c.addi16sp edge cases - 10-bit signed immediate scaled by 16
    mv t5, sp            # Save original SP
    
    # Test maximum positive adjustment
    c.addi16sp sp, 496   # 31 * 16 = 496 (max positive)
    
    # Test maximum negative adjustment  
    c.addi16sp sp, -512  # -32 * 16 = -512 (max negative)
    
    # Test small adjustments
    c.addi16sp sp, 16    # Minimum positive adjustment
    c.addi16sp sp, -16   # Minimum negative adjustment
    
    mv sp, t5            # Restore SP
    
    # c.addi4spn edge cases - 10-bit unsigned immediate scaled by 4
    # Maximum offset is 1020 (255 * 4)
    c.addi4spn x14, sp, 1020  # Maximum offset
    c.addi4spn x15, sp, 4     # Minimum offset
    
    # Verify the calculations
    sub t0, x14, sp
    li t1, 1020
    bne t0, t1, error
    
    sub t0, x15, sp  
    li t1, 4
    bne t0, t1, error
    
    # c.lwsp/c.swsp edge cases - 8-bit unsigned immediate scaled by 4
    mv t5, sp
    addi sp, sp, -256    # Allocate space
    
    # Test maximum offset using regular instructions
    li x16, 0xdeadbeef
    sw x16, 252(sp)      # Maximum offset
    lw x17, 252(sp)      # Load back
    bne x16, x17, error
    
    # Test minimum offset
    li x18, 0xcafebabe
    sw x18, 0(sp)        # Minimum offset
    lw x19, 0(sp)        # Load back
    bne x18, x19, error
    
    mv sp, t5            # Restore SP
    
    # c.lw/c.sw edge cases - 7-bit unsigned immediate scaled by 4
    la x12, test_data    # Use compressed register for base
    
    # Test maximum offset for compressed load/store (both registers must be x8-x15)
    li x8, 0x11223344    # Use compressed register
    c.sw x8, 124(x12)    # Maximum offset (31 * 4 = 124), both x8,x12 compressed
    c.lw x9, 124(x12)    # Load back, both x9,x12 compressed
    bne x8, x9, error
    
    # Test minimum offset
    li x10, 0x55667788   # Use compressed register
    c.sw x10, 0(x12)     # Minimum offset, both x10,x12 compressed
    c.lw x11, 0(x12)     # Load back, both x11,x12 compressed
    bne x10, x11, error
    
    # c.andi edge cases - 6-bit signed immediate (compressed registers only)
    li x12, 0xffffffff   # Use compressed register
    c.andi x12, 31       # AND with max positive immediate
    li t0, 31
    bne x12, t0, error
    
    li x13, 0xffffffff   # Use compressed register
    c.andi x13, -32      # AND with max negative immediate  
    li t0, 0xffffffe0    # Should be 0xffffffff & 0xffffffe0
    bne x13, t0, error
    
    li x14, 0x12345678   # Use compressed register
    c.andi x14, 0        # AND with zero (should clear all)
    bne x14, x0, error
    
    li x15, 0x12345678   # Use compressed register
    c.andi x15, -1       # AND with -1 (should preserve all)
    li t0, 0x12345678
    bne x15, t0, error
    
    # Shift immediate edge cases (use appropriate registers)
    li x8, 1             # Use compressed register for c.srli/c.srai
    c.slli x8, 31        # Maximum shift left (c.slli works on any register)
    li t0, 0x80000000
    bne x8, t0, error
    
    li x9, 0x80000000    # Use compressed register
    c.srli x9, 31        # Maximum logical shift right (c.srli only on x8-x15)
    li t0, 1
    bne x9, t0, error
    
    li x10, 0x80000000   # Use compressed register
    c.srai x10, 31       # Maximum arithmetic shift right (c.srai only on x8-x15)
    li t0, -1            # Should sign extend to all 1s
    bne x10, t0, error
    
    # Test shift by 1 (minimum non-zero shift)
    li x8, 0x12345678
    c.slli x8, 1
    li t0, 0x2468acf0
    bne x8, t0, error
    
    li x9, 0x12345678
    c.srli x9, 1  
    li t0, 0x091a2b3c
    bne x9, t0, error
    
    li x10, 0x92345678   # Negative number
    c.srai x10, 1
    li t0, 0xc91a2b3c    # Sign extended
    bne x10, t0, error
    
    # Test boundary between positive and negative immediates
    c.li x11, 15         # Positive
    c.li x12, 16         # Still positive (but check encoding)
    c.li x13, -16        # Negative  
    c.li x14, -15        # Less negative
    
    # Verify correct values
    li t0, 15
    bne x11, t0, error
    li t0, 16
    bne x12, t0, error
    li t0, -16
    bne x13, t0, error
    li t0, -15
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

.section .data
.align 4
test_data:
    .space 256           # Space for testing various offsets