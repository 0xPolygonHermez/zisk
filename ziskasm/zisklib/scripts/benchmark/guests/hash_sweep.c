/* Hash every length 0..LMAX at two alignments; fold each digest into a u64
   checksum per (hash, alignment). Output is compared against a golden snapshot. */
#include "zkvm_accelerators.h"
#ifndef LMAX
#define LMAX 420
#endif
static uint8_t buf[LMAX + 16];
static uint64_t acc[8];
static void fold(int k, const uint8_t *h, unsigned n) {
    for (unsigned i = 0; i < n; i++) acc[k] = acc[k] * 1099511628211ULL + h[i] + 1;
}
int main(void) {
    for (unsigned i = 0; i < sizeof buf; i++) buf[i] = (uint8_t)(i * 167 + 13);
    zkvm_bytes_32 h; int st = 0;
    for (unsigned off = 0; off < 2; off++) {
        const uint8_t *p = buf + off * 3;
        for (unsigned len = 0; len <= LMAX; len++) {
            st |= zkvm_keccak256(p, len, &h);  fold(0 + off, h.data, 32);
            st |= zkvm_sha256(p, len, &h);     fold(2 + off, h.data, 32);
            st |= zkvm_ripemd160(p, len, &h);  fold(4 + off, h.data, 32);
        }
    }
    /* blake2f: chain a few rounds counts and flags */
    zkvm_blake2f_state s; zkvm_blake2f_message m; zkvm_blake2f_offset t;
    for (unsigned i = 0; i < 64; i++) s.data[i] = (uint8_t)(i * 7 + 1);
    for (unsigned i = 0; i < 128; i++) m.data[i] = (uint8_t)(i * 11 + 5);
    for (unsigned i = 0; i < 16; i++) t.data[i] = (uint8_t)(i * 3 + 2);
    for (unsigned r = 0; r < 25; r++) { st |= zkvm_blake2f(r, &s, &m, &t, (uint8_t)(r & 1)); fold(6, s.data, 64); }
    acc[7] = (uint64_t)st;
    volatile uint64_t *o = (volatile uint64_t *)0xA0410000ULL;
    for (int i = 0; i < 8; i++) o[i] = acc[i];
    return 0;
}
