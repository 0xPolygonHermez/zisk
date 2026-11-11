# auipc_rv64_safe: auipc rd, 0x57fff at PC=0xA8000000 → rd=0xFFFFF000 (fits 32 bits, proving OK)
# Compare: auipc_rv64_overflow/ — same instruction 4 KB later, result overflows 32 bits.

.section .text.init
.global _start
_start:
    auipc a0, 0x57fff   # 0xA8000000 + 0x57FFF000 = 0xFFFFF000
    li a7, 93
    ecall
1:  j 1b
