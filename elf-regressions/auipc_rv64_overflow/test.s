# auipc_rv64_overflow: auipc rd, 0x57fff at PC=0xA8001000 → rd=0x100000000 (= 2^32, high limb 1)
# .balign 4096 places auipc_test at the next 4 KB boundary, as a linker does for a new section.
# The result goes to a1 and the program exits with code 0, so it must be proven. See README.md.

.section .text.init
.global _start
_start:
    j auipc_test
.balign 4096            # → auipc_test at 0xA8001000
auipc_test:
    auipc a1, 0x57fff   # 0xA8001000 + 0x57FFF000 = 0x100000000 (crosses the 32-bit limb boundary)
    nop                 # keeps the auipc on its own: no fusion with the next instruction
    mv a2, a1           # read the register back
    li a0, 0
    li a7, 93
    ecall
1:  j 1b
