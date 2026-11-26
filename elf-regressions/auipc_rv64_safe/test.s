# auipc_rv64_safe: auipc rd, 0x57fff at PC=0xA8000000 → rd=0xFFFFF000 (fits 32 bits, high limb 0)
# Compare: auipc_rv64_overflow/ — same instruction 4 KB later, result crosses 32 bits.
# The result goes to a1 and the program exits with code 0.

.section .text.init
.global _start
_start:
    auipc a1, 0x57fff   # 0xA8000000 + 0x57FFF000 = 0xFFFFF000
    nop                 # keeps the auipc on its own: no fusion with the next instruction
    mv a2, a1           # read the register back
    li a0, 0
    li a7, 93
    ecall
1:  j 1b
