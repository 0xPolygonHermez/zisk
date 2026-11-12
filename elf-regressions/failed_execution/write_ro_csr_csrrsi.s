# The immediate form fails the same way: csrrsi with a nonzero uimm sets bits of a
# read-only CSR (mvendorid, 0xF11), an illegal instruction. The exit below must
# never be reached.

.section .text.init
.global _start

_start:
    csrrsi x0, mvendorid, 1
    li a0, 0
    li a7, 93
    ecall
loop:
    j loop
