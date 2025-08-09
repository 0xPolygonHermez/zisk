# Test case for remainder operation bug
# This test exposes the "undefined reference to 'pc_80000008_rem_check_underflow'" error
# when running rom-setup with a remainder instruction.
# 
# The issue is that the remainder operation implementation hasn't defined the label,
# so there is nothing to reference.

.section .text.init
.global _start

_start:
    # Simple remainder operation
    li a0, 100      # dividend
    li a1, 7        # divisor
    rem a2, a0, a1  # remainder = 100 % 7 = 2
    
    # Exit
    li a7, 93
    ecall
    
1:  j 1b