/* zkvm_fcall.s -- zkvm_fcall.h on RISC-V: free-input calls (unverified hints).
 *
 * The transpiler turns the fcall CSR accesses into the fcall instructions:
 *   csrs  0x8F0, rs      passes one parameter word (port 0: the value itself)
 *   csrwi 0x8C0, id      issues fcall `id` (IDs below 32 use CSR 0x8C0)
 *   csrr  rd, 0xFFE      reads the next result word
 * The executor needs the ID in the instruction, so zkvm_fcall jumps to one csrwi
 * per hint, picked by the ID. The IDs are those of zkvm_fcall.h, and wasm2rom
 * accepts the same ones (FCALL_HINT_IDS in transpilers/wasm/src/zkvm.rs).
 *
 * Each function has its own section so --gc-sections drops the unused ones.
 */

/* zkvm_status zkvm_fcall(uint32_t id, const uint64_t* params, size_t n_params) */
    .section .text.zkvm_fcall,"ax",@progbits
    .globl  zkvm_fcall
    .type   zkvm_fcall, @function
    .p2align 2
zkvm_fcall:
    li      t0, 386                 /* ZKVM_FCALL_MAX_PARAMS */
    bltu    t0, a2, 9f
    li      t0, 26                  /* the hints are 1..22 and 26 */
    beq     a0, t0, 1f
    beqz    a0, 9f
    li      t0, 22
    bltu    t0, a0, 9f
1:  slli    a2, a2, 3               /* a2 = the end of params */
    add     a2, a1, a2
2:  beq     a1, a2, 3f
    ld      t0, 0(a1)
    csrs    0x8F0, t0
    addi    a1, a1, 8
    j       2b
3:  la      t0, 4f                  /* jump to entry `id`, 8 bytes each */
    slli    t1, a0, 3
    add     t0, t0, t1
    jr      t0
    .option push
    .option norvc
4:
    .irp    id, 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26
    .if     (\id >= 1 && \id <= 22) || \id == 26
    csrwi   0x8C0, \id
    .else
    nop                             /* not a hint: never reached */
    .endif
    j       5f
    .endr
    .option pop
5:  li      a0, 0                   /* ZKVM_EOK */
    ret
9:  li      a0, -1                  /* ZKVM_EFAIL */
    ret
    .size   zkvm_fcall, . - zkvm_fcall

/* uint64_t zkvm_fcall_get(void) */
    .section .text.zkvm_fcall_get,"ax",@progbits
    .globl  zkvm_fcall_get
    .type   zkvm_fcall_get, @function
    .p2align 2
zkvm_fcall_get:
    csrr    a0, 0xFFE
    ret
    .size   zkvm_fcall_get, . - zkvm_fcall_get

/* void zkvm_fcall_read(uint64_t* results, size_t n) */
    .section .text.zkvm_fcall_read,"ax",@progbits
    .globl  zkvm_fcall_read
    .type   zkvm_fcall_read, @function
    .p2align 2
zkvm_fcall_read:
    slli    a1, a1, 3               /* a1 = the end of results */
    add     a1, a0, a1
1:  beq     a0, a1, 2f
    csrr    t0, 0xFFE
    sd      t0, 0(a0)
    addi    a0, a0, 8
    j       1b
2:  ret
    .size   zkvm_fcall_read, . - zkvm_fcall_read
