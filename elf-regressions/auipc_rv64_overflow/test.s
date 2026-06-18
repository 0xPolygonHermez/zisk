# auipc_rv64_overflow: auipc rd, 0x7ffff at PC=0x80001000 → rd=0x100000000 (= 2^32, proving FAILS)
# .balign 4096 places auipc_test at the next 4 KB boundary, as a linker does for a new section.
# See README.md for the circuit analysis.

.section .text.init
.global _start
_start:
    j auipc_test
.balign 4096            # → auipc_test at 0x80001000
auipc_test:
    auipc a0, 0x7ffff   # 0x80001000 + 0x7FFFF000 = 0x100000000 (overflows 32-bit Mem AIR constraint)
    li a7, 93
    ecall
1:  j 1b
