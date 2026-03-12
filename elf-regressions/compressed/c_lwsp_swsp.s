# Test compressed stack operations - using c.addi16sp and regular load/store
# Tests SP-relative operations with compressed instructions where available

.section .text.init
.global _start

_start:
    # Save original stack pointer
    mv t0, sp
    
    # Allocate stack space using compressed instruction
    c.addi16sp sp, -256   # Use compressed SP adjustment
    
    # Initialize some test data on stack
    li t1, 0x11111111
    li t2, 0x22222222
    li t3, 0x33333333
    li t4, 0x44444444
    li t5, 0x55555555
    li t6, 0x66666666
    
    # Store test data using regular stack operations
    sw t1, 0(sp)          # Store at SP + 0
    sw t2, 4(sp)          # Store at SP + 4  
    sw t3, 8(sp)          # Store at SP + 8
    sw t4, 12(sp)         # Store at SP + 12
    sw t5, 16(sp)         # Store at SP + 16
    sw t6, 20(sp)         # Store at SP + 20
    
    # Test larger offsets
    sw t1, 64(sp)         # Store at SP + 64
    sw t2, 128(sp)        # Store at SP + 128
    sw t3, 192(sp)        # Store at SP + 192
    sw t4, 252(sp)        # Store at SP + 252
    
    # Clear registers
    li x1, 0
    li x2, 0
    li x3, 0
    li x4, 0
    li x5, 0
    li x6, 0
    
    # Load back using regular loads
    lw x1, 0(sp)          # Load from SP + 0
    lw x2, 4(sp)          # Load from SP + 4
    lw x3, 8(sp)          # Load from SP + 8
    lw x4, 12(sp)         # Load from SP + 12
    lw x5, 16(sp)         # Load from SP + 16
    lw x6, 20(sp)         # Load from SP + 20
    
    # Verify values match original
    bne x1, t1, error
    bne x2, t2, error
    bne x3, t3, error
    bne x4, t4, error
    bne x5, t5, error
    bne x6, t6, error
    
    # Test larger offsets
    lw x7, 64(sp)         # Load from SP + 64
    lw x8, 128(sp)        # Load from SP + 128
    lw x9, 192(sp)        # Load from SP + 192
    lw x10, 252(sp)       # Load from SP + 252
    
    # Verify larger offset values
    bne x7, t1, error
    bne x8, t2, error
    bne x9, t3, error
    bne x10, t4, error
    
    # Test stack frame simulation
    sw ra, 248(sp)        # Save return address
    sw s0, 244(sp)        # Save frame pointer
    lw s0, 244(sp)        # Restore frame pointer
    lw ra, 248(sp)        # Restore return address
    
success:
    # Restore stack pointer
    mv sp, t0
    
    # Exit with success
    li a0, 0
    li a7, 93
    ecall
    
error:
    # Restore stack pointer
    mv sp, t0
    
    # Exit with error
    li a0, 1
    li a7, 93
    ecall
    
1:  j 1b