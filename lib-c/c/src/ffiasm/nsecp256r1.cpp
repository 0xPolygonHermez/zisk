#include "nsecp256r1.hpp"
#include <stdio.h>
#include <stdlib.h>
#include <gmp.h>
#include <assert.h>
#include <string>


static mpz_t q;
static mpz_t zero;
static mpz_t one;
static mpz_t mask;
static size_t nBits;
static bool initialized = false;


void nSecp256r1_toMpz(mpz_t r, PnSecp256r1Element pE) {
    nSecp256r1Element tmp;
    nSecp256r1_toNormal(&tmp, pE);
    if (!(tmp.type & nSecp256r1_LONG)) {
        mpz_set_si(r, tmp.shortVal);
        if (tmp.shortVal<0) {
            mpz_add(r, r, q);
        }
    } else {
        mpz_import(r, nSecp256r1_N64, -1, 8, -1, 0, (const void *)tmp.longVal);
    }
}

void nSecp256r1_fromMpz(PnSecp256r1Element pE, mpz_t v) {
    if (mpz_fits_sint_p(v)) {
        pE->type = nSecp256r1_SHORT;
        pE->shortVal = mpz_get_si(v);
    } else {
        pE->type = nSecp256r1_LONG;
        for (int i=0; i<nSecp256r1_N64; i++) pE->longVal[i] = 0;
        mpz_export((void *)(pE->longVal), NULL, -1, 8, -1, 0, v);
    }
}


bool nSecp256r1_init() {
    if (initialized) return false;
    initialized = true;
    mpz_init(q);
    mpz_import(q, nSecp256r1_N64, -1, 8, -1, 0, (const void *)nSecp256r1_q.longVal);
    mpz_init_set_ui(zero, 0);
    mpz_init_set_ui(one, 1);
    nBits = mpz_sizeinbase (q, 2);
    mpz_init(mask);
    mpz_mul_2exp(mask, one, nBits);
    mpz_sub(mask, mask, one);
    return true;
}

void nSecp256r1_str2element(PnSecp256r1Element pE, char const *s) {
    mpz_t mr;
    mpz_init_set_str(mr, s, 10);
    mpz_fdiv_r(mr, mr, q);
    nSecp256r1_fromMpz(pE, mr);
    mpz_clear(mr);
}

char *nSecp256r1_element2str(PnSecp256r1Element pE) {
    nSecp256r1Element tmp;
    mpz_t r;
    if (!(pE->type & nSecp256r1_LONG)) {
        if (pE->shortVal>=0) {
            char *r = new char[32];
            sprintf(r, "%d", pE->shortVal);
            return r;
        } else {
            mpz_init_set_si(r, pE->shortVal);
            mpz_add(r, r, q);
        }
    } else {
        nSecp256r1_toNormal(&tmp, pE);
        mpz_init(r);
        mpz_import(r, nSecp256r1_N64, -1, 8, -1, 0, (const void *)tmp.longVal);
    }
    char *res = mpz_get_str (0, 10, r);
    mpz_clear(r);
    return res;
}

void nSecp256r1_idiv(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b) {
    mpz_t ma;
    mpz_t mb;
    mpz_t mr;
    mpz_init(ma);
    mpz_init(mb);
    mpz_init(mr);

    nSecp256r1_toMpz(ma, a);
    // char *s1 = mpz_get_str (0, 10, ma);
    // printf("s1 %s\n", s1);
    nSecp256r1_toMpz(mb, b);
    // char *s2 = mpz_get_str (0, 10, mb);
    // printf("s2 %s\n", s2);
    mpz_fdiv_q(mr, ma, mb);
    // char *sr = mpz_get_str (0, 10, mr);
    // printf("r %s\n", sr);
    nSecp256r1_fromMpz(r, mr);

    mpz_clear(ma);
    mpz_clear(mb);
    mpz_clear(mr);
}

