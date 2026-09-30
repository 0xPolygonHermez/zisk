// Differential test and microbenchmark of the lib-c precompile entry points, called exactly as
// the ASM emulator calls them (emulator-asm/src/emu.c), e.g. with the result overwriting p1.
// It checks that a change to lib-c keeps every result byte-identical: records are generated with
// a reference build of the library and then checked against the modified one. test/difftest.sh
// does both steps, building the reference from a git commit.
//
//   difftest gen <dir> [filter]    write reference records for every op
//   difftest check <dir> [filter]  recompute records and compare them with <dir>
//   difftest bench [filter]        cycles and retired instructions per call, canonical inputs
//                                  (x86_64 Linux only)
//
// filter selects the ops whose name contains it. Every op runs in two input modes:
//   canon: canonical field elements (< p), the normal case for guest programs
//   any:   also non-canonical values (>= p), edge values, and outputs aliasing inputs
// A record is [inputs..., return code, outputs...], so a mismatch shows the input that caused it.
// Inputs are generated deterministically, so records are comparable across builds and machines.

#include <gmpxx.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <string>
#include <vector>
#include <functional>
#include "fcall/fcall.hpp"

#if defined(__x86_64__) && defined(__linux__)
#define DIFFTEST_BENCH
#include <x86intrin.h>
#include <unistd.h>
#include <sys/syscall.h>
#include <sys/mman.h>
#include <linux/perf_event.h>
#endif

extern "C" {
void zisk_keccakf1600(uint64_t state[25]);
void blake2b_round(uint64_t v[16], const uint64_t m[16], uint64_t round);
void blake2s_f(uint32_t v[16], const uint32_t m[16]);
void blake3_f(uint32_t v[16], const uint32_t m[16]);
void poseidon2_hash(uint64_t *state);
void poseidon1_hash(uint64_t *state);
int Arith256(const uint64_t *a, const uint64_t *b, const uint64_t *c, uint64_t *dl, uint64_t *dh);
int Arith384Mod(const uint64_t *a, const uint64_t *b, const uint64_t *c, const uint64_t *module, uint64_t *d);
int Add256(const uint64_t *a, const uint64_t *b, const uint64_t cin, uint64_t *c);
int AddPointEcP(const uint64_t dbl, const uint64_t *p1, const uint64_t *p2, uint64_t *p3);
int AddPointEc(uint64_t dbl, const uint64_t *x1, const uint64_t *y1, const uint64_t *x2, const uint64_t *y2, uint64_t *x3, uint64_t *y3);
int secp256r1_add_point_ecp(const uint64_t dbl, const uint64_t *p1, const uint64_t *p2, uint64_t *p3);
int BN254CurveAddP(const uint64_t *p1, const uint64_t *p2, uint64_t *p3);
int BN254CurveDblP(const uint64_t *p1, uint64_t *p2);
int BN254ComplexAddP(const uint64_t *p1, const uint64_t *p2, uint64_t *p3);
int BN254ComplexSubP(const uint64_t *p1, const uint64_t *p2, uint64_t *p3);
int BN254ComplexMulP(const uint64_t *p1, const uint64_t *p2, uint64_t *p3);
int BLS12_381CurveAddP(const uint64_t *p1, const uint64_t *p2, uint64_t *p3);
int BLS12_381CurveDblP(const uint64_t *p1, uint64_t *p2);
int BLS12_381ComplexAddP(const uint64_t *p1, const uint64_t *p2, uint64_t *p3);
int BLS12_381ComplexSubP(const uint64_t *p1, const uint64_t *p2, uint64_t *p3);
int BLS12_381ComplexMulP(const uint64_t *p1, const uint64_t *p2, uint64_t *p3);
int BabyJubJubAddP(const uint64_t *p1, const uint64_t *p2, uint64_t *p3);
}

/**********/
/* TIMING */
/**********/

#ifdef DIFFTEST_BENCH

// Per-call cost of the op itself, read with rdtsc and rdpmc (see emulator-asm/src/emu.hpp)
static struct perf_event_mmap_page *perf_page = NULL;
static uint64_t acc_cycles = 0, acc_instructions = 0, overhead_cycles = 0, overhead_instructions = 0;

