# A trap must end the execution as failed. `unimp` is `csrrw x0, cycle, x0`, a
# write to a read-only CSR, which RISC-V defines as an illegal instruction; it is
# what compilers emit for every trap (core::intrinsics::abort, __rust_abort,
# llvm.trap). The exit below must never be reached.

.section .text.init
.global _start

_start:
    unimp
    li a0, 0
    li a7, 93
    ecall
loop:
    j loop
