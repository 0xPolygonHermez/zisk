# Test c.addi4spn instruction - compressed add immediate scaled by 4 to SP for narrow registers
# Tests SP-relative addressing for compressed register set (x8-x15)

.section .text.init
.global _start

_start:
    # Test with different immediate values and compressed registers
    c.addi4spn x8, sp, 4      # Add 4 to SP, store in x8
    c.addi4spn x9, sp, 8      # Add 8 to SP, store in x9
    c.addi4spn x10, sp, 12    # Add 12 to SP, store in x10
    c.addi4spn x11, sp, 16    # Add 16 to SP, store in x11
    
    # Test larger offsets
    c.addi4spn x12, sp, 64    # Add 64 to SP
    c.addi4spn x13, sp, 128   # Add 128 to SP
    c.addi4spn x14, sp, 256   # Add 256 to SP
    c.addi4spn x15, sp, 512   # Add 512 to SP
    
    # Test maximum offset
    c.addi4spn x8, sp, 1020   # Maximum offset (1020 = 255 * 4)
    
    # Test various multiples of 4
    c.addi4spn x9, sp, 20
    c.addi4spn x10, sp, 36
    c.addi4spn x11, sp, 100
    c.addi4spn x12, sp, 200
    c.addi4spn x13, sp, 300
    c.addi4spn x14, sp, 400
    c.addi4spn x15, sp, 500
    
    sub t0, x9, x8            # Should be (20-1020) = -1000
    sub t1, x10, x9           # Should be (36-20) = 16
    
    # Exit
    li a7, 93
    ecall
    
1:  j 1b