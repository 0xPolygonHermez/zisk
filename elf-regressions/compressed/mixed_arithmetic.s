# Mixed compressed arithmetic operations test
# Combines various compressed arithmetic and logical instructions

.section .text.init
.global _start

_start:
    # Test arithmetic expression: ((a + b) - c) * d
    li x8, 10            # a
    li x9, 5             # b  
    li x10, 3            # c
    li x11, 2            # d
    
    c.add x8, x9         # x8 = a + b = 15
    c.sub x8, x10        # x8 = (a + b) - c = 12
    # Note: No c.mul, so we'll simulate with shifts/adds
    c.slli x8, 1         # x8 = x8 * 2 = 24
    
    # Test bit manipulation chain: (x | y) & (~z)
    li x12, 0xf0f0f0f0
    li x13, 0x0f0f0f0f
    li x14, 0x00ff00ff
    
    c.or x12, x13        # x12 = 0xf0f0f0f0 | 0x0f0f0f0f = 0xffffffff
    c.xor x14, x14       # x14 = x14 ^ x14 = 0 (NOT simulation)
    c.addi x14, -1       # x14 = 0 - 1 = 0xffffffff (all 1s)
    c.xor x14, x15       # Assume x15 has the value we want to invert
    
    # Test stack operations simulation
    c.addi16sp sp, -32   # Allocate stack space
    
    # Store some values (use regular instructions since c.swsp/c.lwsp not supported)
    li x15, 0x12345678
    sw x15, 0(sp)        # Store to stack
    sw x8, 4(sp)         # Store our calculated result
    
    # Modify and reload
    lw x16, 0(sp)        # Load back
    addi x16, x16, 1     # Increment
    sw x16, 8(sp)        # Store modified value
    
    # Test register move chain with arithmetic
    li x17, 100
    c.mv x8, x17         # x8 = 100 (use compressed register)
    c.addi x8, 31        # x8 = 131 (c.addi limited to -32 to 31)
    c.mv x9, x8          # x9 = 131
    c.sub x9, x8         # x9 = 131 - 131 = 0 (c.sub only on compressed regs)
    
    # Test shift patterns (shift ops only work on compressed registers for c.srli/c.srai)
    li x10, 0x11111111   # Use compressed register x10
    c.slli x10, 1        # x10 = 0x22222222
    c.srli x10, 2        # x10 = 0x08888888 (c.srli only on x8-x15)
    c.srai x10, 1        # x10 = 0x04444444 (c.srai only on x8-x15)
    
    # Test immediate operations chain (use compressed registers)
    li x11, 0
    c.li x11, 15         # Load immediate
    c.addi x11, 5        # x11 = 20 (c.addi only on compressed regs)
    c.andi x11, 0x1f     # x11 = 20 & 31 = 20 (c.andi only on x8-x15)
    c.srli x11, 2        # x11 = 20 >> 2 = 5 (c.srli only on x8-x15)
    
    # Test upper immediate with calculations (fix c.lui immediate range)
    c.lui x12, 16        # x12 = 16 << 12 (c.lui uses 6-bit signed immediate)
    c.srli x12, 4        # x12 shifted right (c.srli only on x8-x15)
    c.slli x12, 2        # x12 shifted left
    
    # Test compressed register operations mixing (all on x8-x15)
    li x8, 0xaaaaaaaa
    li x9, 0x55555555
    li x10, 0xff00ff00
    li x11, 0x00ff00ff
    
    c.and x8, x9         # x8 = 0 (no common bits)
    c.or x10, x11        # x10 = 0xffffffff (all bits)
    c.xor x8, x10        # x8 = 0 ^ 0xffffffff = 0xffffffff
    c.sub x10, x8        # x10 = 0xffffffff - 0xffffffff = 0
    
    # Test address calculation pattern
    la x13, test_data    # Use compressed register
    c.addi x13, 8        # Point to offset 8 (c.addi only on x8-x15)
    c.lw x14, 0(x13)     # Load from calculated address (c.lw only on x8-x15)
    
    # Test loop counter pattern (use compressed registers)
    li x15, 5            # Counter (compressed register)
    li x8, 0             # Sum (compressed register)
    
loop:
    c.beqz x15, loop_end # c.beqz only on x8-x15
    c.add x8, x15        # Add counter to sum (c.add with compressed regs)
    c.addi x15, -1       # Decrement (c.addi only on x8-x15)
    c.j loop
    
loop_end:
    # x8 should be 5+4+3+2+1 = 15
    
    # Test function call pattern
    la x9, test_function # Use compressed register
    c.jalr x9            # Call function
    
    # Should return here with x10 modified
    
    # Restore stack
    c.addi16sp sp, 32
    
    # Test final calculations
    c.add x8, x10        # Combine results (compressed registers)
    c.andi x8, 0x1f      # Mask result (c.andi only on x8-x15)
    
    # Verification
    li t0, 15
    bne x8, t0, error    # Check if sum is correct (before masking)
    
    # Success
    li a0, 0
    li a7, 93
    ecall
    
test_function:
    # Simple function that modifies x10
    c.li x10, 10         # Use compressed register
    c.jr x1              # Return
    
error:
    li a0, 1
    li a7, 93
    ecall
    
1:  j 1b

.section .data
test_data:
    .word 0x11111111
    .word 0x22222222
    .word 0x33333333
    .word 0x44444444
