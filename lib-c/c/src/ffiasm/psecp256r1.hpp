#ifndef __PSECP256R1_H
#define __PSECP256R1_H

#include "psecp256r1_element.hpp"
#include <cstdint>
#include <string>
#include <gmp.h>

extern pSecp256r1Element pSecp256r1_q;
extern pSecp256r1Element pSecp256r1_R2;
extern pSecp256r1Element pSecp256r1_R3;

#ifdef USE_ASM

#if defined(ARCH_X86_64)

extern "C" void pSecp256r1_copy(PpSecp256r1Element r, PpSecp256r1Element a);
extern "C" void pSecp256r1_copyn(PpSecp256r1Element r, PpSecp256r1Element a, int n);
extern "C" void pSecp256r1_add(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_sub(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_neg(PpSecp256r1Element r, PpSecp256r1Element a);
extern "C" void pSecp256r1_mul(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_square(PpSecp256r1Element r, PpSecp256r1Element a);
extern "C" void pSecp256r1_band(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_bor(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_bxor(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_bnot(PpSecp256r1Element r, PpSecp256r1Element a);
extern "C" void pSecp256r1_shl(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_shr(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_eq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_neq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_lt(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_gt(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_leq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_geq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_land(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_lor(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
extern "C" void pSecp256r1_lnot(PpSecp256r1Element r, PpSecp256r1Element a);
extern "C" void pSecp256r1_toNormal(PpSecp256r1Element r, PpSecp256r1Element a);
extern "C" void pSecp256r1_toLongNormal(PpSecp256r1Element r, PpSecp256r1Element a);
extern "C" void pSecp256r1_toMontgomery(PpSecp256r1Element r, PpSecp256r1Element a);

extern "C" int pSecp256r1_isTrue(PpSecp256r1Element pE);
extern "C" int pSecp256r1_toInt(PpSecp256r1Element pE);

extern "C" void pSecp256r1_rawCopy(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
extern "C" void pSecp256r1_rawSwap(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA);
extern "C" void pSecp256r1_rawAdd(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB);
extern "C" void pSecp256r1_rawSub(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB);
extern "C" void pSecp256r1_rawNeg(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
extern "C" void pSecp256r1_rawMMul(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB);
extern "C" void pSecp256r1_rawMSquare(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
extern "C" void pSecp256r1_rawMMul1(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, uint64_t pRawB);
extern "C" void pSecp256r1_rawToMontgomery(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
extern "C" void pSecp256r1_rawFromMontgomery(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
extern "C" int pSecp256r1_rawIsEq(pSecp256r1RawElement const pRawA, const pSecp256r1RawElement pRawB);
extern "C" int pSecp256r1_rawIsZero(const pSecp256r1RawElement pRawB);
extern "C" void pSecp256r1_rawShl(pSecp256r1RawElement r, pSecp256r1RawElement a, uint64_t b);
extern "C" void pSecp256r1_rawShr(pSecp256r1RawElement r, pSecp256r1RawElement a, uint64_t b);

extern "C" void pSecp256r1_fail();

#elif defined(ARCH_ARM64)

           void pSecp256r1_copy(PpSecp256r1Element r, PpSecp256r1Element a);
           void pSecp256r1_mul(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_toNormal(PpSecp256r1Element r, PpSecp256r1Element a);

           void pSecp256r1_toLongNormal(PpSecp256r1Element r, PpSecp256r1Element a);
           int  pSecp256r1_isTrue(PpSecp256r1Element pE);
           void pSecp256r1_copyn(PpSecp256r1Element r, PpSecp256r1Element a, int n);
           void pSecp256r1_lt(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           int  pSecp256r1_toInt(PpSecp256r1Element pE);
           void pSecp256r1_shr(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_shl(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_band(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_bor(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_bxor(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_bnot(PpSecp256r1Element r, PpSecp256r1Element a);
           void pSecp256r1_sub(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_eq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_neq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_add(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_gt(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_leq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_geq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_lor(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_lnot(PpSecp256r1Element r, PpSecp256r1Element a);
           void pSecp256r1_land(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
           void pSecp256r1_neg(PpSecp256r1Element r, PpSecp256r1Element a);
           void pSecp256r1_toMontgomery(PpSecp256r1Element r, PpSecp256r1Element a);
           void pSecp256r1_square(PpSecp256r1Element r, PpSecp256r1Element a);

extern "C" void pSecp256r1_rawCopy(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
extern "C" void pSecp256r1_rawSwap(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA);
extern "C" void pSecp256r1_rawAdd(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB);
extern "C" void pSecp256r1_rawSub(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB);
extern "C" void pSecp256r1_rawNeg(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
extern "C" void pSecp256r1_rawMMul(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB);
           void pSecp256r1_rawMSquare(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
extern "C" void pSecp256r1_rawMMul1(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, uint64_t pRawB);
           void pSecp256r1_rawToMontgomery(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
extern "C" void pSecp256r1_rawFromMontgomery(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
extern "C" int  pSecp256r1_rawIsEq(const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB);
extern "C" int  pSecp256r1_rawIsZero(const pSecp256r1RawElement pRawB);
           void pSecp256r1_rawZero(pSecp256r1RawElement pRawResult);
extern "C" void pSecp256r1_rawCopyS2L(pSecp256r1RawElement pRawResult, int64_t val);
extern "C" void pSecp256r1_rawAddLS(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, uint64_t rawB);
extern "C" void pSecp256r1_rawSubSL(pSecp256r1RawElement pRawResult, uint64_t rawA, pSecp256r1RawElement pRawB);
extern "C" void pSecp256r1_rawSubLS(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, uint64_t rawB);
extern "C" void pSecp256r1_rawNegLS(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, uint64_t rawB);
extern "C" int  pSecp256r1_rawCmp(pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB);
extern "C" void pSecp256r1_rawAnd(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB);
extern "C" void pSecp256r1_rawOr(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB);
extern "C" void pSecp256r1_rawXor(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB);
extern "C" void pSecp256r1_rawShl(pSecp256r1RawElement r, pSecp256r1RawElement a, uint64_t b);
extern "C" void pSecp256r1_rawShr(pSecp256r1RawElement r, pSecp256r1RawElement a, uint64_t b);
extern "C" void pSecp256r1_rawNot(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA);
extern "C" void pSecp256r1_rawSubRegular(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB);

           void pSecp256r1_fail();
           void pSecp256r1_longErr();

#endif

#else


void pSecp256r1_copy(PpSecp256r1Element r, PpSecp256r1Element a);
void pSecp256r1_mul(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_toNormal(PpSecp256r1Element r, PpSecp256r1Element a);

void pSecp256r1_toLongNormal(PpSecp256r1Element r, PpSecp256r1Element a);
int pSecp256r1_isTrue(PpSecp256r1Element pE);
void pSecp256r1_copyn(PpSecp256r1Element r, PpSecp256r1Element a, int n);
void pSecp256r1_lt(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
int pSecp256r1_toInt(PpSecp256r1Element pE);
void pSecp256r1_shl(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_shr(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_band(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_bor(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_bxor(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_bnot(PpSecp256r1Element r, PpSecp256r1Element a);
void pSecp256r1_sub(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_eq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_neq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_add(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_gt(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_leq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_geq(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_lor(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_lnot(PpSecp256r1Element r, PpSecp256r1Element a);
void pSecp256r1_land(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_neg(PpSecp256r1Element r, PpSecp256r1Element a);
void pSecp256r1_toMontgomery(PpSecp256r1Element r, PpSecp256r1Element a);
void pSecp256r1_square(PpSecp256r1Element r, PpSecp256r1Element a);

void pSecp256r1_rawCopy(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
void pSecp256r1_rawSwap(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA);
void pSecp256r1_rawAdd(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB);
void pSecp256r1_rawSub(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB);
void pSecp256r1_rawNeg(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
void pSecp256r1_rawMMul(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB);
void pSecp256r1_rawMSquare(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
void pSecp256r1_rawMMul1(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA, uint64_t pRawB);
void pSecp256r1_rawToMontgomery(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
void pSecp256r1_rawFromMontgomery(pSecp256r1RawElement pRawResult, const pSecp256r1RawElement pRawA);
int pSecp256r1_rawIsEq(const pSecp256r1RawElement pRawA, const pSecp256r1RawElement pRawB);
int pSecp256r1_rawIsZero(const pSecp256r1RawElement pRawB);
void pSecp256r1_rawZero(pSecp256r1RawElement pRawResult);
void pSecp256r1_rawCopyS2L(pSecp256r1RawElement pRawResult, int64_t val);
void pSecp256r1_rawAddLS(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, uint64_t rawB);
void pSecp256r1_rawSubSL(pSecp256r1RawElement pRawResult, uint64_t rawA, pSecp256r1RawElement pRawB);
void pSecp256r1_rawSubLS(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, uint64_t rawB);
void pSecp256r1_rawNegLS(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, uint64_t rawB);
int  pSecp256r1_rawCmp(pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB);
void pSecp256r1_rawAnd(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB);
void pSecp256r1_rawOr(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB);
void pSecp256r1_rawXor(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB);
void pSecp256r1_rawShl(pSecp256r1RawElement r, pSecp256r1RawElement a, uint64_t b);
void pSecp256r1_rawShr(pSecp256r1RawElement r, pSecp256r1RawElement a, uint64_t b);
void pSecp256r1_rawNot(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA);
void pSecp256r1_rawSubRegular(pSecp256r1RawElement pRawResult, pSecp256r1RawElement pRawA, pSecp256r1RawElement pRawB);

void pSecp256r1_fail();
void pSecp256r1_longErr();

#endif

// Pending functions to convert

void pSecp256r1_str2element(PpSecp256r1Element pE, char const*s);
char *pSecp256r1_element2str(PpSecp256r1Element pE);
void pSecp256r1_idiv(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_mod(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_inv(PpSecp256r1Element r, PpSecp256r1Element a);
void pSecp256r1_div(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);
void pSecp256r1_pow(PpSecp256r1Element r, PpSecp256r1Element a, PpSecp256r1Element b);

class RawpSecp256r1 {

public:
    const static int N64 = pSecp256r1_N64;
    const static int MaxBits = 256;


    struct Element {
        pSecp256r1RawElement v;
    };

private:
    Element fZero;
    Element fOne;
    Element fNegOne;

public:

    RawpSecp256r1();
    ~RawpSecp256r1();

    const Element &zero() { return fZero; }
    const Element &one() { return fOne; }
    const Element &negOne() { return fNegOne; }
    Element set(int value);
    void set(Element &r, int value);

    void fromString(Element &r, const std::string &n, uint32_t radix = 10);
    std::string toString(const Element &a, uint32_t radix = 10);

    void inline copy(Element &r, const Element &a) { pSecp256r1_rawCopy(r.v, a.v); }
    void inline swap(Element &a, Element &b) { pSecp256r1_rawSwap(a.v, b.v); }
    void inline add(Element &r, const Element &a, const Element &b) { pSecp256r1_rawAdd(r.v, a.v, b.v); }
    void inline sub(Element &r, const Element &a, const Element &b) { pSecp256r1_rawSub(r.v, a.v, b.v); }
    void inline mul(Element &r, const Element &a, const Element &b) { pSecp256r1_rawMMul(r.v, a.v, b.v); }

    Element inline add(const Element &a, const Element &b) { Element r; pSecp256r1_rawAdd(r.v, a.v, b.v); return r;}
    Element inline sub(const Element &a, const Element &b) { Element r; pSecp256r1_rawSub(r.v, a.v, b.v); return r;}
    Element inline mul(const Element &a, const Element &b) { Element r; pSecp256r1_rawMMul(r.v, a.v, b.v); return r;}

    Element inline neg(const Element &a) { Element r; pSecp256r1_rawNeg(r.v, a.v); return r; }
    Element inline square(const Element &a) { Element r; pSecp256r1_rawMSquare(r.v, a.v); return r; }

    Element inline add(int a, const Element &b) { return add(set(a), b);}
    Element inline sub(int a, const Element &b) { return sub(set(a), b);}
    Element inline mul(int a, const Element &b) { return mul(set(a), b);}

    Element inline add(const Element &a, int b) { return add(a, set(b));}
    Element inline sub(const Element &a, int b) { return sub(a, set(b));}
    Element inline mul(const Element &a, int b) { return mul(a, set(b));}

    void inline mul1(Element &r, const Element &a, uint64_t b) { pSecp256r1_rawMMul1(r.v, a.v, b); }
    void inline neg(Element &r, const Element &a) { pSecp256r1_rawNeg(r.v, a.v); }
    void inline square(Element &r, const Element &a) { pSecp256r1_rawMSquare(r.v, a.v); }
    void inv(Element &r, const Element &a);
    void div(Element &r, const Element &a, const Element &b);
    void exp(Element &r, const Element &base, uint8_t* scalar, unsigned int scalarSize);

    void inline toMontgomery(Element &r, const Element &a) { pSecp256r1_rawToMontgomery(r.v, a.v); }
    void inline fromMontgomery(Element &r, const Element &a) { pSecp256r1_rawFromMontgomery(r.v, a.v); }
    int inline eq(const Element &a, const Element &b) { return pSecp256r1_rawIsEq(a.v, b.v); }
    int inline isZero(const Element &a) { return pSecp256r1_rawIsZero(a.v); }

    void toMpz(mpz_t r, const Element &a);
    void fromMpz(Element &a, const mpz_t r);

    int toRprBE(const Element &element, uint8_t *data, int bytes);
    int fromRprBE(Element &element, const uint8_t *data, int bytes);

    int bytes ( void ) { return pSecp256r1_N64 * 8; }


    void fromUI(Element &r, unsigned long int v);

    static RawpSecp256r1 field;

};

#endif // __PSECP256R1_H
