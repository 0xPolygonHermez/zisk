# Edge cases for compressed instruction register usage
# Tests register constraints, aliasing, and special register behaviors

.section .text.init
.global _start

_start:
    # Test compressed register set (x8-x15) vs full register set
    
    # Initialize all compressed registers
    li x8, 0x08080808
    li x9, 0x09090909
    li x10, 0x10101010
    li x11, 0x11111111
    li x12, 0x12121212
    li x13, 0x13131313
    li x14, 0x14141414
    li x15, 0x15151515
    
    # Test operations only available on compressed registers
    c.and x8, x9         # Both operands must be x8-x15
    c.or x10, x11
    c.xor x12, x13
    c.sub x14, x15
    
    # Verify results
    li t0, 0x08080808
    li t1, 0x09090909
    and t2, t0, t1
    bne x8, t2, error
    
    # Test c.andi only works on compressed registers
    li x8, 0xffffffff
    c.andi x8, 15        # x8 must be in x8-x15 range
    li t0, 15
    bne x8, t0, error
    
    # Test shift operations on compressed registers
    li x9, 0x12345678
    c.srli x9, 4         # x9 must be x8-x15
    li t0, 0x01234567
    bne x9, t0, error
    
    li x10, 0x80000000
    c.srai x10, 1        # x10 must be x8-x15  
    li t0, 0xc0000000
    bne x10, t0, error
    
    # Test c.lw/c.sw compressed register constraints
    la x11, test_data    # Base register must be x8-x15
    li x12, 0xaabbccdd   # Data register must be x8-x15
    
    c.sw x12, 0(x11)     # Both registers x8-x15
    c.lw x13, 0(x11)     # Both registers x8-x15
    bne x12, x13, error
    
    # Test register x0 special cases
    # c.li x0, imm is reserved (should not be used)
    # c.addi x0, imm is a hint (NOP-like)
    # c.mv x0, rs is reserved
    
    # Test that x0 always reads as zero even with operations that try to modify it
    mv x1, x0            # x1 should be 0
    bne x1, x0, error
    
    # Test stack pointer (x2/sp) special handling
    mv t5, sp            # Save original SP
    
    # c.addi16sp only works with SP
    c.addi16sp sp, -16   # Must use SP as target
    c.addi16sp sp, 16    # Restore
    
    # c.lwsp/c.swsp use SP as base
    li x1, 0x87654321
    sw x1, 0(sp)         # Uses SP as base
    lw x2, 0(sp)         # Uses SP as base
    bne x1, x2, error
    
    mv sp, t5            # Restore SP
    
    # Test c.addi4spn with compressed register targets
    c.addi4spn x8, sp, 4     # Target must be x8-x15
    c.addi4spn x15, sp, 8    # Test boundary register
    
    sub t0, x8, sp
    li t1, 4
    bne t0, t1, error
    
    sub t0, x15, sp
    li t1, 8  
    bne t0, t1, error
    
    # Test return address register (x1/ra) in c.jalr
    la x3, test_function
    c.jalr x3            # Should save return address in x1
    
    # Verify we returned (x4 should be set by function)
    li t0, 0xf00c        # Valid hex constant
    bne x4, t0, error
    
    # Test c.jr with various registers
    la x5, jump_target1
    c.jr x5              # Can use any register
    
    li x6, 0xbad         # Should not execute
    
jump_target1:
    la x31, jump_target2
    c.jr x31             # Test with high register number
    
    li x7, 0xbad
    
jump_target2:
    # Test register aliasing and name conflicts
    # Verify that compressed registers are same as regular registers
    li s0, 0x12345678    # s0 is x8
    mv t0, x8
    bne s0, t0, error
    
    li s1, 0x87654321    # s1 is x9  
    mv t0, x9
    bne s1, t0, error
    
    # Test operations between compressed and non-compressed registers
    li x16, 0x11111111   # Non-compressed register
    c.mv x8, x16         # Move from non-compressed to compressed allowed
    bne x8, x16, error
    
    c.mv x17, x8         # Move from compressed to non-compressed allowed
    bne x17, x8, error
    
    # Test c.add with register constraints
    li x1, 100           # Any register for c.add
    li x8, 200           # Compressed register
    c.add x1, x8         # First operand any reg, second any reg
    li t0, 300
    bne x1, t0, error
    
    # Test boundary registers
    li x7, 0x07070707    # Just before compressed range
    li x8, 0x08080808    # First compressed register
    li x15, 0x15151515   # Last compressed register  
    li x16, 0x16161616   # Just after compressed range
    
    # These should work (c.mv accepts any registers)
    c.mv x7, x8          # Move to non-compressed from compressed
    c.mv x16, x15        # Move to non-compressed from compressed
    c.mv x8, x7          # Move to compressed from non-compressed
    c.mv x15, x16        # Move to compressed from non-compressed
    
    # Verify moves worked
    li t0, 0x08080808
    bne x7, t0, error
    li t0, 0x15151515
    bne x16, t0, error
    li t0, 0x07070707
    bne x8, t0, error
    li t0, 0x16161616
    bne x15, t0, error
    
    # Test special register encodings
    # x0 should always be zero regardless of operations
    add x0, x1, x2       # Try to modify x0 (should be ignored)
    bne x0, zero, error  # x0 should still be zero
    
    # Test that SP operations preserve stack integrity
    mv t6, sp
    li t0, 0x1000
    sub t1, sp, t0       # Calculate new SP value
    
    c.addi16sp sp, -64   # Modify SP with compressed instruction
    c.addi16sp sp, 64    # Restore SP
    bne sp, t6, error    # Should be back to original
    
    # Success
    li a0, 0
    li a7, 93
    ecall

test_function:
    li x4, 0xf00c        # Mark function called
    c.jr x1              # Return using return address

error:
    li a0, 1
    li a7, 93
    ecall
    
1:  j 1b

.section .data
.align 4
test_data:
    .space 64