# Test JALR with gap/padding before actual code
# Program section starts at 0x80001000 but first real instruction is later
# This tests that min_program_pc is correctly found even with padding

.section .text
.global _start

# Start with padding at 0x80001000
.align 4
    nop                 # 0x80001000: padding
    nop                 # 0x80001004: padding
    nop                 # 0x80001008: padding
    j _start            # 0x8000100c: jump to actual start

# Actual code starts here at 0x80001010
_start:
    # Simple JALR test
    la t0, target       # Load address of target
    jalr ra, t0, 0      # Jump and link to target
    
    # If we get here, JALR worked
    li a0, 42
    
    # Exit
    li a7, 93
    ecall

target:
    li a1, 100          # Set a1 to verify we got here
    ret                 # Return using JALR