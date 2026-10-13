/* zkvm_mem.s -- the out-of-line side of zkvm_mem.h: zkvm_memset_any, and weak libc
 * memcpy / memmove / memcmp / memset for freestanding ZisK guests (the compiler
 * emits calls to these names). Each is the same DMA marker the header inlines: a
 * `csrs` plus the `add`/`addi` that follows, which the transpiler folds into one
 * DMA operation. memmove is memcpy: the DMA copy is overlap-safe.
 *
 * memset's fill byte must be the `addi` immediate, so a 256-entry jump table (16
 * bytes per entry, assembled without compressed instructions) turns the run-time
 * byte into one. */

        .section ".note.GNU-stack","",@progbits
        .text

        .globl  memcpy
        .weak   memcpy
        .globl  memmove
        .weak   memmove
        .p2align 4
        .type   memcpy,@function
memcpy:
memmove:
        csrs    0x813, a1               /* src */
        add     x0, a0, a2              /* dst, count */
        ret
        .size   memcpy, .-memcpy

        .globl  memcmp
        .weak   memcmp
        .p2align 4
        .type   memcmp,@function
memcmp:
        csrrs   a0, 0x814, a1           /* result -> a0, src (b) = a1 */
        add     x0, a0, a2              /* dst (a) = a0, count */
        ret
        .size   memcmp, .-memcmp

/* void* zkvm_memset_any(void* dst = a0, int c = a1, size_t n = a2) -> a0 = dst */
        .globl  zkvm_memset_any
        .globl  memset
        .weak   memset
        .p2align 4
        .type   zkvm_memset_any,@function
zkvm_memset_any:
memset:
        bnez    a1, 1f
        csrs    0x816, a0
        addi    x0, a2, 0
        ret
1:      andi    a1, a1, 0xff
        slli    a1, a1, 4
        la      t0, 2f
        add     t0, t0, a1
        jr      t0
        /* 16 bytes per entry: no compressed ret/nop, or the a1 << 4 index misses */
        .option push
        .option norvc
        .p2align 4
2:
        .irp    val, 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31
        csrs 0x816, a0; addi x0, a2, \val; ret; nop
        .endr
        .irp    val, 32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63
        csrs 0x816, a0; addi x0, a2, \val; ret; nop
        .endr
        .irp    val, 64,65,66,67,68,69,70,71,72,73,74,75,76,77,78,79,80,81,82,83,84,85,86,87,88,89,90,91,92,93,94,95
        csrs 0x816, a0; addi x0, a2, \val; ret; nop
        .endr
        .irp    val, 96,97,98,99,100,101,102,103,104,105,106,107,108,109,110,111,112,113,114,115,116,117,118,119,120,121,122,123,124,125,126,127
        csrs 0x816, a0; addi x0, a2, \val; ret; nop
        .endr
        .irp    val, 128,129,130,131,132,133,134,135,136,137,138,139,140,141,142,143,144,145,146,147,148,149,150,151,152,153,154,155,156,157,158,159
        csrs 0x816, a0; addi x0, a2, \val; ret; nop
        .endr
        .irp    val, 160,161,162,163,164,165,166,167,168,169,170,171,172,173,174,175,176,177,178,179,180,181,182,183,184,185,186,187,188,189,190,191
        csrs 0x816, a0; addi x0, a2, \val; ret; nop
        .endr
        .irp    val, 192,193,194,195,196,197,198,199,200,201,202,203,204,205,206,207,208,209,210,211,212,213,214,215,216,217,218,219,220,221,222,223
        csrs 0x816, a0; addi x0, a2, \val; ret; nop
        .endr
        .irp    val, 224,225,226,227,228,229,230,231,232,233,234,235,236,237,238,239,240,241,242,243,244,245,246,247,248,249,250,251,252,253,254,255
        csrs 0x816, a0; addi x0, a2, \val; ret; nop
        .endr
        .option pop
        .size   zkvm_memset_any, .-zkvm_memset_any