void nSecp256r1_mod(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b) {
    mpz_t ma;
    mpz_t mb;
    mpz_t mr;
    mpz_init(ma);
    mpz_init(mb);
    mpz_init(mr);

    nSecp256r1_toMpz(ma, a);
    nSecp256r1_toMpz(mb, b);
    mpz_fdiv_r(mr, ma, mb);
    nSecp256r1_fromMpz(r, mr);

    mpz_clear(ma);
    mpz_clear(mb);
    mpz_clear(mr);
}

void nSecp256r1_pow(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b) {
    mpz_t ma;
    mpz_t mb;
    mpz_t mr;
    mpz_init(ma);
    mpz_init(mb);
    mpz_init(mr);

    nSecp256r1_toMpz(ma, a);
    nSecp256r1_toMpz(mb, b);
    mpz_powm(mr, ma, mb, q);
    nSecp256r1_fromMpz(r, mr);

    mpz_clear(ma);
    mpz_clear(mb);
    mpz_clear(mr);
}

void nSecp256r1_inv(PnSecp256r1Element r, PnSecp256r1Element a) {
    mpz_t ma;
    mpz_t mr;
    mpz_init(ma);
    mpz_init(mr);

    nSecp256r1_toMpz(ma, a);
    mpz_invert(mr, ma, q);
    nSecp256r1_fromMpz(r, mr);
    mpz_clear(ma);
    mpz_clear(mr);
}

void nSecp256r1_div(PnSecp256r1Element r, PnSecp256r1Element a, PnSecp256r1Element b) {
    nSecp256r1Element tmp;
    nSecp256r1_inv(&tmp, b);
    nSecp256r1_mul(r, a, &tmp);
}

void nSecp256r1_fail() {
    assert(false);
}

void nSecp256r1_longErr() {
    nSecp256r1_fail();
}

RawnSecp256r1::RawnSecp256r1() {
    nSecp256r1_init();
    set(fZero, 0);
    set(fOne, 1);
    neg(fNegOne, fOne);
}

RawnSecp256r1::~RawnSecp256r1() {
}

void RawnSecp256r1::fromString(Element &r, const std::string &s, uint32_t radix) {
    mpz_t mr;
    mpz_init_set_str(mr, s.c_str(), radix);
    mpz_fdiv_r(mr, mr, q);
    for (int i=0; i<nSecp256r1_N64; i++) r.v[i] = 0;
    mpz_export((void *)(r.v), NULL, -1, 8, -1, 0, mr);
    nSecp256r1_rawToMontgomery(r.v,r.v);
    mpz_clear(mr);
}

void RawnSecp256r1::fromUI(Element &r, unsigned long int v) {
    mpz_t mr;
    mpz_init(mr);
    mpz_set_ui(mr, v);
    for (int i=0; i<nSecp256r1_N64; i++) r.v[i] = 0;
    mpz_export((void *)(r.v), NULL, -1, 8, -1, 0, mr);
    nSecp256r1_rawToMontgomery(r.v,r.v);
    mpz_clear(mr);
}

RawnSecp256r1::Element RawnSecp256r1::set(int value) {
  Element r;
  set(r, value);
  return r;
}

void RawnSecp256r1::set(Element &r, int value) {
  mpz_t mr;
  mpz_init(mr);
  mpz_set_si(mr, value);
  if (value < 0) {
      mpz_add(mr, mr, q);
  }

  mpz_export((void *)(r.v), NULL, -1, 8, -1, 0, mr);

  for (int i=0; i<nSecp256r1_N64; i++) r.v[i] = 0;
  mpz_export((void *)(r.v), NULL, -1, 8, -1, 0, mr);
  nSecp256r1_rawToMontgomery(r.v,r.v);
  mpz_clear(mr);
}

std::string RawnSecp256r1::toString(const Element &a, uint32_t radix) {
    Element tmp;
    mpz_t r;
    nSecp256r1_rawFromMontgomery(tmp.v, a.v);
    mpz_init(r);
    mpz_import(r, nSecp256r1_N64, -1, 8, -1, 0, (const void *)(tmp.v));
    char *res = mpz_get_str (0, radix, r);
    mpz_clear(r);
    std::string resS(res);
    free(res);
    return resS;
}


