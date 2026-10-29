/* Data in .text: a word whose low half is no instruction's start (0x0000)
 * and whose high half is (0x0013, low bits 11). The transpiler must walk the
 * code a word at a time, as RISC-V does with IALIGN = 32: a halt for that
 * word, and the instruction after it at its own address, which the jump
 * reaches. Run, it exits with code 0. */
    .option norvc
    .section .text.init
    .globl _start, rvtest_entry_point
_start:
rvtest_entry_point:
    j over
    .word 0x00130000
over:
    li a0, 0
    li a7, 93
    ecall
