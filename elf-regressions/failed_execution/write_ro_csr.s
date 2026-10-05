# Setting bits of a read-only CSR (mvendorid, 0xF11) is an illegal instruction too,
# so the execution fails there. The exit below must never be reached.

.section .text.init
.global _start

_start:
    li t0, 1
    csrrs x0, mvendorid, t0
    li a0, 0
    li a7, 93
    ecall
loop:
    j loop
