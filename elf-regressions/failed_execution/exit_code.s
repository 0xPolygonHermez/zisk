# A nonzero exit code is a failed execution: exit(42) must not end successfully,
# and the error must carry the code.

.section .text.init
.global _start

_start:
    li a0, 42
    li a7, 93
    ecall
loop:
    j loop
