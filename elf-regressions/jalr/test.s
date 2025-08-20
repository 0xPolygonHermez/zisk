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