static inline uint64_t rdpmc_instructions() {
    struct perf_event_mmap_page *pc = perf_page;
    if (pc == NULL) return 0;
    uint32_t seq;
    uint64_t count;
    do {
        seq = pc->lock;
        __asm__ volatile("" ::: "memory");
        uint32_t index = pc->index;
        count = pc->offset;
        if (pc->cap_user_rdpmc && index != 0) {
            uint32_t lo, hi;
            __asm__ volatile("rdpmc" : "=a"(lo), "=d"(hi) : "c"(index - 1));
            int64_t pmc = (int64_t)(((uint64_t)hi << 32) | lo);
            uint16_t shift = 64 - pc->pmc_width;
            count += (pmc << shift) >> shift;
        }
        __asm__ volatile("" ::: "memory");
    } while (pc->lock != seq);
    return count;
}

struct Sample { uint64_t c, i; };
static inline Sample sample() {
    Sample s;
    _mm_lfence(); s.c = __rdtsc(); _mm_lfence(); s.i = rdpmc_instructions(); _mm_lfence();
    return s;
}
static inline void accumulate(const Sample &a, const Sample &b) {
    uint64_t c = b.c - a.c, i = b.i - a.i;
    acc_cycles += c > overhead_cycles ? c - overhead_cycles : 0;
    acc_instructions += i > overhead_instructions ? i - overhead_instructions : 0;
}
#define TIMED(x) ({ Sample _s0 = sample(); auto _r = (x); Sample _s1 = sample(); accumulate(_s0, _s1); _r; })
#define TIMEDV(x) do { Sample _s0 = sample(); x; Sample _s1 = sample(); accumulate(_s0, _s1); } while (0)

#else

#define TIMED(x) (x)
#define TIMEDV(x) x

#endif

/*********/
/* INPUT */
/*********/

struct Rng {
    uint64_t s;
    uint64_t next() {
        uint64_t z = (s += 0x9e3779b97f4a7c15ULL);
        z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9ULL;
        z = (z ^ (z >> 27)) * 0x94d049bb133111ebULL;
        return z ^ (z >> 31);
    }
    uint64_t below(uint64_t n) { return next() % n; }
};

static const mpz_class P_SECP256K1("fffffffffffffffffffffffffffffffffffffffffffffffffffffffefffffc2f", 16);
static const mpz_class N_SECP256K1("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141", 16);
static const mpz_class P_SECP256R1("ffffffff00000001000000000000000000000000ffffffffffffffffffffffff", 16);
static const mpz_class N_SECP256R1("ffffffff00000000ffffffffffffffffbce6faada7179e84f3b9cac2fc632551", 16);
static const mpz_class P_BN254("30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47", 16);
static const mpz_class R_BN254("30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001", 16);
static const mpz_class P_BLS12_381("1a0111ea397fe69a4b1ba7b6434bacd764774b84f38512bf6730d2a0f6b0f6241eabfffeb153ffffb9feffffffffaaab", 16);
static const mpz_class P_GOLDILOCKS("ffffffff00000001", 16);

static void to_limbs(const mpz_class &v, uint64_t *out, int n) {
    memset(out, 0, n * 8);
    mpz_class m = v;
    mpz_class mask = (mpz_class(1) << (64 * n)) - 1;
    m &= mask;
    mpz_export(out, NULL, -1, 8, -1, 0, m.get_mpz_t());
}

static mpz_class from_limbs(const uint64_t *a, int n) {
    mpz_class v;
    mpz_import(v.get_mpz_t(), n, -1, 8, -1, 0, a);
    return v;
}

static void rand_limbs(Rng &r, uint64_t *out, int n) {
    for (int i = 0; i < n; i++) out[i] = r.next();
}

