# A nonzero exit code is a failed execution: exit(1) must not end successfully.

.section .text.init
.global _start

_start:
    li a0, 1
    li a7, 93
    ecall
loop:
    j loop
