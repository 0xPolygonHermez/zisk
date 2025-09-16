# Test c.jr and c.jalr instructions - compressed jump register

.section .text.init
.global _start

_start:
    # Test c.jr (jump register)
    la x1, jr_target
    c.jr x1              # Jump to address in x1
    
    # Should not reach here
    li a0, 1
    li a7, 93
    ecall

jr_target:
    # Test c.jalr (jump and link register)
    la x2, function
    c.jalr x2            # Call function, return address in x1
    
    # Should return here
    # Test that we returned by calling another function
    la x3, function2
    c.jalr x3
    
    # Success
    li a0, 0
    li a7, 93
    ecall

function:
    # Simple function that returns
    c.jr x1              # Return using saved address

function2:
    # Another simple function
    c.jr x1              # Return

1:  j 1b