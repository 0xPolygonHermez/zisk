/* zkvm_keccak256_cached against zkvm_keccak256: every length 0..LMAX at two input and
   two output alignments, before any table is registered (plain sponge), with a fresh
   table (misses), the same table again (hits), a table that fills up after 3
   entries, a second table shifted by one entry, which the executor's hints (filed
   under the first table's indices) point into the wrong entries of, and capacity 0. */
#include "zkvm_accelerators.h"
#include "zkvm_zisklib.h"
#ifndef LMAX
#define LMAX 420
#endif
#define CAP 4096
static uint8_t buf[LMAX + 16];
static uint64_t ta[ZKVM_KECCAKF_CACHE_WORDS(CAP)], tb[ZKVM_KECCAKF_CACHE_WORDS(CAP)];
static uint64_t ts[ZKVM_KECCAKF_CACHE_WORDS(3)];
static uint64_t bad, calls;
static int eq32(const uint8_t *a, const uint8_t *b) {
    for (int i = 0; i < 32; i++) if (a[i] != b[i]) return 0;
    return 1;
}
static void pass(void) {
    zkvm_bytes_32 h; uint8_t c[33];
    for (unsigned off = 0; off < 2; off++)
        for (unsigned len = 0; len <= LMAX; len++) {
            const uint8_t *p = buf + off * 3;
            zkvm_keccak256(p, len, &h);
            int st = zkvm_keccak256_cached(p, len, c + off);
            bad += (st != 0) + !eq32(h.data, c + off);
            calls++;
        }
}
int main(void) {
    for (unsigned i = 0; i < sizeof buf; i++) buf[i] = (uint8_t)(i * 167 + 13);
    ta[0] = 12345; tb[0] = 777; ts[0] = 99;   /* init must reset the count */
    pass();                                   /* nothing registered yet */
    zkvm_keccak256_cache_init(ta, CAP);
    pass(); uint64_t a1 = ta[0];
    pass(); uint64_t a2 = ta[0];
    zkvm_keccak256_cache_init(ts, 3);
    pass();
    zkvm_keccak256_cache_init(tb, CAP);
    { uint8_t d[32]; zkvm_keccak256_cached(buf + 1, 7, d); }   /* shift tb's indices by one */
    pass();
    zkvm_keccak256_cache_init(ta, 0);         /* memo off */
    pass();
    volatile uint64_t *o = (volatile uint64_t *)0xA0410000ULL;
    o[0] = bad; o[1] = calls; o[2] = a1; o[3] = a2; o[4] = ts[0]; o[5] = tb[0]; o[6] = ta[0];
    return 0;
}