#if defined(__SIZEOF_INT128__) && 1

// Variable-time modular inversion with the safegcd algorithm of Bernstein and Yang, using 62-bit
// signed limbs, as implemented in libsecp256k1 (src/modinv64_impl.h, modinv64_var() and its
// helpers), with the limb count set for this field. It is faster than mpz_invert() and gives the
// same result. libsecp256k1 is Copyright (c) 2013 Pieter Wuille, and modinv64_impl.h is
// Copyright (c) 2020 Peter Dettman, distributed under the MIT software license:
// https://www.opensource.org/licenses/mit-license.php
// For an explanation of the algorithm see libsecp256k1's doc/safegcd_implementation.md.

namespace {

const int MODINV_N = 5;

// Signed 62-bit limb integer: sum(v[i] * 2^(62*i), i=0..MODINV_N-1)
struct ModInvSigned62 {
    int64_t v[MODINV_N];
};

// Transition matrix t = [u v; q r], scaled by 2^62
struct ModInvTrans2x2 {
    int64_t u, v, q, r;
};

const ModInvSigned62 modinv_modulus = {{ 0x33b9cac2fc632551LL, 0x339beab69c5e7a13LL, 0x3ffffffffffffffbLL, 0x3fffffc00000003fLL, 0xffLL }};
const uint64_t modinv_modulus_inv62 = 0x332e375511ff43b1ULL;
const uint64_t modinv_q[nSecp256r1_N64] = { 0xf3b9cac2fc632551,0xbce6faada7179e84,0xffffffffffffffff,0xffffffff00000000 };

// Takes r in range (-2*q, q), with limbs in range (-2^62, 2^62), and adds a multiple of q to
// bring it to range [0, q); if sign < 0, it also negates it. Output limbs are in range [0, 2^62)
inline void modinv_normalize_62(ModInvSigned62 &r, int64_t sign) {
    const int64_t M62 = (int64_t)(UINT64_MAX >> 2);
    int64_t cond_add = r.v[MODINV_N - 1] >> 63;
    for (int i = 0; i < MODINV_N; i++) r.v[i] += modinv_modulus.v[i] & cond_add;
    int64_t cond_negate = sign >> 63;
    for (int i = 0; i < MODINV_N; i++) r.v[i] = (r.v[i] ^ cond_negate) - cond_negate;
    for (int i = 0; i < MODINV_N - 1; i++) { r.v[i + 1] += r.v[i] >> 62; r.v[i] &= M62; }
    cond_add = r.v[MODINV_N - 1] >> 63;
    for (int i = 0; i < MODINV_N; i++) r.v[i] += modinv_modulus.v[i] & cond_add;
    for (int i = 0; i < MODINV_N - 1; i++) { r.v[i + 1] += r.v[i] >> 62; r.v[i] &= M62; }
}

// Computes the transition matrix and eta for 62 divsteps (variable time, eta = -delta)
inline int64_t modinv_divsteps_62_var(int64_t eta, uint64_t f0, uint64_t g0, ModInvTrans2x2 &t) {
    uint64_t u = 1, v = 0, q = 0, r = 1;
    uint64_t f = f0, g = g0, m;
    uint32_t w;
    int i = 62, limit, zeros;

    for (;;) {
        // Use a sentinel bit to count zeros only up to i
        zeros = __builtin_ctzll(g | (UINT64_MAX << i));
        // Perform zeros divsteps at once; they all just divide g by two
        g >>= zeros;
        u <<= zeros;
        v <<= zeros;
        eta -= zeros;
        i -= zeros;
        // We're done once we've done 62 divsteps
        if (i == 0) break;
        // If eta is negative, negate it and replace f,g with g,-f
        if (eta < 0) {
            uint64_t tmp;
            eta = -eta;
            tmp = f; f = g; g = -tmp;
            tmp = u; u = q; q = -tmp;
            tmp = v; v = r; r = -tmp;
            // Cancel out up to 6 bits of g; no more than i can be cancelled out, and no more than
            // eta+1 can be done as its sign will flip again once that happens
            limit = ((int)eta + 1) > i ? i : ((int)eta + 1);
            m = (UINT64_MAX >> (64 - limit)) & 63U;
            w = (f * g * (f * f - 2)) & m;
        } else {
            // A simpler formula that cancels up to 4 bits of g, as eta tends to be smaller here
            limit = ((int)eta + 1) > i ? i : ((int)eta + 1);
            m = (UINT64_MAX >> (64 - limit)) & 15U;
            w = f + (((f + 1) & 4) << 1);
            w = (-w * g) & m;
        }
        g += f * w;
        q += u * w;
        r += v * w;
    }
    t.u = (int64_t)u;
    t.v = (int64_t)v;
    t.q = (int64_t)q;
    t.r = (int64_t)r;
    return eta;
}

// Computes (t/2^62) * [d, e] mod q. On input and output, d and e are in range (-2*q, q), and
// all output limbs are in range (-2^62, 2^62)
inline void modinv_update_de_62(ModInvSigned62 &d, ModInvSigned62 &e, const ModInvTrans2x2 &t) {
    const uint64_t M62 = UINT64_MAX >> 2;
    const int64_t u = t.u, v = t.v, q = t.q, r = t.r;
    int64_t md, me, sd, se;
    __int128 cd, ce;

    // [md,me] start as zero; plus [u,q] if d is negative; plus [v,r] if e is negative
    sd = d.v[MODINV_N - 1] >> 63;
    se = e.v[MODINV_N - 1] >> 63;
    md = (u & sd) + (v & se);
    me = (q & sd) + (r & se);
    // Begin computing t*[d,e]
    cd = (__int128)u * d.v[0] + (__int128)v * e.v[0];
    ce = (__int128)q * d.v[0] + (__int128)r * e.v[0];
    // Correct md,me so that t*[d,e]+modulus*[md,me] has 62 zero bottom bits
    md -= (modinv_modulus_inv62 * (uint64_t)cd + md) & M62;
    me -= (modinv_modulus_inv62 * (uint64_t)ce + me) & M62;
    cd += (__int128)modinv_modulus.v[0] * md;
    ce += (__int128)modinv_modulus.v[0] * me;
    // The low 62 bits are now zero: throw them away
    cd >>= 62;
    ce >>= 62;
    // Compute limb i of t*[d,e]+modulus*[md,me], and store it as output limb i-1 (= down shift)
    for (int i = 1; i < MODINV_N; i++) {
        cd += (__int128)u * d.v[i] + (__int128)v * e.v[i];
        ce += (__int128)q * d.v[i] + (__int128)r * e.v[i];
        if (modinv_modulus.v[i]) {
            cd += (__int128)modinv_modulus.v[i] * md;
            ce += (__int128)modinv_modulus.v[i] * me;
        }
        d.v[i - 1] = (int64_t)((uint64_t)cd & M62); cd >>= 62;
        e.v[i - 1] = (int64_t)((uint64_t)ce & M62); ce >>= 62;
    }
    // What remains is limb MODINV_N of the result; store it as output limb MODINV_N-1
    d.v[MODINV_N - 1] = (int64_t)cd;
    e.v[MODINV_N - 1] = (int64_t)ce;
}

// Computes (t/2^62) * [f, g], using only the len lowest limbs
inline void modinv_update_fg_62_var(int len, ModInvSigned62 &f, ModInvSigned62 &g, const ModInvTrans2x2 &t) {
    const uint64_t M62 = UINT64_MAX >> 2;
    const int64_t u = t.u, v = t.v, q = t.q, r = t.r;
    int64_t fi = f.v[0], gi = g.v[0];
    __int128 cf = (__int128)u * fi + (__int128)v * gi;
    __int128 cg = (__int128)q * fi + (__int128)r * gi;
    // The bottom 62 bits of the result are zero: throw them away
    cf >>= 62;
    cg >>= 62;
    for (int i = 1; i < len; ++i) {
        fi = f.v[i];
        gi = g.v[i];
        cf += (__int128)u * fi + (__int128)v * gi;
        cg += (__int128)q * fi + (__int128)r * gi;
        f.v[i - 1] = (int64_t)((uint64_t)cf & M62); cf >>= 62;
        g.v[i - 1] = (int64_t)((uint64_t)cg & M62); cg >>= 62;
    }
    f.v[len - 1] = (int64_t)cf;
    g.v[len - 1] = (int64_t)cg;
}

// Computes r = a^-1 mod q, or 0 if a = 0 mod q. Any a < 2^(64*N64) is accepted: values >= q are
// reduced first, which gives the same result as mpz_invert()
void modinv_var(uint64_t *r, const uint64_t *a) {
    const uint64_t M62 = UINT64_MAX >> 2;
    const int N64 = nSecp256r1_N64;

    // Reduce a to [0, q)
    uint64_t x[N64];
    for (int i = 0; i < N64; i++) x[i] = a[i];
    for (;;) {
        int lt = 0;
        for (int i = N64 - 1; i >= 0; i--) {
            if (x[i] != modinv_q[i]) { lt = x[i] < modinv_q[i]; break; }
        }
        if (lt) break;
        uint64_t borrow = 0;
        for (int i = 0; i < N64; i++) {
            unsigned __int128 diff = (unsigned __int128)x[i] - modinv_q[i] - borrow;
            x[i] = (uint64_t)diff;
            borrow = (uint64_t)(diff >> 64) & 1;
        }
    }

    // Convert to 62-bit limbs
    ModInvSigned62 g;
    for (int i = 0; i < MODINV_N; i++) {
        int bit = 62 * i, word = bit / 64, offset = bit % 64;
        uint64_t value = 0;
        if (word < N64) {
            value = x[word] >> offset;
            if ((offset > 2) && (word + 1 < N64)) value |= x[word + 1] << (64 - offset);
        }
        g.v[i] = (int64_t)(value & M62);
    }

    // Start with d=0, e=1, f=q, g=x, eta=-1, and do iterations of 62 divsteps each until g=0
    ModInvSigned62 d, e;
    for (int i = 0; i < MODINV_N; i++) { d.v[i] = 0; e.v[i] = 0; }
    e.v[0] = 1;
    ModInvSigned62 f = modinv_modulus;
    int len = MODINV_N;
    int64_t eta = -1;
    for (;;) {
        ModInvTrans2x2 t;
        eta = modinv_divsteps_62_var(eta, (uint64_t)f.v[0], (uint64_t)g.v[0], t);
        modinv_update_de_62(d, e, t);
        modinv_update_fg_62_var(len, f, g, t);
        // If the bottom limb of g is zero, there is a chance that g=0
        if (g.v[0] == 0) {
            int64_t cond = 0;
            for (int j = 1; j < len; ++j) cond |= g.v[j];
            if (cond == 0) break;
        }
        // Determine if len>1 and limb (len-1) of both f and g is 0 or -1
        int64_t fn = f.v[len - 1];
        int64_t gn = g.v[len - 1];
        int64_t cond = ((int64_t)len - 2) >> 63;
        cond |= fn ^ (fn >> 63);
        cond |= gn ^ (gn >> 63);
        // If so, reduce length, propagating the sign of f and g's top limb into the one below
        if (cond == 0) {
            f.v[len - 2] |= (int64_t)((uint64_t)fn << 62);
            g.v[len - 2] |= (int64_t)((uint64_t)gn << 62);
            --len;
        }
    }

    // Now g is 0 and f is +/-1 (or x was 0), so d is +/- the inverse: negate it if needed and
    // normalize it to [0, q)
    modinv_normalize_62(d, f.v[len - 1]);

    // Convert back to 64-bit limbs
    for (int j = 0; j < N64; j++) r[j] = 0;
    for (int i = 0; i < MODINV_N; i++) {
        int bit = 62 * i, word = bit / 64, offset = bit % 64;
        uint64_t value = (uint64_t)d.v[i];
        if (word < N64) r[word] |= value << offset;
        if ((offset > 2) && (word + 1 < N64)) r[word + 1] |= value >> (64 - offset);
    }
}

} // namespace

