/* _start.s — ZisK guest entry point.
 *
 * The ZisK runtime / linker script declares ENTRY(_start) and places this
 * code (section .text.init) first in .text. We set up the global pointer and
 * stack pointer, run the static constructors, call main(), run the static
 * destructors, then halt via the dual hardware/emulator path.
 *
 * Pattern mirrors the ZisK SDK (ziskos/entrypoint/src/lib.rs).
 *
 * This file also defines the DMA-accelerated memcpy/memmove/memcmp/memset (end of
 * file). They live in the same object as _start on purpose, see the note there.
 */
.section .text.init,"ax",@progbits
.global _start
.type _start, @function
_start:
    /* Set gp. .option norelax stops the linker relaxing this into a
       gp-relative form that would use gp before it is initialized. */
    .option push
    .option norelax
    la gp, _global_pointer
    .option pop

    /* Stack pointer -> top of the reserved stack region. */
    la sp, _init_stack_top

    /* C++ static constructors: call every function pointer in
       [__init_array_start, __init_array_end). The linker script KEEPs and
       priority-sorts .init_array, so without this walk a C++ guest's static
       initializers are silently skipped -- the EF standard requires _start to run
       them before main. Referencing the bracket symbols here is also what
       materializes them: they are PROVIDE'd, so an unreferenced pair is absent
       from the link. With no constructors the range is empty (start == end) and
       the loop body never runs.

       s0/s1/s2 are callee-saved, so they survive the calls below; a0 is not, which
       is why main's status is parked in s2 before the destructors run. */
    la   s0, __init_array_start
    la   s1, __init_array_end
4:  bgeu s0, s1, 5f
    ld   t0, 0(s0)
    jalr t0
    addi s0, s0, 8
    j    4b
5:

    /* Call main(); return value lands in a0. */
    call main
    mv   s2, a0                    /* preserve the exit status across destructors */

    /* C++ static destructors, in REVERSE registration order:
       (__fini_array_end, __fini_array_start]. */
    la   s0, __fini_array_end
    la   s1, __fini_array_start
6:  bgeu s1, s0, 7f
    addi s0, s0, -8
    ld   t0, 0(s0)
    jalr t0
    j    6b
7:  mv   a0, s2                    /* restore it for the exit paths below */

    /* Exit dispatch: marchid == 0xFFFEEEE only on real ZisK hardware. */
    csrr t0, marchid
    li   t1, 0xFFFEEEE
    beq  t0, t1, 1f

    /* Emulator (ziskemu / QEMU): sifive_test device @ 0x100000. Encode main's
       return value, which is still in a0: 0 => 0x5555 (pass), nonzero =>
       (a0 << 16) | 0x3333. This mirrors ziskos/entrypoint/src/lib.rs. Writing
       0x5555 unconditionally would report a FAILING guest as a pass, while the
       hardware path below already forwards a0 via ecall -- so the two exit paths
       have to agree. */
    li   t0, 0x100000     /* QEMU_EXIT_ADDR */
    beqz a0, 3f
    slli t1, a0, 16
    li   t2, 0x3333
    or   t1, t1, t2
    sw   t1, 0(t0)
    j    2f
3:  li   t1, 0x5555       /* QEMU_EXIT_CODE (pass) */
    sw   t1, 0(t0)
    j    2f

1:  /* Hardware: standard RISC-V ecall, a7 = 93 (SYS_exit). */
    li   a7, 93
    ecall

2:  /* Spin if the halt above somehow falls through. */
    j    2b

/* ---------------------------------------------------------------------------
 * DMA-accelerated memcpy / memmove / memcmp / memset (EF §2).
 *
 * Each thunk is a `csrs <port>, ...` + `add`/`addi x0, ...` pair that the
 * transpiler lowers to a single DMA precompile op. Unlike the accelerator stubs,
 * these work without the `ziskasm` feature. Bodies are the ones in
 * ziskos/entrypoint/src/dma/*.s, minus their `.attribute` lines (an arch
 * attribute is rejected after the instructions above).
 *
 * Why here and not in their own archive member: ENTRY(_start) makes the linker
 * pull this object into every guest, so these strong definitions are always in
 * the link, whatever the link order. If a libc archive earlier on the command
 * line has already supplied its own mem*, the link fails with a
 * multiple-definition error. It never silently keeps the byte loop, which a
 * separate member would do (it is only pulled if the symbol is still undefined).
 * ------------------------------------------------------------------------- */
