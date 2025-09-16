# Edge cases for compressed branch instructions
# Tests branch distances, target alignment, and conditional edge cases

.section .text.init
.global _start

_start:
    # Test compressed branch distance limits
    # c.beqz and c.bnez have 9-bit signed offset (scaled by 2)
    # Range: -256 to +254 bytes
    
    # Test forward branch near maximum distance
    li x8, 0
    c.beqz x8, far_forward_target    # Should branch
    
    # Fill space to test distance (but not too much to exceed limits)
    .rept 50
    nop
    .endr
    
    # Should not reach here
    li x1, 0xbad
    c.j error
    
far_forward_target:
    li x1, 0x100         # Mark successful far forward branch
    
    # Test backward branch setup
    li x9, 42
    c.j setup_backward_test
    
backward_target:
    li x2, 0x200         # Mark successful backward branch
    c.j continue_test
    
setup_backward_test:
    # Test backward branch
    c.bnez x9, backward_target       # Should branch back
    
    # Should not reach here
    li x2, 0xbad
    
continue_test:
    # Test branch target alignment
    # Branch targets should be 2-byte aligned for compressed instructions
    
    li x10, 1
    c.j aligned_target   # Jump to aligned target
    
    .align 2
    nop                  # Ensure alignment
    
aligned_target:
    c.bnez x10, next_aligned         # Branch to another aligned target
    li x3, 0xbad
    
    .align 1             # 2-byte alignment
next_aligned:
    li x3, 0x300
    
    # Test edge case: branch to current instruction (infinite loop prevention)
    li x11, 0
    c.bnez x11, after_self_branch    # Should not branch (x11 is 0)
    li x4, 0x400         # Should execute this
    c.j after_self_branch
    
after_self_branch:
    # Test branch with zero flag edge cases
    
    # Test with exactly zero
    li x12, 0x00000000
    c.beqz x12, zero_branch_taken
    li x5, 0xbad
    c.j error
    
zero_branch_taken:
    li x5, 0x500
    
    # Test with non-zero patterns
    li x13, 0x80000000   # MSB set (negative in signed interpretation)
    c.bnez x13, negative_nonzero
    li x6, 0xbad
    c.j error
    
negative_nonzero:
    li x6, 0x600
    
    li x14, 0x00000001   # LSB set (smallest positive)
    c.bnez x14, positive_nonzero
    li x7, 0xbad
    c.j error
    
positive_nonzero:
    li x7, 0x700
    
    # Test with alternating bit patterns
    li x15, 0xaaaaaaaa
    c.bnez x15, alternating_nonzero
    li x8, 0xbad
    c.j error
    
alternating_nonzero:
    li x8, 0x800
    
    # Test branch prediction scenarios
    # Loops that are likely to be taken vs not taken
    
    # Countdown loop (usually taken, except last iteration)
    li x8, 3             # Loop counter (compressed register)
    li x9, 0             # Accumulator (compressed register)
    
countdown_loop:
    c.beqz x8, countdown_end     # Exit condition (c.beqz only on x8-x15)
    c.add x9, x8         # Add counter to accumulator (compressed regs)
    c.addi x8, -1        # Decrement (c.addi only on x8-x15)
    c.j countdown_loop   # Continue loop
    
countdown_end:
    # x9 should be 3+2+1 = 6
    li t0, 6
    bne x9, t0, error
    
    # Test nested conditional branches (use compressed registers)
    li x10, 1            # Use compressed register
    li x11, 0            # Use compressed register
    li x12, 2            # Use compressed register
    
    c.bnez x10, outer_condition    # Should branch (x10 is compressed)
    li x9, 0xbad
    c.j error
    
outer_condition:
    c.beqz x11, inner_condition    # Should branch (x11 is compressed)
    li x10, 0xbad
    c.j error
    
inner_condition:
    c.bnez x12, nested_end         # Should branch (x12 is compressed)
    li x11, 0xbad
    c.j error
    
nested_end:
    li x9, 0x900
    li x10, 0xa00
    li x11, 0xb00
    
    # Test branch over different instruction types
    li x13, 5            # Use compressed register
    c.bnez x13, skip_mixed_instructions
    
    # Mixed instructions to skip
    addi x12, x0, 1      # Valid instruction (0xbad is invalid hex)
    li x13, 1            # Use regular li (c.li limited to -32 to 31)
    lw x14, 0(x0)        # This might fault, but should be skipped
    c.add x15, x14
    
skip_mixed_instructions:
    li x12, 0xc00        # Mark successful skip
    
    # Test branch to function-like targets
    li x14, 10           # Use compressed register
    c.bnez x14, function_like_target
    li x13, 0xbad
    c.j error
    
return_point:
    li x13, 0xd00
    c.j final_verification
    
function_like_target:
    # Simulate function that "returns"
    li x14, 0xe00
    c.j return_point     # "Return" to caller
    
final_verification:
    # Test rapid successive branches (use compressed registers)
    li x8, 1             # Use compressed register
    li x9, 0             # Use compressed register
    li x10, 1            # Use compressed register
    
    c.bnez x8, rapid1    # Should branch (x8 is compressed)
    c.j error
rapid1:
    c.beqz x9, rapid2    # Should branch (x9 is compressed)
    c.j error  
rapid2:
    c.bnez x10, rapid3   # Should branch (x10 is compressed)
    c.j error
rapid3:
    li x15, 0xf00
    
    # Test branch distance calculations
    # Verify that we can branch to nearby targets
    li x11, 1            # Use compressed register
    c.bnez x11, near_target_1
    c.j error
    
near_target_1:
    c.bnez x11, near_target_2
    c.j error
    
near_target_2: 
    c.bnez x11, near_target_3
    c.j error
    
near_target_3:
    li x16, 0x1600
    
    # Verify all results
    li t0, 0x100
    bne x1, t0, error
    li t0, 0x200
    bne x2, t0, error
    li t0, 0x300
    bne x3, t0, error
    li t0, 0x400
    bne x4, t0, error
    li t0, 0x500
    bne x5, t0, error
    li t0, 0x600
    bne x6, t0, error
    li t0, 0x700
    bne x7, t0, error
    li t0, 0x800
    bne x8, t0, error
    li t0, 0x900
    bne x9, t0, error
    li t0, 0xa00
    bne x10, t0, error
    li t0, 0xb00
    bne x11, t0, error
    li t0, 0xc00
    bne x12, t0, error
    li t0, 0xd00
    bne x13, t0, error
    li t0, 0xe00
    bne x14, t0, error
    li t0, 0xf00
    bne x15, t0, error
    li t0, 0x1600
    bne x16, t0, error
    
    # Success
    li a0, 0
    li a7, 93
    ecall

error:
    li a0, 1
    li a7, 93
    ecall
    
1:  j 1b