void RawnSecp256r1::inv(Element &r, const Element &a) {
    // a = x*R, so its inverse times R^3 (a Montgomery multiplication by R^3) is x^-1*R
    modinv_var(r.v, a.v);
    nSecp256r1_rawMMul(r.v, r.v, nSecp256r1_R3.longVal);
}

#else

void RawnSecp256r1::inv(Element &r, const Element &a) {
    mpz_t mr;
    mpz_init(mr);
    mpz_import(mr, nSecp256r1_N64, -1, 8, -1, 0, (const void *)(a.v));
    mpz_invert(mr, mr, q);


    for (int i=0; i<nSecp256r1_N64; i++) r.v[i] = 0;
    mpz_export((void *)(r.v), NULL, -1, 8, -1, 0, mr);

    nSecp256r1_rawMMul(r.v, r.v,nSecp256r1_R3.longVal);
    mpz_clear(mr);
}

#endif

void RawnSecp256r1::div(Element &r, const Element &a, const Element &b) {
    Element tmp;
    inv(tmp, b);
    mul(r, a, tmp);
}

#define BIT_IS_SET(s, p) (s[p>>3] & (1 << (p & 0x7)))
void RawnSecp256r1::exp(Element &r, const Element &base, uint8_t* scalar, unsigned int scalarSize) {
    bool oneFound = false;
    Element copyBase;
    copy(copyBase, base);
    for (int i=scalarSize*8-1; i>=0; i--) {
        if (!oneFound) {
            if ( !BIT_IS_SET(scalar, i) ) continue;
            copy(r, copyBase);
            oneFound = true;
            continue;
        }
        square(r, r);
        if ( BIT_IS_SET(scalar, i) ) {
            mul(r, r, copyBase);
        }
    }
    if (!oneFound) {
        copy(r, fOne);
    }
}

