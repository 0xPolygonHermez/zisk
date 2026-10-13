/* Self-checking test of the ZisK memory and EVM extensions (zkvm_mem.h, zkvm_evm.h)
   against plain C reference loops: memcpy over lengths 0..300 at unaligned offsets
   and overlapping in both directions, memset with fills 0, 0xff and others (a
   run-time fill goes through zkvm_memset_any), memcmp's sign at every differing
   position, the one-instruction forms a constant size selects, the weak libc names
   of src/zkvm_mem.s, and the JUMPDEST bitmap of pseudo-random bytecode (full of
   PUSHes) against a software walk, including the cases the precompile refuses
   (unaligned, empty), which must write nothing.
   Output: one byte per test group at 0xa0410000, 1 = a failure in that group, and
   1 in the byte after the last group once all have run. check.sh compares it with a
   golden snapshot. Build with -march=rv64ima_zicsr. */
#include <stddef.h>
#include <stdint.h>

#include <string.h>  /* the libc names, defined by src/zkvm_mem.s */

#include "zkvm_mem.h"
#include "zkvm_evm.h"

static volatile uint8_t* const O = (volatile uint8_t*)0xA0410000ULL;

static uint64_t rng = 0x9e3779b97f4a7c15ULL;
static uint8_t rnd8(void) { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; return (uint8_t)rng; }

static uint8_t A[1024], B[1024], R[1024];

static void fill_rand(uint8_t* p, size_t n) { for (size_t i = 0; i < n; i++) p[i] = rnd8(); }
static int same(const uint8_t* a, const uint8_t* b, size_t n) {
    for (size_t i = 0; i < n; i++) if (a[i] != b[i]) return 0;
    return 1;
}
static void ref_move(uint8_t* d, const uint8_t* s, size_t n) {   /* memmove */
    if (d < s) for (size_t i = 0; i < n; i++) d[i] = s[i];
    else for (size_t i = n; i-- > 0;) d[i] = s[i];
}
static int sign(int x) { return (x > 0) - (x < 0); }

/* The software JUMPDEST walk, the reference for the precompile. */
static void ref_jumpdest(const uint8_t* code, size_t n, uint64_t* bm) {
    for (size_t w = 0; w < (n + 63) / 64; w++) bm[w] = 0;
    for (size_t i = 0; i < n; i++) {
        uint8_t op = code[i];
        if (op == 0x5b) bm[i / 64] |= 1ULL << (i % 64);
        else if (op >= 0x60 && op <= 0x7f) i += op - 0x5f;   /* skip PUSH1..PUSH32 data */
    }
}

