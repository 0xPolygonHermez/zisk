// abort (EF termination standard, draft eth-act PR #46) guest: main writes an output,
// then calls abort from a nested function. abort must end the run as a failed
// execution with exit code 134 and run no destructors: the destructor below traps,
// so a run that reached it would fail as a trap instead. Built with RETURN_NORMALLY,
// main returns 0 instead and the destructor's trap ends the run, which shows that the
// destructor is registered.
#include <stdint.h>
#include "zkvm_io.h"

_Noreturn void abort(void);

__attribute__((destructor)) static void trap_if_run(void) {
    __asm__ volatile("ebreak");
}

__attribute__((noinline)) static void fail(uint32_t depth) {
    if (depth == 0) abort();
    fail(depth - 1);
}

int main(void) {
    static const uint8_t out[4] = {0x2a, 0, 0, 0};
    write_output(out, sizeof out);
#ifndef RETURN_NORMALLY
    fail(3);
#endif
    return 0;
}
