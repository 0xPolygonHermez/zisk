/* _start.s — ZisK guest entry point.
 *
 * The ZisK runtime / linker script declares ENTRY(_start) and places this
 * code (section .text.init) first in .text. We set up the global pointer and
 * stack pointer, run the static constructors, call main(), run the static
 * destructors, then halt via the dual hardware/emulator path.
 *
 * Pattern mirrors the ZisK SDK (ziskos/entrypoint/src/lib.rs).
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
