# Test case for RemuW (32-bit unsigned remainder) formatting bug
# Essentially there is a missing '\n' in the assembly file for RemuW operation.
#
# This causes malformed assembly output when using remuw instruction.

.section .text.init
.global _start

_start:
    # Test 32-bit unsigned remainder operation (remuw)
    li a0, 100          # dividend (32-bit value)
    li a1, 7            # divisor (32-bit value)
    remuw a2, a0, a1    # 32-bit unsigned remainder: 100 % 7 = 2
    
    # Exit
    li a7, 93
    ecall
    
1:  j 1b