// A field element of n limbs; canonical ones are < p, otherwise any n-limb value may come out
static void rand_fe(Rng &r, const mpz_class &p, int n, uint64_t *out, bool canon, bool nonzero = false) {
    for (;;) {
        uint64_t k = r.below(100);
        mpz_class v;
        if (k < 75) {
            rand_limbs(r, out, n);
            v = from_limbs(out, n) % p;
        } else if (k < 85) {
            static const int64_t edges[] = {0, 1, 2, 3, -1, -2};
            int64_t e = edges[r.below(6)];
            v = e >= 0 ? mpz_class((unsigned long)e) : p + e;
        } else if (k < 90) {
            v = mpz_class((unsigned long)r.next());
        } else {
            if (canon) {
                rand_limbs(r, out, n);
                v = from_limbs(out, n) % p;
            } else {
                // Non-canonical: p..p+small, 2^bits-1, or uniform over all n limbs
                uint64_t j = r.below(4);
                if (j == 0) v = p + mpz_class((unsigned long)r.below(3));
                else if (j == 1) v = (mpz_class(1) << (64 * n)) - 1;
                else { rand_limbs(r, out, n); v = from_limbs(out, n); }
            }
        }
        if (nonzero && (v % p) == 0) continue;
        to_limbs(v, out, n);
        return;
    }
}

/**********/
/* RECORD */
/**********/

struct Record {
    std::vector<uint64_t> w;
    void put(const uint64_t *a, int n) { w.insert(w.end(), a, a + n); }
    void put(const uint32_t *a, int n) { for (int i = 0; i < n; i++) w.push_back(a[i]); }
    void put(uint64_t v) { w.push_back(v); }
};

struct Op {
    std::string name;
    int n;  // cases per mode
    // Generates the inputs of the next case from r, records them, runs the op and records the
    // results; the op call itself is wrapped in TIMED()/TIMEDV() for the benchmark
    std::function<void(Rng &r, bool canon, Record &rec)> run;
};

static std::vector<Op> ops;

// Point ops: p1 is the in/out buffer, as in the emulator
static void add_point_op(const char *name, const mpz_class &p, int limbs, bool binary,
                         std::function<int(uint64_t *, uint64_t *)> f, bool nonzero_y = false) {
    ops.push_back({name, 20000, [=](Rng &r, bool canon, Record &rec) {
        uint64_t p1[12], p2[12];
        rand_fe(r, p, limbs, p1, canon);
        rand_fe(r, p, limbs, p1 + limbs, canon, nonzero_y);
        if (binary) {
            rand_fe(r, p, limbs, p2, canon);
            rand_fe(r, p, limbs, p2 + limbs, canon);
            // Some cases with equal x coordinates
            if (r.below(100) == 0) memcpy(p2, p1, limbs * 8);
            rec.put(p2, 2 * limbs);
        }
        rec.put(p1, 2 * limbs);
        int rc = TIMED(f(p1, p2));
        rec.put((uint64_t)(int64_t)rc);
        // On error (e.g. x1 == x2) the output is undefined, and the emulator aborts
        if (rc == 0) rec.put(p1, 2 * limbs);
    }});
}

// Fcall ops: params are generated by gen, which returns params_size
static void add_fcall_op(const char *name, uint64_t id, int n,
                         std::function<uint64_t(Rng &, bool, uint64_t *)> gen) {
    ops.push_back({std::string("fcall_") + name, n, [=](Rng &r, bool canon, Record &rec) {
        static FcallContext ctx;
        ctx.function_id = id;
        ctx.params_max_size = FCALL_PARAMS_MAX_SIZE;
        ctx.params_size = gen(r, canon, ctx.params);
        ctx.result_max_size = FCALL_RESULT_MAX_SIZE;
        ctx.result_size = 0;
        rec.put(ctx.params, (int)ctx.params_size);
        int rc = TIMED(Fcall(&ctx));
        rec.put((uint64_t)(int64_t)rc);
        rec.put(ctx.result_size);
        rec.put(ctx.result, (int)(ctx.result_size < 64 ? ctx.result_size : 64));
    }});
}

