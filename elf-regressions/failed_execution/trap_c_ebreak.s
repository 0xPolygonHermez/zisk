# c.ebreak, the compressed ebreak, fails the same way (with the `compressed`
# feature it decodes as c.ebreak; without it, any 16-bit instruction halts). The
# exit below must never be reached.

.section .text.init
.global _start

_start:
    .option push
    .option rvc
    c.ebreak
    .option pop
    .balign 4
    li a0, 0
    li a7, 93
    ecall
loop:
    j loop
