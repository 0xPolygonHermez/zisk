# Edge cases for compressed instruction alignment and encoding
# Tests instruction alignment, mixed 16/32-bit instructions, and boundary conditions

.section .text.init
.global _start

_start:
    # Test 16-bit instruction alignment
    # Compressed instructions must be 16-bit aligned
    
    # Mix compressed and uncompressed instructions
    c.li x1, 10          # 16-bit instruction
    addi x2, x1, 20      # 32-bit instruction
    c.add x1, x2         # 16-bit instruction
    li x3, 0x12345678    # 32-bit instruction
    c.mv x4, x3          # 16-bit instruction
    
    # Test instruction sequence that crosses boundaries
    .align 2
boundary_test:
    c.li x5, 1           # At 4-byte boundary
    c.li x6, 2           # At 4-byte boundary + 2
    add x7, x5, x6       # 32-bit instruction at odd 16-bit boundary
    c.mv x8, x7          # Back to compressed
    
    # Test maximum compressed immediate values at boundaries
    c.li x9, 31          # Maximum 6-bit signed positive
    c.li x10, -32        # Maximum 6-bit signed negative
    addi x11, x9, 1      # Force to 32-bit to get 32
    addi x12, x10, -1    # Force to 32-bit to get -33
    
    # Compare compressed vs uncompressed versions
    bne x9, x11, different_immediate_ranges  # Should be different (31 vs 32)
    c.j error            # Should not reach here
    
different_immediate_ranges:
    # Test shift amount boundaries
    li x13, 1
    c.slli x13, 31       # Maximum shift amount for compressed
    li x14, 1
    slli x14, x14, 31    # Same shift with 32-bit instruction
    bne x13, x14, error  # Should be the same
    
    # Test compressed vs uncompressed load/store
    la x15, test_data
    li x8, 0xabcdef01    # Use compressed register
    
    # Compressed store and load (both registers must be x8-x15)
    c.sw x8, 0(x15)      # Compressed store (both x8,x15 are compressed)
    c.lw x9, 0(x15)      # Compressed load (both x9,x15 are compressed)
    
    # Uncompressed store and load with same data
    sw x8, 4(x15)        # Uncompressed store (larger offset range)
    lw x18, 4(x15)       # Uncompressed load
    
    bne x9, x18, error   # Should load same data
    
    # Test offset encoding differences
    # Compressed: offset scaled by 4, max 124 (31*4)
    # Uncompressed: offset not scaled, max 2047
    
    c.sw x8, 124(x15)    # Maximum compressed offset (x8 compressed)
    sw x8, 128(x15)      # Uncompressed beyond compressed range
    
    c.lw x10, 124(x15)   # Load with compressed instruction (x10 compressed)
    lw x20, 128(x15)     # Load with uncompressed instruction
    
    bne x10, x20, error  # Should be same data
    
    # Test stack pointer operations with alignment
    mv t6, sp
    
    # Ensure SP is aligned before compressed operations
    andi t0, sp, 15      # Check if SP is 16-byte aligned
    bnez t0, align_sp    # If not, align it
    c.j sp_aligned
    
align_sp:
    addi sp, sp, -16     # Align to 16-byte boundary
    
sp_aligned:
    # Test compressed stack operations
    c.addi16sp sp, -32   # Must be multiple of 16
    c.addi16sp sp, 32    # Restore
    
    # Test that odd adjustments fail (should use uncompressed)
    # c.addi16sp sp, -17   # This would be invalid encoding
    addi sp, sp, -17     # Use uncompressed for odd values
    addi sp, sp, 17      # Restore with uncompressed
    
    mv sp, t6            # Restore original SP
    
    # Test instruction fetch alignment
    # Jump to odd 16-bit boundary
    c.j odd_boundary_test
    
    .align 2
    nop                  # Ensure we start at 4-byte boundary
    
odd_boundary_test:
    # These instructions start at 4-byte + 2 boundary
    li x21, 42           # Use regular li (c.li limited to -32 to 31)
    li x22, 84           # Use regular li (c.li limited to -32 to 31)
    add x23, x21, x22    # 32-bit instruction at 4-byte boundary
    
    # Test branch targets and alignment
    c.j even_target      # Jump to even boundary
    
    .align 2
    nop                  # Padding to create even target
    
even_target:
    li x24, 100          # At 4-byte boundary (c.li limited to -32 to 31)
    
    # Test that unaligned accesses work properly
    la x11, unaligned_data   # Use compressed register
    
    # Load from properly aligned address
    c.lw x12, 0(x11)     # Aligned load (both x12,x11 compressed)
    
    # Test memory alignment requirements
    la x27, test_data
    
    # Ensure test_data is word-aligned for compressed operations
    andi t0, x27, 3      # Check word alignment
    bnez t0, error       # Should be word-aligned
    
    # Test compressed instruction encoding boundaries
    # Some instructions have special encodings for certain values
    
    # c.li x0, 0 is reserved (HINT)
    # c.addi x0, 0 is NOP
    # c.mv x0, x0 is reserved
    
    # Test legal NOP variants
    nop                  # Use standard nop instead
    addi x0, x0, 0       # 32-bit NOP
    
    # Test register encoding boundaries
    # Compressed registers use 3-bit encoding (x8-x15)
    # Full registers use 5-bit encoding (x0-x31)
    
    # Test that high registers require uncompressed instructions
    li x30, 0x30303030   # High register number
    li x31, 0x31313131   # Highest register
    
    # These operations require uncompressed instructions
    add x30, x30, x31    # Cannot use compressed form
    mv x29, x30          # c.mv can handle any register
    
    # Test instruction length detection
    # Ensure emulator correctly identifies 16-bit vs 32-bit instructions
    
    # Create specific pattern to test decoder
    c.li x28, 15         # 16-bit: should decode as c.li
    ori x28, x28, 16     # 32-bit: should decode as ori (not compressed)
    
    # Result should be 15 | 16 = 31
    li t0, 31
    bne x28, t0, error
    
    # Test boundary between compressed and uncompressed encodings
    # Some instruction patterns overlap between 16-bit and 32-bit encodings
    
    # Verify all our test values
    li t0, 10
    bne x1, t0, error
    li t0, 30
    bne x2, t0, error
    li t0, 40
    bne x4, t0, error    # x4 = x1 + x2
    
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
    .word 0x11111111
    .word 0x22222222
    .word 0x33333333
    .word 0x44444444
    .space 64

# Intentionally unaligned data for testing
.align 1
unaligned_data:
    .byte 0x01
    .word 0x12345678     # This will be at odd byte boundary