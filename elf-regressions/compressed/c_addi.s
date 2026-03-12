# Test c.addi instruction - compressed add immediate
# Tests adding immediate values to registers

.section .text.init
.global _start

_start:
    # Initialize base values
    li x1, 100
    li x2, 0
    li x3, -50
    
    # Test positive immediate additions
    c.addi x1, 1
    c.addi x1, 31
    c.addi x1, 15
    
    # Test negative immediate additions
    c.addi x1, -1
    c.addi x1, -32
    c.addi x1, -16
    
    # Test zero addition
    c.addi x2, 0
    
    # Test overflow scenarios
    li x4, 0x7fffffff
    c.addi x4, 1      # Should overflow to negative
    
    li x5, 0x80000000
    c.addi x5, -1     # Should underflow to positive
    
    # Test different registers
    c.addi t0, 10
    c.addi t1, -5
    c.addi a0, 7
    c.addi a1, -3
    
    # Exit
    li a7, 93
    ecall
    
1:  j 1b