void RawnSecp256r1::toMpz(mpz_t r, const Element &a) {
    Element tmp;
    nSecp256r1_rawFromMontgomery(tmp.v, a.v);
    mpz_import(r, nSecp256r1_N64, -1, 8, -1, 0, (const void *)tmp.v);
}

void RawnSecp256r1::fromMpz(Element &r, const mpz_t a) {
    for (int i=0; i<nSecp256r1_N64; i++) r.v[i] = 0;
    mpz_export((void *)(r.v), NULL, -1, 8, -1, 0, a);
    nSecp256r1_rawToMontgomery(r.v, r.v);
}

int RawnSecp256r1::toRprBE(const Element &element, uint8_t *data, int bytes)
{
    if (bytes < nSecp256r1_N64 * 8) {
      return -(nSecp256r1_N64 * 8);
    }

    mpz_t r;
    mpz_init(r);

    toMpz(r, element);
   
    mpz_export(data, NULL, 1, bytes, 1, 0, r);

    return nSecp256r1_N64 * 8;
}

int RawnSecp256r1::fromRprBE(Element &element, const uint8_t *data, int bytes)
{
    if (bytes < nSecp256r1_N64 * 8) {
      return -(nSecp256r1_N64* 8);
    }
    mpz_t r;
    mpz_init(r);

    mpz_import(r, nSecp256r1_N64 * 8, 0, 1, 0, 0, data);
    fromMpz(element, r);
    return nSecp256r1_N64 * 8;
}

static bool init = nSecp256r1_init();

RawnSecp256r1 RawnSecp256r1::field;

