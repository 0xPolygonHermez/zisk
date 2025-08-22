# Test c.j instruction - compressed unconditional jump

.section .text.init
.global _start

_start:
    # Test forward jump
    c.j forward_target
    
    # Should not reach here
    li a0, 1
    li a7, 93
    ecall

forward_target:
    # Test backward jump
    c.j test_backward
    
after_backward:
    # Test jump over instructions
    c.j skip_section
    
    # These should be skipped
    li x1, 0xbad
    li x2, 0xbad
    
skip_section:
    # Success - all jumps worked
    li a0, 0
    li a7, 93
    ecall

test_backward:
    # Jump back to continue test
    c.j after_backward

1:  j 1b