.text

/* void *memcpy(void *dst, const void *src, size_t n) -> dma_xmemcpy(a0, a1, a2) */
.globl  memcpy
.p2align 4
.type   memcpy, @function
memcpy:
    csrs 0x813, a1
    add  x0, a0, a2
    ret
.size memcpy, .-memcpy

/* void *memmove(void *dst, const void *src, size_t n): the same op as memcpy.
   The DMA copy has memmove semantics: when src and dst overlap, the emulator
   copies through a temporary buffer (Mem::memcpy in core/src/mem.rs) and the
   prover constrains the overlapping case too, so one op is correct for every
   overlap direction. */
.globl  memmove
.p2align 4
.type   memmove, @function
memmove:
    csrs 0x813, a1
    add  x0, a0, a2
    ret
.size memmove, .-memmove

/* int memcmp(const void *a, const void *b, size_t n) -> dma_xmemcmp, result in a0 */
.globl  memcmp
.p2align 4
.type   memcmp, @function
memcmp:
    csrrs a0, 0x814, a1            /* result -> a0 */
    add   x0, a0, a2
    ret
.size memcmp, .-memcmp

/* void *memset(void *s, int c, size_t n) -> dma_xmemset(a0, a2, fill). The fill
   byte travels as the addi immediate, so a 256-entry jump table (16 bytes per
   entry) selects the right immediate at runtime; c == 0 takes the direct path. */
.globl  memset
.p2align 4
.type   memset, @function
memset:
    bnez a1, .Lmemset_non_zero
    csrs 0x816, a0
    addi x0, a2, 0
    ret
.Lmemset_non_zero:
    andi a1, a1, 0xff
    slli a1, a1, 4
    la   t0, .Lmemset_table
    add  t0, t0, a1
    jr   t0
    .p2align 4
.Lmemset_table:
.irp val, 0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31
    csrs 0x816, a0; addi x0, a2, \val; ret; nop
.endr
.irp val, 32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,52,53,54,55,56,57,58,59,60,61,62,63
    csrs 0x816, a0; addi x0, a2, \val; ret; nop
.endr
.irp val, 64,65,66,67,68,69,70,71,72,73,74,75,76,77,78,79,80,81,82,83,84,85,86,87,88,89,90,91,92,93,94,95
    csrs 0x816, a0; addi x0, a2, \val; ret; nop
.endr
.irp val, 96,97,98,99,100,101,102,103,104,105,106,107,108,109,110,111,112,113,114,115,116,117,118,119,120,121,122,123,124,125,126,127
    csrs 0x816, a0; addi x0, a2, \val; ret; nop
.endr
.irp val, 128,129,130,131,132,133,134,135,136,137,138,139,140,141,142,143,144,145,146,147,148,149,150,151,152,153,154,155,156,157,158,159
    csrs 0x816, a0; addi x0, a2, \val; ret; nop
.endr
.irp val, 160,161,162,163,164,165,166,167,168,169,170,171,172,173,174,175,176,177,178,179,180,181,182,183,184,185,186,187,188,189,190,191
    csrs 0x816, a0; addi x0, a2, \val; ret; nop
.endr
.irp val, 192,193,194,195,196,197,198,199,200,201,202,203,204,205,206,207,208,209,210,211,212,213,214,215,216,217,218,219,220,221,222,223
    csrs 0x816, a0; addi x0, a2, \val; ret; nop
.endr
.irp val, 224,225,226,227,228,229,230,231,232,233,234,235,236,237,238,239,240,241,242,243,244,245,246,247,248,249,250,251,252,253,254,255
    csrs 0x816, a0; addi x0, a2, \val; ret; nop
.endr
.size memset, .-memset
