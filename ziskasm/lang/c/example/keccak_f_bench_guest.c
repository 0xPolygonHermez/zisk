/* keccak_f_bench_guest.c -- step cost of calling Keccak-f[1600] N times, inlined vs
 * through a real (out-of-line) function call.
 *
 * Default build: zkvm_keccak_f1600 from zkvm_accelerators.h, which is inline, so
 * each call is a single `csrs 0x800, <reg>` at the call site.
 *
 * -DKF_CALL: the same precompile behind a real function (kf_call below: `csrs 0x800,
 * a0; li a0, 0; ret`), which is what an out-of-line zkvm_keccak_f1600 in
 * zkvm_calls.s would look like. The call is opaque to the compiler, so it also has
 * to keep live values out of caller-saved registers.
 *
 * Both variants permute the same state N times (each permutation depends on the
 * previous one) and emit its first 32 bytes, so their outputs must match.
 *
 *   riscv64-unknown-elf-gcc -march=rv64ima -mabi=lp64 -mcmodel=medany -nostdlib \
 *       -ffreestanding -O2 -Wl,--gc-sections -I. -I../include \
 *       -T ../../../../ziskbuild/zisk_linker_script.ld [-DKF_CALL] [-DKF_N=10000] \
 *       -o kf.elf ../src/_start.s keccak_f_bench_guest.c
 *   ziskemu -e kf.elf -i empty.bin -o out.bin -X
 */
#include "zkvm_accelerators.h"
#include "emit.h"

#ifndef KF_N
#define KF_N 10000
#endif

#ifdef KF_CALL
zkvm_status kf_call(uint64_t *state);
__asm__(
    "    .section .text.kf_call,\"ax\",@progbits\n"
    "    .globl  kf_call\n"
    "    .type   kf_call, @function\n"
    "kf_call:\n"
    "    csrs    0x800, a0\n"
    "    li      a0, 0\n"
    "    ret\n"
    "    .size   kf_call, . - kf_call\n");
#define KECCAK_F kf_call
#else
#define KECCAK_F zkvm_keccak_f1600
#endif

static uint64_t st[25] = {1};

int main(void) {
    unsigned failures = 0;
    for (unsigned i = 0; i < KF_N; i++) {
        /* Check the status, as a real caller would. */
        if (KECCAK_F(st) != ZKVM_EOK) failures++;
    }
    if (failures) __asm__ volatile("unimp");
    emit32((const uint8_t *)st);
    return 0;
}
