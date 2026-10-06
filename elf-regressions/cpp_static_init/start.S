// Startup for the C++ static-initialization regression test.
//
// Mirrors the ordering of ziskos's `_start` / `_zisk_main`
// (ziskos/entrypoint/src/lib.rs): set up gp/sp, walk `.init_array` forwards,
// call `main`, run the `__cxa_atexit`-registered destructors, walk
// `.fini_array` backwards, then terminate with the exit ecall.
//
// `.bss` is deliberately NOT cleared here: the ELF interpreter must zero it
// from the writable segment's `p_memsz > p_filesz` tail, and the guarded
// function-local static in main.cpp depends on that.

    .section .text.init
    .global _start
_start:
    .option push
    .option norelax
    la      gp, _global_pointer
    .option pop

    la      sp, _init_stack_top

    // .init_array, forward order
    la      s0, __init_array_start
    la      s1, __init_array_end
1:  bgeu    s0, s1, 2f
    ld      t0, 0(s0)
    jalr    t0
    addi    s0, s0, 8
    j       1b
2:

    call    main
    mv      s2, a0                  // keep the exit code

    call    __cxa_finalize_all

    // .fini_array, reverse order
    la      s0, __fini_array_start
    la      s1, __fini_array_end
3:  bgeu    s0, s1, 4f
    addi    s1, s1, -8
    ld      t0, 0(s1)
    jalr    t0
    j       3b
4:

    mv      a0, s2
    li      a7, 93                  // exit
    ecall
5:  j       5b
