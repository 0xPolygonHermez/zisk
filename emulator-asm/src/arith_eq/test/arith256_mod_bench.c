// Correctness + benchmark for arith256_mod (a*b+c mod m, 256-bit).
//
// Compares two call paths over many operations:
//   * legacy_op : the existing Rust Arith256Mod, called with the full register-save wrapper the
//                 emulator currently generates (8 GPR pushes + 16 xmm saves).
//   * new_op    : our assembly arith256_mod, called with a minimal wrapper (saves every volatile
//                 GPR except r8/r9/r11, no xmm).
//
// It first checks both paths agree with each other and with a GMP reference, then benchmarks them.
//
// Build & run:  make && ./arith256_mod_bench

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <gmp.h>
#include <x86intrin.h>

extern int legacy_op(uint64_t **address);
extern int new_op(uint64_t **address);
extern int legacy_nop(uint64_t **address);   // full wrapper around an empty callee
extern int new_nop(uint64_t **address);       // minimal wrapper around an empty callee

static uint64_t r64(void) {
    return ((uint64_t)rand() << 48) ^ ((uint64_t)rand() << 32) ^ ((uint64_t)rand() << 16) ^ rand();
}

static void ref_gmp(const uint64_t a[4], const uint64_t b[4], const uint64_t c[4],
                    const uint64_t m[4], uint64_t out[4]) {
    mpz_t A, B, C, M, R;
    mpz_inits(A, B, C, M, R, NULL);
    mpz_import(A, 4, -1, 8, 0, 0, a);
    mpz_import(B, 4, -1, 8, 0, 0, b);
    mpz_import(C, 4, -1, 8, 0, 0, c);
    mpz_import(M, 4, -1, 8, 0, 0, m);
    mpz_mul(R, A, B);
    mpz_add(R, R, C);
    mpz_mod(R, R, M);
    out[0] = out[1] = out[2] = out[3] = 0;
    mpz_export(out, NULL, -1, 8, 0, 0, R);
    mpz_clears(A, B, C, M, R, NULL);
}

typedef int (*op_fn)(uint64_t **);

static unsigned long long bench(op_fn f, uint64_t **addr, uint64_t *scratch, uint64_t *d, long n) {
    unsigned long long best = ~0ULL;
    for (long i = 0; i < 100000; i++) { scratch[0] += d[0] | 1; f(addr); }
    for (int rep = 0; rep < 9; rep++) {
        unsigned aux;
        _mm_lfence();
        unsigned long long t0 = __rdtscp(&aux);
        _mm_lfence();
        for (long i = 0; i < n; i++) { scratch[0] += d[0] | 1; f(addr); }
        _mm_lfence();
        unsigned long long t1 = __rdtscp(&aux);
        _mm_lfence();
        unsigned long long per = (t1 - t0) / n;
        if (per < best) best = per;
    }
    return best;
}

int main(int argc, char **argv) {
    long checks = argc > 1 ? atol(argv[1]) : 1000000;
    long benchn = argc > 2 ? atol(argv[2]) : 2000000;
    srand(20260710);

    // ---- correctness: legacy vs new vs GMP, over many random operations & modulus sizes ----
    long fails = 0;
    for (long t = 0; t < checks; t++) {
        uint64_t a[4], b[4], c[4], m[4], dl[4] = {0}, dn[4] = {0}, e[4];
        for (int i = 0; i < 4; i++) { a[i] = r64(); b[i] = r64(); c[i] = r64(); m[i] = r64(); }
        if ((m[0] | m[1] | m[2] | m[3]) == 0) m[0] = 1;
        switch (t % 9) {                     // exercise 1..4-limb moduli and the s==0 path
            case 0: m[1] = m[2] = m[3] = 0; m[0] = (r64() % 1000000) + 1; break;
            case 2: m[2] = m[3] = 0; break;
            case 3: m[3] = 0; break;
            case 4: m[0] = m[1] = m[2] = 0; m[3] = (r64() | 0x8000000000000000ULL); break;
        }
        uint64_t *la[5] = {a, b, c, m, dl};
        uint64_t *na[5] = {a, b, c, m, dn};
        legacy_op(la);
        new_op(na);
        ref_gmp(a, b, c, m, e);
        if (memcmp(dl, dn, 32) || memcmp(dn, e, 32)) {
            if (fails < 4)
                printf("MISMATCH t=%ld\n legacy %016lx%016lx%016lx%016lx\n new    %016lx%016lx%016lx%016lx\n gmp    %016lx%016lx%016lx%016lx\n",
                       t, dl[3], dl[2], dl[1], dl[0], dn[3], dn[2], dn[1], dn[0], e[3], e[2], e[1], e[0]);
            fails++;
        }
    }
    printf("correctness: %ld/%ld ops match (legacy == new == gmp)%s\n",
           checks - fails, checks, fails ? "  <-- FAILURES" : "");

    // ---- benchmark ----
    struct { const char *name; uint64_t m[4]; } cases[] = {
        {"4-limb modulus (~2^255)",   {0xffffffffffffff43ULL, ~0ULL, ~0ULL, 0x7fffffffffffffffULL}},
        {"3-limb modulus (~2^191)",   {0xfedcba9876543211ULL, 0x1122334455667788ULL, 0x7fffffffffffffffULL, 0}},
        {"2-limb modulus (~2^127)",   {0xdeadbeefcafef00dULL, 0x7fffffffffffffffULL, 0, 0}},
        {"1-limb modulus (~2^63)",    {0x7fffffffffffffe7ULL, 0, 0, 0}},
        {"small modulus (~2^20)",     {1000003, 0, 0, 0}},
    };
    uint64_t a0[4] = {0x123456789abcdef0ULL, 0xfedcba98ULL, 0x11112222ULL, 0x55556666ULL};
    uint64_t b0[4] = {0xdeadbeefcafef00dULL, 0x0f0f0f0fULL, 0xa5a5a5a5ULL, 0x99998888ULL};
    uint64_t c0[4] = {1, 2, 3, 4};

    printf("\nbenchmark (min cycles/op over 9 runs of %ld ops):\n", benchn);
    printf("  %-28s %10s %10s %9s\n", "case", "legacy", "new-asm", "speedup");
    for (int k = 0; k < (int)(sizeof(cases) / sizeof(cases[0])); k++) {
        uint64_t a[4], b[4], c[4], d[4] = {0};
        memcpy(a, a0, 32); memcpy(b, b0, 32); memcpy(c, c0, 32);
        uint64_t *addr[5] = {a, b, c, cases[k].m, d};
        unsigned long long L = bench(legacy_op, addr, a, d, benchn);
        unsigned long long N = bench(new_op, addr, a, d, benchn);
        printf("  %-28s %10llu %10llu %8.2fx\n", cases[k].name, L, N, (double)L / (double)N);
    }

    // ---- isolated wrapper cost (same wrappers around an empty callee) ----
    {
        uint64_t a[4], b[4], c[4], d[4] = {0};
        memcpy(a, a0, 32); memcpy(b, b0, 32); memcpy(c, c0, 32);
        uint64_t *addr[5] = {a, b, c, cases[0].m, d};
        unsigned long long L = bench(legacy_nop, addr, a, d, benchn);
        unsigned long long N = bench(new_nop, addr, a, d, benchn);
        printf("\nisolated wrapper cost (empty callee):\n");
        printf("  %-28s %10s %10s\n", "", "full", "minimal");
        printf("  %-28s %10llu %10llu   (delta %llu cyc saved)\n",
               "8 GPR + 16 xmm  vs  6 GPR", L, N, L > N ? L - N : 0);
    }
    return fails ? 1 : 0;
}
