// Benchmark driver: includes the input file GUEST (its own main is renamed away)
// and runs CALL N times. bench.sh builds it at two N and takes the difference, so
// the fixed costs (boot, RAM image, inputs) cancel out.
//   -DGUEST="\"inputs.c\"" -DCALL="zkvm_...(...)" -DN=3 [-DGUEST_HAS_OB]
#define main guest_main
#include GUEST
#undef main
#ifndef GUEST_HAS_OB
static uint8_t ob[1100];
static bool okb;
#endif
int main(void) {
    for (int i = 0; i < N; i++) {
        __asm__ volatile("" ::: "memory");
        CALL;
    }
    return 0;
}