static void register_ops() {
    ops.push_back({"keccakf", 20000, [](Rng &r, bool, Record &rec) {
        uint64_t s[25];
        rand_limbs(r, s, 25);
        rec.put(s, 25);
        TIMEDV(zisk_keccakf1600(s));
        rec.put(s, 25);
    }});
    ops.push_back({"blake2b_round", 20000, [](Rng &r, bool, Record &rec) {
        uint64_t v[16], m[16];
        rand_limbs(r, v, 16);
        rand_limbs(r, m, 16);
        uint64_t round = r.below(12);
        rec.put(v, 16); rec.put(m, 16); rec.put(round);
        TIMEDV(blake2b_round(v, m, round));
        rec.put(v, 16);
    }});
    ops.push_back({"blake2s_f", 20000, [](Rng &r, bool, Record &rec) {
        uint32_t v[16], m[16];
        for (int i = 0; i < 16; i++) { v[i] = (uint32_t)r.next(); m[i] = (uint32_t)r.next(); }
        rec.put(v, 16); rec.put(m, 16);
        TIMEDV(blake2s_f(v, m));
        rec.put(v, 16);
    }});
    ops.push_back({"blake3_f", 20000, [](Rng &r, bool, Record &rec) {
        uint32_t v[16], m[16];
        for (int i = 0; i < 16; i++) { v[i] = (uint32_t)r.next(); m[i] = (uint32_t)r.next(); }
        rec.put(v, 16); rec.put(m, 16);
        TIMEDV(blake3_f(v, m));
        rec.put(v, 16);
    }});
    ops.push_back({"poseidon2", 20000, [](Rng &r, bool canon, Record &rec) {
        uint64_t s[16];
        for (int i = 0; i < 16; i++) rand_fe(r, P_GOLDILOCKS, 1, &s[i], canon);
        rec.put(s, 16);
        TIMEDV(poseidon2_hash(s));
        rec.put(s, 16);
    }});
    ops.push_back({"poseidon1", 20000, [](Rng &r, bool canon, Record &rec) {
        uint64_t s[16];
        for (int i = 0; i < 16; i++) rand_fe(r, P_GOLDILOCKS, 1, &s[i], canon);
        rec.put(s, 16);
        TIMEDV(poseidon1_hash(s));
        rec.put(s, 16);
    }});
    ops.push_back({"arith256", 20000, [](Rng &r, bool canon, Record &rec) {
        uint64_t a[4], b[4], c[4], dl[4], dh[4];
        rand_fe(r, mpz_class(1) << 256, 4, a, true);
        rand_fe(r, mpz_class(1) << 256, 4, b, true);
        rand_fe(r, mpz_class(1) << 256, 4, c, true);
        rec.put(a, 4); rec.put(b, 4); rec.put(c, 4);
        int rc;
        if (canon) {
            rc = TIMED(Arith256(a, b, c, dl, dh));
            rec.put((uint64_t)(int64_t)rc); rec.put(dl, 4); rec.put(dh, 4);
        } else {
            // Outputs aliasing the inputs
            rc = Arith256(a, b, c, a, b);
            rec.put((uint64_t)(int64_t)rc); rec.put(a, 4); rec.put(b, 4);
        }
    }});
    ops.push_back({"arith384_mod", 20000, [](Rng &r, bool canon, Record &rec) {
        uint64_t a[6], b[6], c[6], m[6], d[6];
        if (canon || r.below(2) == 0) {
            // The usual module; in "any" mode with non-canonical a, b and c
            to_limbs(P_BLS12_381, m, 6);
        } else {
            rand_fe(r, mpz_class(1) << 384, 6, m, true, true);
        }
        mpz_class mod = from_limbs(m, 6);
        rand_fe(r, mod, 6, a, canon);
        rand_fe(r, mod, 6, b, canon);
        rand_fe(r, mod, 6, c, canon);
        rec.put(a, 6); rec.put(b, 6); rec.put(c, 6); rec.put(m, 6);
        int rc;
        if (canon) {
            rc = TIMED(Arith384Mod(a, b, c, m, d));
            rec.put((uint64_t)(int64_t)rc); rec.put(d, 6);
        } else {
            rc = Arith384Mod(a, b, c, m, a);
            rec.put((uint64_t)(int64_t)rc); rec.put(a, 6);
        }
    }});
    ops.push_back({"add256", 20000, [](Rng &r, bool canon, Record &rec) {
        uint64_t a[4], b[4], c[4];
        rand_fe(r, mpz_class(1) << 256, 4, a, true);
        rand_fe(r, mpz_class(1) << 256, 4, b, true);
        uint64_t cin = r.below(2);
        rec.put(a, 4); rec.put(b, 4); rec.put(cin);
        int rc = canon ? TIMED(Add256(a, b, cin, c)) : Add256(a, b, cin, a);
        rec.put((uint64_t)(int64_t)rc); rec.put(canon ? c : a, 4);
    }});

    add_point_op("secp256k1_add", P_SECP256K1, 4, true, [](uint64_t *p1, uint64_t *p2) { return AddPointEcP(0, p1, p2, p1); });
    add_point_op("secp256k1_dbl", P_SECP256K1, 4, false, [](uint64_t *p1, uint64_t *) { return AddPointEcP(1, p1, NULL, p1); });
    // AddPointEc(), with separate coordinates, as called from Rust; in "any" mode the result
    // overwrites the first point
    for (int dbl = 0; dbl < 2; dbl++) {
        ops.push_back({dbl ? "secp256k1_dbl_xy" : "secp256k1_add_xy", 20000, [dbl](Rng &r, bool canon, Record &rec) {
            uint64_t x1[4], y1[4], x2[4], y2[4], x3[4], y3[4];
            rand_fe(r, P_SECP256K1, 4, x1, canon);
            rand_fe(r, P_SECP256K1, 4, y1, canon, dbl);
            rand_fe(r, P_SECP256K1, 4, x2, canon);
            rand_fe(r, P_SECP256K1, 4, y2, canon);
            if (r.below(100) == 0) memcpy(x2, x1, sizeof(x1));
            rec.put(x1, 4); rec.put(y1, 4);
            if (!dbl) { rec.put(x2, 4); rec.put(y2, 4); }
            uint64_t *rx = canon ? x3 : x1, *ry = canon ? y3 : y1;
            int rc = TIMED(AddPointEc(dbl, x1, y1, x2, y2, rx, ry));
            rec.put((uint64_t)(int64_t)rc);
            if (rc == 0) { rec.put(rx, 4); rec.put(ry, 4); }
        }});
    }
    add_point_op("secp256r1_add", P_SECP256R1, 4, true, [](uint64_t *p1, uint64_t *p2) { return secp256r1_add_point_ecp(0, p1, p2, p1); });
    add_point_op("secp256r1_dbl", P_SECP256R1, 4, false, [](uint64_t *p1, uint64_t *) { return secp256r1_add_point_ecp(1, p1, NULL, p1); });
    add_point_op("bn254_curve_add", P_BN254, 4, true, [](uint64_t *p1, uint64_t *p2) { return BN254CurveAddP(p1, p2, p1); });
    add_point_op("bn254_curve_dbl", P_BN254, 4, false, [](uint64_t *p1, uint64_t *) { return BN254CurveDblP(p1, p1); });
    add_point_op("bn254_complex_add", P_BN254, 4, true, [](uint64_t *p1, uint64_t *p2) { return BN254ComplexAddP(p1, p2, p1); });
    add_point_op("bn254_complex_sub", P_BN254, 4, true, [](uint64_t *p1, uint64_t *p2) { return BN254ComplexSubP(p1, p2, p1); });
    add_point_op("bn254_complex_mul", P_BN254, 4, true, [](uint64_t *p1, uint64_t *p2) { return BN254ComplexMulP(p1, p2, p1); });
    add_point_op("bls12_381_curve_add", P_BLS12_381, 6, true, [](uint64_t *p1, uint64_t *p2) { return BLS12_381CurveAddP(p1, p2, p1); });
    add_point_op("bls12_381_curve_dbl", P_BLS12_381, 6, false, [](uint64_t *p1, uint64_t *) { return BLS12_381CurveDblP(p1, p1); });
    add_point_op("bls12_381_complex_add", P_BLS12_381, 6, true, [](uint64_t *p1, uint64_t *p2) { return BLS12_381ComplexAddP(p1, p2, p1); });
    add_point_op("bls12_381_complex_sub", P_BLS12_381, 6, true, [](uint64_t *p1, uint64_t *p2) { return BLS12_381ComplexSubP(p1, p2, p1); });
    add_point_op("bls12_381_complex_mul", P_BLS12_381, 6, true, [](uint64_t *p1, uint64_t *p2) { return BLS12_381ComplexMulP(p1, p2, p1); });
    add_point_op("babyjubjub_add", R_BN254, 4, true, [](uint64_t *p1, uint64_t *p2) { return BabyJubJubAddP(p1, p2, p1); });

    auto fe = [](const mpz_class &p, int limbs, int count, bool nonzero) {
        return [=](Rng &r, bool canon, uint64_t *params) -> uint64_t {
            for (int i = 0; i < count; i++) rand_fe(r, p, limbs, params + i * limbs, canon, nonzero);
            return (uint64_t)(limbs * count);
        };
    };
    add_fcall_op("secp256k1_fp_inv", FCALL_SECP256K1_FP_INV_ID, 20000, fe(P_SECP256K1, 4, 1, true));
    add_fcall_op("secp256k1_fn_inv", FCALL_SECP256K1_FN_INV_ID, 20000, fe(N_SECP256K1, 4, 1, true));
    add_fcall_op("secp256k1_fp_sqrt", FCALL_SECP256K1_FP_SQRT_ID, 5000, [](Rng &r, bool canon, uint64_t *params) -> uint64_t {
        rand_fe(r, P_SECP256K1, 4, params, canon);
        params[4] = r.below(2);
        return 5;
    });
    add_fcall_op("secp256k1_glv_decompose", FCALL_SECP256K1_GLV_DECOMPOSE_ID, 20000, fe(N_SECP256K1, 4, 1, false));
    add_fcall_op("secp256r1_fn_inv", FCALL_SECP256R1_FN_INV_ID, 20000, fe(N_SECP256R1, 4, 1, true));
    add_fcall_op("bn254_fp_inv", FCALL_BN254_FP_INV_ID, 20000, fe(P_BN254, 4, 1, true));
    add_fcall_op("bn254_fp2_inv", FCALL_BN254_FP2_INV_ID, 20000, fe(P_BN254, 4, 2, true));
    add_fcall_op("bn254_twist_add_line_coeffs", FCALL_BN254_TWIST_ADD_LINE_COEFFS_ID, 20000, fe(P_BN254, 4, 8, true));
    add_fcall_op("bn254_twist_dbl_line_coeffs", FCALL_BN254_TWIST_DBL_LINE_COEFFS_ID, 20000, fe(P_BN254, 4, 4, true));
    add_fcall_op("bls12_381_fp_inv", FCALL_BLS12_381_FP_INV_ID, 20000, fe(P_BLS12_381, 6, 1, true));
    add_fcall_op("bls12_381_fp_sqrt", FCALL_BLS12_381_FP_SQRT_ID, 2000, fe(P_BLS12_381, 6, 1, false));
    add_fcall_op("bls12_381_fp2_inv", FCALL_BLS12_381_FP2_INV_ID, 20000, fe(P_BLS12_381, 6, 2, true));
    add_fcall_op("bls12_381_fp2_sqrt", FCALL_BLS12_381_FP2_SQRT_ID, 1000, fe(P_BLS12_381, 6, 2, true));
    add_fcall_op("bls12_381_twist_add_line_coeffs", FCALL_BLS12_381_TWIST_ADD_LINE_COEFFS_ID, 20000, fe(P_BLS12_381, 6, 8, true));
    add_fcall_op("bls12_381_twist_dbl_line_coeffs", FCALL_BLS12_381_TWIST_DBL_LINE_COEFFS_ID, 20000, fe(P_BLS12_381, 6, 4, true));
    add_fcall_op("bin_decomp", FCALL_BIN_DECOMP_ID, 20000, [](Rng &r, bool, uint64_t *params) -> uint64_t {
        uint64_t len = 1 + r.below(6);
        params[0] = len;
        for (uint64_t i = 0; i < len; i++) params[1 + i] = r.below(4) == 0 ? 0 : r.next() >> r.below(64);
        return 1 + len;
    });
    add_fcall_op("msb_pos_256", FCALL_MSB_POS_256_ID, 20000, [](Rng &r, bool, uint64_t *params) -> uint64_t {
        uint64_t n = 1 + r.below(3);
        params[0] = n;
        for (uint64_t i = 0; i < n * 4; i++) params[1 + i] = r.below(3) == 0 ? 0 : r.next() >> r.below(64);
        params[1 + r.below(n * 4)] |= 1;
        return 1 + n * 4;
    });
    add_fcall_op("msb_pos_384", FCALL_MSB_POS_384_ID, 20000, [](Rng &r, bool, uint64_t *params) -> uint64_t {
        uint64_t n = 1 + r.below(3);
        params[0] = n;
        for (uint64_t i = 0; i < n * 6; i++) params[1 + i] = r.below(3) == 0 ? 0 : r.next() >> r.below(64);
        params[1 + r.below(n * 6)] |= 1;
        return 1 + n * 6;
    });
    add_fcall_op("uint256_div", FCALL_UINT256_DIV_ID, 20000, [](Rng &r, bool, uint64_t *params) -> uint64_t {
        rand_fe(r, mpz_class(1) << 256, 4, params, true);
        rand_fe(r, mpz_class(1) << (64 * (1 + r.below(4))), 4, params + 4, true, true);
        return 8;
    });
    add_fcall_op("uint256_inv", FCALL_UINT256_INV_ID, 20000, fe(mpz_class(1) << 256, 4, 1, false));
    add_fcall_op("uint256_inv_mod", FCALL_UINT256_INV_MOD_ID, 20000, [](Rng &r, bool, uint64_t *params) -> uint64_t {
        rand_fe(r, mpz_class(1) << 256, 4, params, true);
        rand_fe(r, mpz_class(1) << 256, 4, params + 4, true, true);
        return 8;
    });
    add_fcall_op("bigint_div", FCALL_BIGINT_DIV_ID, 20000, [](Rng &r, bool, uint64_t *params) -> uint64_t {
        uint64_t la = 1 + r.below(8), lb = 1 + r.below(6);
        params[0] = la;
        rand_limbs(r, params + 1, (int)la);
        params[1 + la] = lb;
        do { rand_limbs(r, params + 2 + la, (int)lb); params[2 + la + lb - 1] >>= r.below(64); }
        while (from_limbs(params + 2 + la, (int)lb) == 0);
        return 2 + la + lb;
    });
}

