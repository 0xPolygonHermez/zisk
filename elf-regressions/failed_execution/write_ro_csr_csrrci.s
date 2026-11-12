# csrrci with a nonzero uimm clears bits of a read-only CSR (marchid, 0xF12), an
# illegal instruction too. The exit below must never be reached.

.section .text.init
.global _start

_start:
    csrrci x0, marchid, 1
    li a0, 0
    li a7, 93
    ecall
loop:
    j loop
