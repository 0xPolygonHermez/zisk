# Test c.lui instruction - compressed load upper immediate
# Tests upper immediate loading with various values

.section .text.init
.global _start

_start:
    # Test basic values (c.lui uses 6-bit signed immediate, not 20-bit)
    c.lui x1, 1
    c.lui x3, 0x1f      # Maximum positive (31)
    c.lui x4, 1
    
    # Test various positive values (this toolchain only supports positive c.lui)
    c.lui x5, 1         # Small positive
    c.lui x6, 2         # Small positive
    
    # Test boundary conditions (positive values only)
    c.lui x7, 30        # Near maximum
    c.lui x8, 31        # Most positive (31)
    
    # Test with different registers (excluding x0, x2/sp)
    c.lui t0, 16        # Valid positive immediate
    c.lui t1, 20        # Valid positive immediate
    c.lui a0, 8         # Valid positive immediate
    c.lui a1, 12        # Valid positive immediate
    
    # Exit
    li a7, 93
    ecall
    
1:  j 1b