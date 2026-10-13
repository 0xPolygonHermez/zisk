# ebreak raises a breakpoint exception, so the execution fails there. GCC emits it
# for __builtin_trap, which C's abort() reaches. The exit below must never be
# reached.

.section .text.init
.global _start

_start:
    ebreak
    li a0, 0
    li a7, 93
    ecall
loop:
    j loop
