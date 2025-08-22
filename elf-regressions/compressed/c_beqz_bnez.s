# Test c.beqz and c.bnez instructions - compressed branch if equal/not equal to zero

.section .text.init
.global _start

_start:
    # Test c.beqz with zero - should branch
    li x8, 0
    c.beqz x8, test1_pass
    j error

test1_pass:
    # Test c.beqz with non-zero - should not branch
    li x9, 1
    c.beqz x9, error
    
    # Test c.bnez with non-zero - should branch
    li x10, 42
    c.bnez x10, test2_pass
    j error

test2_pass:
    # Test c.bnez with zero - should not branch
    li x11, 0
    c.bnez x11, error
    
    # All tests passed
    li a0, 0
    li a7, 93
    ecall

error:
    li a0, 1
    li a7, 93
    ecall

1:  j 1b