/********/
/* MAIN */
/********/

static bool matches(const Op &op, const char *filter) {
    return filter == NULL || op.name.find(filter) != std::string::npos;
}

// FNV-1a, a fixed hash so that every build derives the same seeds from the op names
static uint64_t name_hash(const std::string &name) {
    uint64_t h = 0xcbf29ce484222325ULL;
    for (unsigned char ch : name) { h ^= ch; h *= 0x100000001b3ULL; }
    return h;
}

static std::vector<uint64_t> run_mode(const Op &op, bool canon, std::vector<size_t> &offsets) {
    std::vector<uint64_t> all;
    Rng r{0x5eed0000ULL + name_hash(op.name) + (canon ? 0 : 1)};
    for (int i = 0; i < op.n; i++) {
        Record rec;
        op.run(r, canon, rec);
        offsets.push_back(all.size());
        all.push_back(rec.w.size());
        all.insert(all.end(), rec.w.begin(), rec.w.end());
    }
    return all;
}

static int gen_or_check(const char *dir, bool check, const char *filter) {
    int failures = 0;
    for (const Op &op : ops) {
        if (!matches(op, filter)) continue;
        for (int m = 0; m < 2; m++) {
            bool canon = m == 0;
            std::string path = std::string(dir) + "/" + op.name + (canon ? ".canon" : ".any") + ".bin";
            std::vector<size_t> offsets;
            std::vector<uint64_t> got = run_mode(op, canon, offsets);
            if (!check) {
                FILE *f = fopen(path.c_str(), "wb");
                if (f == NULL || fwrite(got.data(), 8, got.size(), f) != got.size()) {
                    fprintf(stderr, "failed writing %s\n", path.c_str());
                    return -1;
                }
                fclose(f);
                continue;
            }
            FILE *f = fopen(path.c_str(), "rb");
            if (f == NULL) { fprintf(stderr, "%-40s missing %s\n", op.name.c_str(), path.c_str()); failures++; continue; }
            std::vector<uint64_t> ref;
            uint64_t w;
            while (fread(&w, 8, 1, f) == 1) ref.push_back(w);
            fclose(f);
            int bad = 0;
            for (size_t i = 0; i < offsets.size(); i++) {
                size_t o = offsets[i];
                size_t len = got[o] + 1;
                if (o + len <= ref.size() && memcmp(&got[o], &ref[o], len * 8) == 0) continue;
                if (bad < 2) {
                    fprintf(stderr, "  %s %s case %zu mismatch\n    ref:", op.name.c_str(), canon ? "canon" : "any", i);
                    for (size_t k = 1; k < len && o + k < ref.size(); k++) fprintf(stderr, " %lx", ref[o + k]);
                    fprintf(stderr, "\n    got:");
                    for (size_t k = 1; k < len; k++) fprintf(stderr, " %lx", got[o + k]);
                    fprintf(stderr, "\n");
                }
                bad++;
            }
            fprintf(stderr, "%-40s %-5s %s (%d/%d mismatches)\n", op.name.c_str(), canon ? "canon" : "any", bad ? "FAIL" : "ok", bad, op.n);
            if (bad) failures++;
        }
    }
    return failures;
}

