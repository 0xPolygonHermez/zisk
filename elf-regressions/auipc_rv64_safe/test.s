# auipc_rv64_safe: auipc rd, 0x7ffff at PC=0x80000000 → rd=0xFFFFFF00 (fits 32 bits, proving OK)
# Compare: auipc_rv64_overflow/ — same instruction 4 KB later, result overflows 32 bits.

.section .text.init
.global _start
_start:
    auipc a0, 0x7ffff   # 0x80000000 + 0x7FFFF000 = 0xFFFFFF00
    li a7, 93
    ecall
1:  j 1b
