# Tests that JALR works when code doesn't start at 0x80000000
#
# This is mainly testing `jumpt_to_dynamic_pc`; it currently assumes that
# user code starts from 0x800000000.

.section .text.init
.global _start

_start:    
    # Simple JALR test
    la t0, target       # Load address of target
    jalr ra, t0, 0      # Jump and link to target
    
    # If we get here, JALR worked
    li a0, 42           # Success value
    
    # Exit
    li a7, 93
    ecall

target:
    li a1, 100
    ret                 # This is also a JALR