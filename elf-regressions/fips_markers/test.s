# Test: FIPS markers with entry point between them
# Simulates Go binaries where _start (the code) is between FIPS boundary markers

.section .text

# FIPS start marker (data, not code)
.global _fipsstart
_fipsstart:
    .word 0x46495053  # "FIPS"
    .word 0x53544152  # "STAR"

# Entry point is between the markers
.global _start
_start:
    li a0, 42
    li a1, 58
    add a2, a0, a1
    
    # Exit
    li a7, 93
    li a0, 0
    ecall

# FIPS end marker (data, not code)
.global _fipsend
_fipsend:
    .word 0x46495053  # "FIPS"
    .word 0x454E4400  # "END"