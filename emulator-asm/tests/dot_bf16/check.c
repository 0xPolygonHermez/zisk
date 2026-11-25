// Checker of the dot_bf16 C routine against the cases of emulator-asm/tests/dot_bf16/run.sh.
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include "dot_bf16.h"
static void rd(void *p, size_t n, FILE *f) { if (n && fread(p, 1, n, f) != n) { fprintf(stderr, "short read\n"); exit(2); } }
int main(int argc, char **argv) {
    FILE *f = fopen(argv[1], "rb");
    uint64_t h[5], cases = 0, bad = 0, per_flag[3] = {0};
    while (fread(h, 8, 5, f) == 5) {
        uint64_t flags = h[0], n = h[1], block = h[2], xb = h[3], sb = h[4], nb = n / block;
        uint8_t *x = aligned_alloc(8, xb + 8), *s = aligned_alloc(8, sb + 16);
        uint16_t *y = aligned_alloc(8, 2 * n + 8);
        uint32_t total, *exp_out = malloc(4 * nb + 4), *out = aligned_alloc(8, 4 * nb + 8);
        rd(x, xb, f); rd(s, sb, f); rd(y, 2 * n, f); rd(&total, 4, f); rd(exp_out, 4 * nb, f);
        memset(s + sb, 0xff, 8);   // bytes past the scales must be ignored
        uint64_t p[7] = { (uint64_t)x, (uint64_t)y, n, flags, block, (uint64_t)out, (uint64_t)s };
        uint32_t got = dot_bf16_compute(p);
        int ok = got == total;
        for (uint64_t b = 0; b < nb; b++) ok &= out[b] == exp_out[b];
        if (!ok && bad++ < 5) printf("case %lu flags %lu: n=%lu block=%lu total %08x expected %08x\n", cases, flags, n, block, got, total);
        cases++; per_flag[flags]++;
        free(x); free(s); free(y); free(exp_out); free(out);
    }
    printf("%lu cases (%lu BF16, %lu MXFP4, %lu NVFP4), %lu mismatches\n", cases, per_flag[0], per_flag[1], per_flag[2], bad);
    return bad != 0;
}
