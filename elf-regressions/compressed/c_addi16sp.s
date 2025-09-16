# Test c.addi16sp instruction - compressed add immediate to SP scaled by 16
# Tests stack pointer manipulation with 16-byte alignment

.section .text.init
.global _start

_start:
    # Save original stack pointer
    mv t0, sp
    
    # Test positive adjustments (allocate stack space)
    c.addi16sp sp, -16    # Allocate 16 bytes
    c.addi16sp sp, -32    # Allocate 32 bytes
    c.addi16sp sp, -64    # Allocate 64 bytes
    c.addi16sp sp, -512   # Maximum negative adjustment
    
    # Test positive adjustments (deallocate stack space)
    c.addi16sp sp, 16     # Deallocate 16 bytes
    c.addi16sp sp, 32     # Deallocate 32 bytes
    c.addi16sp sp, 64     # Deallocate 64 bytes
    c.addi16sp sp, 496    # Maximum positive adjustment
    
    # Test edge cases
    c.addi16sp sp, -496   # Near maximum negative
    c.addi16sp sp, 496    # Maximum positive
    
    # Test zero adjustment (should be no-op)
    mv t1, sp
    
    # Restore stack pointer
    mv sp, t0
    
    # Exit
    li a7, 93
    ecall
    
1:  j 1b