#ifdef DIFFTEST_BENCH
static void bench(const char *filter) {
    struct perf_event_attr attr;
    memset(&attr, 0, sizeof(attr));
    attr.type = PERF_TYPE_HARDWARE;
    attr.size = sizeof(attr);
    attr.config = PERF_COUNT_HW_INSTRUCTIONS;
    attr.exclude_kernel = 1;
    attr.exclude_hv = 1;
    int fd = syscall(SYS_perf_event_open, &attr, 0, -1, -1, 0);
    if (fd >= 0) {
        void *page = mmap(NULL, sysconf(_SC_PAGESIZE), PROT_READ, MAP_SHARED, fd, 0);
        if (page != MAP_FAILED) perf_page = (struct perf_event_mmap_page *)page;
    }
    if (perf_page == NULL) fprintf(stderr, "warning: no instructions counter\n");

    // Calibrate the cost of an empty measurement
    overhead_cycles = overhead_instructions = UINT64_MAX;
    for (int k = 0; k < 10000; k++) {
        Sample a = sample(), b = sample();
        if (b.c - a.c < overhead_cycles) overhead_cycles = b.c - a.c;
        if (b.i - a.i < overhead_instructions) overhead_instructions = b.i - a.i;
    }

    fprintf(stderr, "%-40s %10s %10s\n", "op (canonical inputs, best of 3)", "cyc/call", "instr/call");
    for (const Op &op : ops) {
        if (!matches(op, filter)) continue;
        double best_cyc = 1e300, best_ins = 1e300;
        for (int rep = 0; rep < 3; rep++) {
            std::vector<size_t> offsets;
            acc_cycles = acc_instructions = 0;
            run_mode(op, true, offsets);
            if ((double)acc_cycles / op.n < best_cyc) best_cyc = (double)acc_cycles / op.n;
            if ((double)acc_instructions / op.n < best_ins) best_ins = (double)acc_instructions / op.n;
        }
        fprintf(stderr, "%-40s %10.0f %10.0f\n", op.name.c_str(), best_cyc, best_ins);
    }
}
#else
static void bench(const char *) {
    fprintf(stderr, "bench is only available on x86_64 Linux\n");
}
#endif

int main(int argc, char **argv) {
    register_ops();
    if (argc >= 3 && strcmp(argv[1], "gen") == 0) {
        return gen_or_check(argv[2], false, argc > 3 ? argv[3] : NULL) == 0 ? 0 : 1;
    }
    if (argc >= 3 && strcmp(argv[1], "check") == 0) {
        int failures = gen_or_check(argv[2], true, argc > 3 ? argv[3] : NULL);
        fprintf(stderr, failures ? "\nFAILED: %d op/mode combinations differ\n" : "\nALL OK\n", failures);
        return failures ? 1 : 0;
    }
    if (argc >= 2 && strcmp(argv[1], "bench") == 0) {
        bench(argc > 2 ? argv[2] : NULL);
        return 0;
    }
    fprintf(stderr, "usage: difftest gen|check <dir> [filter] | bench [filter]\n");
    return 2;
}
