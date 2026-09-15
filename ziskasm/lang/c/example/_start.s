/* _start.s — ZisK guest entry point.
 *
 * The ZisK runtime / linker script declares ENTRY(_start) and places this
 * code (section .text.init) first in .text. We set up the global pointer and
 * stack pointer, call main(), then halt via the dual hardware/emulator path.
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

    /* Call main(); return value lands in a0. */
    call main

    /* Exit dispatch: marchid == 0xFFFEEEE only on real ZisK hardware. */
    csrr t0, marchid
    li   t1, 0xFFFEEEE
    beq  t0, t1, 1f

    /* Emulator (ziskemu / QEMU): write magic to the magic exit address. */
    li   t0, 0x100000     /* QEMU_EXIT_ADDR */
    li   t1, 0x5555       /* QEMU_EXIT_CODE */
    sw   t1, 0(t0)
    j    2f

1:  /* Hardware: standard RISC-V ecall, a7 = 93 (SYS_exit). */
    li   a7, 93
    ecall

2:  /* Spin if the halt above somehow falls through. */
    j    2b