int main(void) {
    int g = 0, bad;

    /* 0: memcpy, disjoint, unaligned, 0..300 bytes */
    bad = 0;
    for (size_t n = 0; n <= 300; n += (n < 40 ? 1 : 7))
        for (int so = 0; so < 8; so += 3)
            for (int dofs = 0; dofs < 8; dofs += 5) {
                fill_rand(A, 400); fill_rand(B, 400);
                for (int i = 0; i < 400; i++) R[i] = B[i];
                ref_move(R + dofs, A + so, n);
                if (zkvm_memcpy(B + dofs, A + so, n) != B + dofs) bad = 1;
                if (!same(B, R, 400)) bad = 1;
            }
    O[g++] = bad;

    /* 1: memcpy/memmove, overlapping both ways */
    bad = 0;
    for (size_t n = 1; n <= 200; n += 13)
        for (int shift = -9; shift <= 9; shift += 3) {
            if (shift == 0) continue;
            fill_rand(A, 600);
            for (int i = 0; i < 600; i++) R[i] = A[i];
            ref_move(R + 200 + shift, R + 200, n);
            zkvm_memcpy(A + 200 + shift, A + 200, n);
            if (!same(A, R, 600)) bad = 1;
        }
    O[g++] = bad;

    /* 2: memset, fills 0, 0xff and others (int c > 255 is truncated) */
    bad = 0;
    {
        const int fills[] = {0, 0xff, 0x5a, 1, 0x180, -1};
        for (unsigned f = 0; f < sizeof fills / sizeof fills[0]; f++)
            for (size_t n = 0; n <= 130; n += 11)
                for (int ofs = 0; ofs < 8; ofs += 3) {
                    fill_rand(A, 200);
                    for (int i = 0; i < 200; i++) R[i] = A[i];
                    for (size_t i = 0; i < n; i++) R[ofs + i] = (uint8_t)fills[f];
                    if (zkvm_memset(A + ofs, fills[f], n) != A + ofs) bad = 1;
                    if (!same(A, R, 200)) bad = 1;
                }
    }
    O[g++] = bad;

    /* 3: memcmp, equal and a difference at every position, both signs */
    bad = 0;
    for (size_t n = 0; n <= 70; n++) {
        fill_rand(A, n + 8);
        for (size_t i = 0; i < n + 8; i++) B[i] = A[i];
        if (zkvm_memcmp(A, B, n) != 0) bad = 1;
        for (size_t k = 0; k < n; k++) {
            uint8_t save = B[k];
            B[k] = (uint8_t)(A[k] + 1 + (rnd8() % 254));             /* any other byte */
            int want = A[k] < B[k] ? -1 : 1;
            if (sign(zkvm_memcmp(A, B, n)) != want) bad = 1;
            if (sign(zkvm_memcmp(B, A, n)) != -want) bad = 1;
            B[k] = save;
        }
    }
    O[g++] = bad;

    /* 4: constant sizes (the immediate forms) */
    bad = 0;
    for (int so = 0; so < 8; so++) {
        fill_rand(A, 200); fill_rand(B, 200);
        for (int i = 0; i < 64; i++) R[i] = B[i];
        ref_move(R + 3, A + so, 32);
        zkvm_memcpy(B + 3, A + so, 32);
        if (!same(B, R, 64)) bad = 1;
        for (int i = 0; i < 56; i++) B[i] = A[so + i];
        if (zkvm_memcmp(A + so, B, 56) != 0) bad = 1;
        B[55] ^= 1;
        if (zkvm_memcmp(A + so, B, 56) == 0) bad = 1;
        zkvm_memset(B, 0, 128);
        for (int i = 0; i < 128; i++) if (B[i] != 0) bad = 1;
        zkvm_memset(B + 1, 0xff, 32);
        for (int i = 1; i < 33; i++) if (B[i] != 0xff) bad = 1;
        if (B[0] != 0 || B[33] != 0) bad = 1;
        zkvm_memset(B, 0x3c, 7);
        for (int i = 0; i < 7; i++) if (B[i] != 0x3c) bad = 1;
        zkvm_memcpy(B, A, 0);
        if (zkvm_memcmp(A, B, 0) != 0) bad = 1;
    }
    {   /* the largest immediate */
        static uint8_t big[2 * 2047 + 16];
        fill_rand(big, sizeof big);
        zkvm_memcpy(big + 2047 + 8, big, 2047);
        for (int i = 0; i < 2047; i++) if (big[2047 + 8 + i] != big[i]) bad = 1;
        if (zkvm_memcmp(big, big + 2047 + 8, 2047) != 0) bad = 1;
    }
    O[g++] = bad;

    /* 5: the weak libc names, called through pointers so they stay calls */
    bad = 0;
    {
        void* (*volatile cpy)(void*, const void*, size_t) = memcpy;
        void* (*volatile mov)(void*, const void*, size_t) = memmove;
        void* (*volatile set)(void*, int, size_t) = memset;
        int (*volatile cmp)(const void*, const void*, size_t) = memcmp;
        for (size_t n = 0; n <= 90; n += 9) {
            fill_rand(A, 200); fill_rand(B, 200);
            for (int i = 0; i < 200; i++) R[i] = B[i];
            ref_move(R + 5, A + 1, n);
            if (cpy(B + 5, A + 1, n) != B + 5 || !same(B, R, 200)) bad = 1;
            for (int i = 0; i < 200; i++) R[i] = A[i];
            ref_move(R + 7, R + 2, n);
            if (mov(A + 7, A + 2, n) != A + 7 || !same(A, R, 200)) bad = 1;
            const int c = (int)(n * 37 + 1);
            for (int i = 0; i < 200; i++) R[i] = B[i];
            for (size_t i = 0; i < n; i++) R[3 + i] = (uint8_t)c;
            if (set(B + 3, c, n) != B + 3 || !same(B, R, 200)) bad = 1;
            if (cmp(B, R, 200) != 0) bad = 1;
            if (n) { R[3 + n - 1] ^= 0x80; if (sign(cmp(B, R, 200)) != sign(B[3 + n - 1] - R[3 + n - 1])) bad = 1; }
        }
    }
    O[g++] = bad;

    /* 6: JUMPDEST bitmap vs the software walk (aligned code of 1..900 bytes) */
    bad = 0;
    {
        static uint64_t code64[128], bm[16], ref[16];
        uint8_t* code = (uint8_t*)code64;
        for (size_t n = 1; n <= 900; n += (n < 70 ? 1 : 29)) {
            for (size_t i = 0; i < n; i++) {
                uint8_t r = rnd8();
                code[i] = r < 60 ? 0x5b : (r < 140 ? (uint8_t)(0x60 + (r & 31)) : r);
            }
            for (int w = 0; w < 16; w++) bm[w] = 0xdeadbeefdeadbeefULL;
            ref_jumpdest(code, n, ref);
            if (zkvm_evm_jumpdest_bitmap(code, n, bm) != ZKVM_EOK) bad = 1;
            for (size_t w = 0; w < (n + 63) / 64; w++) if (bm[w] != ref[w]) bad = 1;
        }
    }
    O[g++] = bad;

    /* 7: JUMPDEST refusals: empty, unaligned code, unaligned bitmap -> EFAIL, no write */
    bad = 0;
    {
        static uint64_t code64[8], bm[4];
        uint8_t* code = (uint8_t*)code64;
        for (int i = 0; i < 64; i++) code[i] = 0x5b;
        for (int w = 0; w < 4; w++) bm[w] = 0x1234;
        if (zkvm_evm_jumpdest_bitmap(code, 0, bm) != ZKVM_EFAIL) bad = 1;
        if (zkvm_evm_jumpdest_bitmap(code + 1, 32, bm) != ZKVM_EFAIL) bad = 1;
        if (zkvm_evm_jumpdest_bitmap(code, 32, (uint64_t*)((uint8_t*)bm + 4)) != ZKVM_EFAIL) bad = 1;
        for (int w = 0; w < 4; w++) if (bm[w] != 0x1234) bad = 1;
    }
    O[g++] = bad;

    O[g] = 1;
    return 0;
}
