# Control: reading read-only CSRs is legal (csrr, and csrrs / csrrsi / csrrci that
# set or clear no bits), and exit(0) ends the execution successfully.

.section .text.init
.global _start

_start:
    csrr t0, marchid
    csrrs t1, mvendorid, x0
    csrrsi t2, mimpid, 0
    csrrci t3, mhartid, 0
    li a0, 0
    li a7, 93
    ecall
loop:
    j loop
