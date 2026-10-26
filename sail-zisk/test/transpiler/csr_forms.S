/* Every CSR form of the contract (doc/riscv_contract.md, §§ 6–8), and the
 * sequences the transpiler refuses, for `make transpiler-check`: never run,
 * only transpiled. */
    .option norvc
    .section .text.init
    .globl _start, rvtest_entry_point
_start:
rvtest_entry_point:
    csrr a0, marchid
    # one pointer
    csrs 0x800, a0
    csrs 0x801, a0
    csrs 0x802, a0
    csrs 0x804, a0
    csrs 0x807, a0
    csrs 0x80b, a0
    csrs 0x80d, a0
    csrs 0x812, a0
    csrs 0x818, a0
    csrs 0x81b, a0
    csrs 0x821, a0
    # two pointers
    csrs 0x803, a0
    add x0, a1, x0
    csrs 0x805, a0
    add x0, a1, x0
    csrs 0x806, a0
    add x0, a1, x0
    csrs 0x808, a0
    add x0, a1, x0
    csrs 0x809, a0
    add x0, a1, x0
    csrs 0x80a, a0
    add x0, a1, x0
    csrs 0x80c, a0
    add x0, a1, x0
    csrs 0x80e, a0
    add x0, a1, x0
    csrs 0x80f, a0
    add x0, a1, x0
    csrs 0x810, a0
    add x0, a1, x0
    csrs 0x817, a0
    add x0, a1, x0
    csrs 0x81d, a0
    add x0, a1, x0
    csrs 0x81e, a0
    add x0, a1, x0
    csrs 0x81f, a0
    add x0, a1, x0
    csrs 0x820, a0
    add x0, a1, x0
    # blake2b
    csrs 0x819, a0
    addi x0, a1, 5
    # add256, any rd
    csrrs a2, 0x811, a0
    csrrs zero, 0x811, a0
    csrrs a0, 0x811, a0
    # memcpy, memcmp, jump_dest: count in a register
    csrrs a3, 0x813, a1
    add x0, a0, a2
    csrrs zero, 0x814, a1
    add x0, a0, a2
    csrrs a4, 0x81c, a1
    add x0, a0, a2
    # the deprecated form: rd from the add
    csrs 0x813, a1
    add a3, a0, a2
    # xmemcpy, xmemcmp: count an immediate
    csrs 0x813, a1
    addi x0, a0, 16
    csrs 0x814, a1
    addi x0, a0, -16
    csrs 0x813, a1
    addi a3, a0, 24
    # memset
    csrs 0x816, a0
    addi a3, a2, 7
    csrs 0x816, a0
    addi x0, a2, 255
    csrsi 0x816, 2
    addi a3, a0, 32
    addi x0, x0, 5
    # inputcpy
    csrs 0x815, a2
    add a3, a0, a2
    csrs 0x815, a0
    addi a3, a0, 64
    # profile
    csrs 0x81a, a0
    addi x0, x0, 3
    # fcall_param
    csrs 0x8f0, a0
    csrs 0x8f1, a0
    csrs 0x8f2, a0
    csrs 0x8f3, a0
    csrs 0x8f4, a0
    csrs 0x8f5, a0
    csrs 0x8f6, a0
    csrs 0x8f7, a0
    csrs 0x8f8, a0
    csrs 0x8f9, a0
    csrs 0x8fa, a0
    csrs 0x8fb, a0
    csrs 0x8fc, a0
    csrs 0x8fd, a0
    csrs 0x8fe, a0
    csrs 0x8ff, a0
    # fcall
    csrwi 0x8c0, 5
    csrwi 0x8df, 31
    csrwi 0x8c3, 0
    # fcall_get
    csrr a0, 0xffe
    csrr t1, 0xffe
    # nothing
    csrs 0x800, x0
    csrs 0x8f0, x0
    csrs 0x811, x0
    # refused
    csrs 0x803, a0
    addi x0, a1, 0
    csrrs a3, 0x81c, a1
    addi x0, a0, 4
    csrs 0x816, a0
    addi a3, a2, 300
    csrsi 0x816, 1
    csrsi 0x816, 2
    addi a3, a0, 32
    addi x0, x0, 300
    csrsi 0x816, 2
    add a3, a0, a1
    addi x0, x0, 5
    csrs 0x81a, a0
    addi x0, a1, 3
    csrs 0x819, a0
    add x0, a1, x0
    csrs 0x815, a0
    sub a3, a0, a2
    csrrs a3, 0x813, a1
    sub x0, a0, a2
    ecall
    # a call at the end of the code
    csrs 0x803, a0
