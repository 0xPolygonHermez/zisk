#ifndef GOLDILOCKS_AVX
#define GOLDILOCKS_AVX
#include "goldilocks_base_field.hpp"
#include <immintrin.h>
#include <cassert>

// NOTATION:
// _c value is in canonical form
// _s value shifted (a_s = a + (1<<63) = a XOR (1<<63)
// _n negative P_n = -P
// _l low part of a variable: uint64 [31:0] or uint128 [63:0]
// _h high part of a variable: uint64 [63:32] or uint128 [127:64]
// _a alingned pointer
// _8 variable can be expressed in 8 bits (<256)

// OBSERVATIONS:
// 1.  a + b overflows iff (a + b) < a (AVX does not suport carry, this is the way to check)
// 2.  a - b underflows iff (a - b) > a (AVX does not suport carry, this is the way to check)
// 3. (unsigned) a < (unsigned) b iff (signed) a_s < (singed) b_s (AVX2 does not support unsingend 64-bit comparisons)
// 4. a_s + b = (a+b)_s. Dem: a+(1<<63)+b = a+b+(1<<63)

const __m256i MSB = _mm256_set_epi64x(MSB_, MSB_, MSB_, MSB_);
const __m256i P = _mm256_set_epi64x(GOLDILOCKS_PRIME, GOLDILOCKS_PRIME, GOLDILOCKS_PRIME, GOLDILOCKS_PRIME);
const __m256i P_n = _mm256_set_epi64x(GOLDILOCKS_PRIME_NEG, GOLDILOCKS_PRIME_NEG, GOLDILOCKS_PRIME_NEG, GOLDILOCKS_PRIME_NEG);
const __m256i P_s = _mm256_xor_si256(P, MSB);
const __m256i sqmask = _mm256_set_epi64x(0x1FFFFFFFF, 0x1FFFFFFFF, 0x1FFFFFFFF, 0x1FFFFFFFF);

inline void Goldilocks::set_avx(__m256i &a, const Goldilocks::Element &a0, const Goldilocks::Element &a1, const Goldilocks::Element &a2, const Goldilocks::Element &a3)
{
    a = _mm256_set_epi64x(a3.fe, a2.fe, a1.fe, a0.fe);
}

inline void Goldilocks::load_avx(__m256i &a_, const Goldilocks::Element *a4)
{
    a_ = _mm256_loadu_si256((__m256i *)(a4));
}

// We assume a4_a aligned on a 32-byte boundary
inline void Goldilocks::load_avx_a(__m256i &a, const Goldilocks::Element *a4_a)
{
    a = _mm256_load_si256((__m256i *)(a4_a));
}

inline void Goldilocks::store_avx(Goldilocks::Element *a4, const __m256i &a)
{
    _mm256_storeu_si256((__m256i *)a4, a);
}
inline void Goldilocks::store_avx(Goldilocks::Element *a4, uint64_t offset_a, const __m256i &a)
{
    Goldilocks::Element a4_[4];
    _mm256_storeu_si256((__m256i *)a4_, a);
    a4[0] = a4_[0];
    a4[offset_a] = a4_[1];
    a4[offset_a << 1] = a4_[2];
    a4[(offset_a << 1) + offset_a] = a4_[3];
}

// We assume a4_a aligned on a 32-byte boundary
inline void Goldilocks::store_avx_a(Goldilocks::Element *a4_a, const __m256i &a)
{
    _mm256_store_si256((__m256i *)a4_a, a);
}

inline void Goldilocks::shift_avx(__m256i &a_s, const __m256i &a)
{
    a_s = _mm256_xor_si256(a, MSB);
}

inline void Goldilocks::Goldilocks::toCanonical_avx(__m256i &a_c, const __m256i &a)
{
    __m256i a_s, a_sc;
    shift_avx(a_s, a);
    toCanonical_avx_s(a_sc, a_s);
    shift_avx(a_c, a_sc);
}

// Obtain cannonical representative of a_s,
// We assume a <= a_c+P
// a_sc a shifted canonical
// a_s  a shifted
inline void Goldilocks::toCanonical_avx_s(__m256i &a_sc, const __m256i &a_s)
{
    // a_s < P_s iff a < P. Then iff a >= P the mask bits are 0
    __m256i mask1_ = _mm256_cmpgt_epi64(P_s, a_s);
    __m256i corr1_ = _mm256_andnot_si256(mask1_, P_n);
    a_sc = _mm256_add_epi64(a_s, corr1_);
}

inline void Goldilocks::add_avx(__m256i &c, const __m256i &a, const __m256i &b)
{
    __m256i a_s, a_sc;
    shift_avx(a_s, a);
    toCanonical_avx_s(a_sc, a_s);
    add_avx_a_sc(c, a_sc, b);
}

// we assume a given in shifted cannonical form (a_sc)
inline void Goldilocks::add_avx_a_sc(__m256i &c, const __m256i &a_sc, const __m256i &b)
{
    // addition (if only one of the arguments is shifted the sumation is shifted)
    const __m256i c0_s = _mm256_add_epi64(a_sc, b);

    // correction if overflow (iff a_sc > a_sc+b )
    __m256i mask_ = _mm256_cmpgt_epi64(a_sc, c0_s);
    __m256i corr_ = _mm256_and_si256(mask_, P_n);
    __m256i c_s = _mm256_add_epi64(c0_s, corr_);

    // shift c_s to get c
    Goldilocks::shift_avx(c, c_s);
}

// Assume a shifted (a_s) and b<=0xFFFFFFFF00000000 (b_small), the result is shifted (c_s)
inline void Goldilocks::add_avx_s_b_small(__m256i &c_s, const __m256i &a_s, const __m256i &b_small)
{
    const __m256i c0_s = _mm256_add_epi64(a_s, b_small);
    // We can use 32-bit comparison that is faster, lets see:
    // 1) a_s > c0_s => a_sh >= c0_sh
    // 2) If a_sh = c0_sh => there is no overlow (demonstration bellow)
    // 3) Therefore: overflow iff a_sh > c0_sh
    // Dem item 2:
    //     c0_sh=a_sh+b_h+carry=a_sh
    //     carry = 0 or 1 is optional, but b_h+carry=0
    //     if carry==0 => b_h = 0 and as there is no carry => no overflow
    //     if carry==1 => b_h = 0xFFFFFFFF => b_l=0 (b <=0xFFFFFFFF00000000) => carry=0!!!!
    const __m256i mask_ = _mm256_cmpgt_epi32(a_s, c0_s);
    const __m256i corr_ = _mm256_srli_epi64(mask_, 32); // corr=P_n when a_s > c0_s
    c_s = _mm256_add_epi64(c0_s, corr_);
}

// Assume b<=0xFFFFFFFF00000000 (b_small), the result is shifted (c_s)
inline void Goldilocks::add_avx_b_small(__m256i &c, const __m256i &a, const __m256i &b_small)
{
    __m256i a_s;
    shift_avx(a_s, a);
    const __m256i c0_s = _mm256_add_epi64(a_s, b_small);
    // We can use 32-bit comparison that is faster, lets see:
    // 1) a_s > c0_s => a_sh >= c0_sh
    // 2) If a_sh = c0_sh => there is no overlow (demonstration bellow)
    // 3) Therefore: overflow iff a_sh > c0_sh
    // Dem item 2:
    //     c0_sh=a_sh+b_h+carry=a_sh
    //     carry = 0 or 1 is optional, but b_h+carry=0
    //     if carry==0 => b_h = 0 and as there is no carry => no overflow
    //     if carry==1 => b_h = 0xFFFFFFFF => b_l=0 (b <=0xFFFFFFFF00000000) => carry=0!!!!
    const __m256i mask_ = _mm256_cmpgt_epi32(a_s, c0_s);
    const __m256i corr_ = _mm256_srli_epi64(mask_, 32); // corr=P_n when a_s > c0_s
    shift_avx(c, _mm256_add_epi64(c0_s, corr_));
}

//
// Sub: a-b = (a+1^63)-(b+1^63)=a_s-b_s
//
inline void Goldilocks::sub_avx(__m256i &c, const __m256i &a, const __m256i &b)
{
    __m256i b_s, b_sc, a_s;
    shift_avx(b_s, b);
    shift_avx(a_s, a);
    toCanonical_avx_s(b_sc, b_s);
    const __m256i c0 = _mm256_sub_epi64(a_s, b_sc);
    const __m256i mask_ = _mm256_cmpgt_epi64(b_sc, a_s);
    // P > b_c > a =>  (a-b_c) < 0 and  P+(a-b_c)< P => 0 < (P-b_c)+a < P
    const __m256i corr_ = _mm256_and_si256(mask_, P);
    c = _mm256_add_epi64(c0, corr_);
}

// Assume a pre-shifted and b <0xFFFFFFFF00000000, the result is shifted
// a_s-b=(a+2^63)-b = 2^63+(a-b)=(a-b)_s
// b<0xFFFFFFFF00000000 => b=b_c
inline void Goldilocks::sub_avx_s_b_small(__m256i &c_s, const __m256i &a_s, const __m256i &b)
{

    const __m256i c0_s = _mm256_sub_epi64(a_s, b);
    // We can use 32-bit comparison that is faster
    // 1) c0_s > a_s => c0_s >= a_s
    // 2) If c0_s = a_s => there is no underflow
    // 3) Therefore: underflow iff c0_sh > a_sh
    // Dem 1: c0_sh=a_sh-b_h+borrow=a_sh
    //        borrow = 0 or 1 is optional, but b_h+borrow=0
    //        if borrow==0 => b_h = 0 and as there is no borrow => no underflow
    //        if borrow==1 => b_h = 0xFFFFFFFF => b_l=0 (b <=0xFFFFFFFF00000000) => borrow=0!!!!
    const __m256i mask_ = _mm256_cmpgt_epi32(c0_s, a_s);
    const __m256i corr_ = _mm256_srli_epi64(mask_, 32); // corr=P_n when a_s > c0_s
    c_s = _mm256_sub_epi64(c0_s, corr_);
}

inline void Goldilocks::mult_avx(__m256i &c, const __m256i &a, const __m256i &b)
{
    __m256i c_h, c_l;
    mult_avx_128(c_h, c_l, a, b);
    reduce_avx_128_64(c, c_h, c_l);
}

// We assume coeficients of b_8 can be expressed with 8 bits (<256)
inline void Goldilocks::mult_avx_8(__m256i &c, const __m256i &a, const __m256i &b_8)
{
    __m256i c_h, c_l;
    mult_avx_72(c_h, c_l, a, b_8);
    reduce_avx_96_64(c, c_h, c_l);
}

// The 128 bits of the result are stored in c_h[64:0]| c_l[64:0]
inline void Goldilocks::mult_avx_128(__m256i &c_h, __m256i &c_l, const __m256i &a, const __m256i &b)
{
    // Obtain a_h and b_h in the lower 32 bits
    //__m256i a_h = _mm256_srli_epi64(a, 32);
    //__m256i b_h = _mm256_srli_epi64(b, 32);
    __m256i a_h = _mm256_castps_si256(_mm256_movehdup_ps(_mm256_castsi256_ps(a)));
    __m256i b_h = _mm256_castps_si256(_mm256_movehdup_ps(_mm256_castsi256_ps(b)));

    // c = (a_h+a_l)*(b_h+b_l)=a_h*b_h+a_h*b_l+a_l*b_h+a_l*b_l=c_hh+c_hl+cl_h+c_ll
    // note: _mm256_mul_epu32 uses only the lower 32bits of each chunk so a=a_l and b=b_l
    __m256i c_ll = _mm256_mul_epu32(a, b);
    __m256i c_lh = _mm256_mul_epu32(a, b_h);
    __m256i c_hl = _mm256_mul_epu32(a_h, b);
    __m256i c_hh = _mm256_mul_epu32(a_h, b_h);

    // Bignum addition
    // Ranges: c_hh[127:64], c_hl[95:32], c_lh[95:32], c_ll[63:0]
    // parts that intersect must be added

    // LOW PART:
    // 1: r0 = c_hl + c_ll_h
    //    does not overflow: c_hl <= (2^32-1)*(2^32-1)=2^64-2*2^32+1
    //                       c_ll_h <= 2^32-1
    //                       c_hl + c_ll_h <= 2^64-2^32
    __m256i c_ll_h = _mm256_srli_epi64(c_ll, 32);
    __m256i r0 = _mm256_add_epi64(c_hl, c_ll_h);

    // 2: r1 = r0_l + c_lh //does not overflow
    __m256i r0_l = _mm256_and_si256(r0, P_n);
    __m256i r1 = _mm256_add_epi64(c_lh, r0_l);

    // 3: c_l = r1_l | c_ll_l
    //__m256i r1_l = _mm256_slli_epi64(r1, 32);
    __m256i r1_l = _mm256_castps_si256(_mm256_moveldup_ps(_mm256_castsi256_ps(r1)));
    c_l = _mm256_blend_epi32(c_ll, r1_l, 0xaa);

    // HIGH PART: c_h = c_hh + r0_h + r1_h
    // 1: r2 = r0_h + c_hh
    //    does not overflow: c_hh <= (2^32-1)*(2^32-1)=2^64-2*2^32+1
    //                       r0_h <= 2^32-1
    //                       r0_h + c_hh <= 2^64-2^32
    __m256i r0_h = _mm256_srli_epi64(r0, 32);
    __m256i r2 = _mm256_add_epi64(c_hh, r0_h);

    // 2: c_h = r3 + r1_h
    //    does not overflow: r2 <= 2^64-2^32
    //                       r1_h <= 2^32-1
    //                       r2 + r1_h <= 2^64-1
    __m256i r1_h = _mm256_srli_epi64(r1, 32);
    c_h = _mm256_add_epi64(r2, r1_h);
}

// The 72 bits the result are stored in c_h[32:0] | c_l[64:0]
inline void Goldilocks::mult_avx_72(__m256i &c_h, __m256i &c_l, const __m256i &a, const __m256i &b)
{
    // Obtain a_h in the lower 32 bits
    __m256i a_h = _mm256_srli_epi64(a, 32);
    //__m256i a_h = _mm256_castps_si256(_mm256_movehdup_ps(_mm256_castsi256_ps(a)));

    // c = (a_h+a_l)*(b_l)=a_h*b_l+a_l*b_l=c_hl+c_ll
    // note: _mm256_mul_epu32 uses only the lower 32bits of each chunk so a=a_l and b=b_l
    __m256i c_hl = _mm256_mul_epu32(a_h, b);
    __m256i c_ll = _mm256_mul_epu32(a, b);

    // Bignum addition
    // Ranges: c_hl[95:32], c_ll[63:0]
    // parts that intersect must be added

    // LOW PART:
    // 1: r0 = c_hl + c_ll_h
    //    does not overflow: c_hl <= (2^32-1)*(2^8-1)< 2^40
    //                       c_ll_h <= 2^32-1
    //                       c_hl + c_ll_h <= 2^41
    __m256i c_ll_h = _mm256_srli_epi64(c_ll, 32);
    __m256i r0 = _mm256_add_epi64(c_hl, c_ll_h);

    // 2: c_l = r0_l | c_ll_l
    __m256i r0_l = _mm256_slli_epi64(r0, 32);
    //__m256i r0_l = _mm256_castps_si256(_mm256_moveldup_ps(_mm256_castsi256_ps(r0)));
    c_l = _mm256_blend_epi32(c_ll, r0_l, 0xaa);

    // HIGH PART: c_h =  r0_h
    c_h = _mm256_srli_epi64(r0, 32);
}

// notes:
// 2^64 = P+P_n => [2^64]=[P_n]
// P = 2^64-2^32+1
// P_n = 2^32-1
// 2^32*P_n = 2^32*(2^32-1) = 2^64-2^32 = P-1
// process:
// c % P = [c] = [c_h*2^64+c_l] = [c_h*P_n+c_l] = [c_hh*2^32*P_n+c_hl*P_n+c_l] =
//             = [c_hh(P-1) +c_hl*P_n+c_l] = [c_l-c_hh+c_hl*P_n]
inline void Goldilocks::reduce_avx_128_64(__m256i &c, const __m256i &c_h, const __m256i &c_l)
{
    __m256i c_hh = _mm256_srli_epi64(c_h, 32);
    __m256i c1_s, c_ls, c_s;
    shift_avx(c_ls, c_l);
    sub_avx_s_b_small(c1_s, c_ls, c_hh);
    __m256i c2 = _mm256_mul_epu32(c_h, P_n); // c_hl*P_n (only 32bits of c_h useds)
    add_avx_s_b_small(c_s, c1_s, c2);
    shift_avx(c, c_s);
}

// notes:
// P = 2^64-2^32+1
// P_n = 2^32-1
// 2^32*P_n = 2^32*(2^32-1) = 2^64-2^32 = P-1
// 2^64 = P+P_n => [2^64]=[P_n]
// c_hh = 0 in this case
// process:
// c % P = [c] = [c_h*1^64+c_l] = [c_h*P_n+c_l] = [c_hh*2^32*P_n+c_hl*P_n+c_l] =
//             = [c_hl*P_n+c_l] = [c_l+c_hl*P_n]
inline void Goldilocks::reduce_avx_96_64(__m256i &c, const __m256i &c_h, const __m256i &c_l)
{
    __m256i c1 = _mm256_mul_epu32(c_h, P_n); // c_hl*P_n (only 32bits of c_h useds)
    add_avx_b_small(c, c_l, c1);             // c1 = c_hl*P_n <= (2^32-1)*(2^32-1) <= 2^64 -2^33+1 < P
}

inline void Goldilocks::square_avx(__m256i &c, __m256i &a)
{
    __m256i c_h, c_l;
    square_avx_128(c_h, c_l, a);
    reduce_avx_128_64(c, c_h, c_l);
}

inline void Goldilocks::square_avx_128(__m256i &c_h, __m256i &c_l, const __m256i &a)
{

    // Obtain a_h
    //__m256i a_h = _mm256_srli_epi64(a, 32);
    __m256i a_h = _mm256_castps_si256(_mm256_movehdup_ps(_mm256_castsi256_ps(a)));

    // c = (a_h+a_l)*(b_h*a_l)=a_h*a_h+2*a_h*a_l+a_l*a_l=c_hh+2*c_hl+c_ll
    // note: _mm256_mul_epu32 uses only the lower 32bits of each chunk so a=a_l
    __m256i c_hh = _mm256_mul_epu32(a_h, a_h);
    __m256i c_lh = _mm256_mul_epu32(a, a_h); // used as 2^c_lh
    __m256i c_ll = _mm256_mul_epu32(a, a);

    // Bignum addition
    // Ranges: c_hh[127:64], c_lh[95:32], 2*c_lh[96:33],c_ll[64:0]
    //         c_ll_h[63:33]
    // parts that intersect must be added

    // LOW PART:
    // 1: r0 = c_lh + c_ll_h (31 bits)
    // Does not overflow c_lh <= (2^32-1)*(2^32-1)=2^64-2*2^32+1
    //                   c_ll_h <= 2^31-1
    //                   r0 <= 2^64-2^33+2^31
    __m256i c_ll_h = _mm256_srli_epi64(c_ll, 33); // yes 33, low part of 2*c_lh is [31:0]
    __m256i r0 = _mm256_add_epi64(c_lh, c_ll_h);

    // 2: c_l = r0_l (31 bits) | c_ll_l (33 bits)
    __m256i r0_l = _mm256_slli_epi64(r0, 33);
    __m256i c_ll_l = _mm256_and_si256(c_ll, sqmask);
    c_l = _mm256_add_epi64(r0_l, c_ll_l);

    // HIGH PART:
    // 1: c_h = r0_h (33 bits) + c_hh (64 bits)
    // Does not overflow c_hh <= (2^32-1)*(2^32-1)=2^64-2*2^32+1
    //                   r0 <= 2^64-2^33+2^31 => r0_h <= 2^33-2 (_h means 33 bits here!)
    //                   Dem: r0_h=2^33-1 => r0 >= r0_h*2^31=2^64-2^31!!
    //                                  contradiction with what we saw above
    //                   c_hh + c0_h <= 2^64-2^33+1+2^33-2 <= 2^64-1
    __m256i r0_h = _mm256_srli_epi64(r0, 31);
    c_h = _mm256_add_epi64(c_hh, r0_h);
}

inline Goldilocks::Element Goldilocks::dot_avx(const __m256i &a0, const __m256i &a1, const __m256i &a2, const Element b[12])
{
    __m256i c_;
    spmv_avx_4x12(c_, a0, a1, a2, b);
    Goldilocks::Element c[4];
    store_avx(c, c_);
    return (c[0] + c[1]) + (c[2] + c[3]);
}

// We assume b_a aligned on a 32-byte boundary
inline Goldilocks::Element Goldilocks::dot_avx_a(const __m256i &a0, const __m256i &a1, const __m256i &a2, const Element b_a[12])
{
    __m256i c_;
    spmv_avx_4x12_a(c_, a0, a1, a2, b_a);
    alignas(32) Goldilocks::Element c[4];
    store_avx_a(c, c_);
    return (c[0] + c[1]) + (c[2] + c[3]);
}

// Sparse matrix-vector product (4x12 sparce matrix formed of three diagonal blocks of size 4x4)
// c[i]=Sum_j(aj[i]*b[j*4+i]) 0<=i<4 0<=j<3
inline void Goldilocks::spmv_avx_4x12(__m256i &c, const __m256i &a0, const __m256i &a1, const __m256i &a2, const Goldilocks::Element b[12])
{

    // load b into avx registers, latter
    __m256i b0, b1, b2;
    load_avx(b0, &(b[0]));
    load_avx(b1, &(b[4]));
    load_avx(b2, &(b[8]));

    __m256i c0, c1, c2;
    mult_avx(c0, a0, b0);
    mult_avx(c1, a1, b1);
    mult_avx(c2, a2, b2);

    __m256i c_;
    add_avx(c_, c0, c1);
    add_avx(c, c_, c2);
}

// Sparse matrix-vector product (4x12 sparce matrix formed of three diagonal blocks of size 4x4)
// c[i]=Sum_j(aj[i]*b[j*4+i]) 0<=i<4 0<=j<3
// We assume b_a aligned on a 32-byte boundary
inline void Goldilocks::spmv_avx_4x12_a(__m256i &c, const __m256i &a0, const __m256i &a1, const __m256i &a2, const Goldilocks::Element b_a[12])
{

    // load b into avx registers, latter
    __m256i b0, b1, b2;
    load_avx_a(b0, &(b_a[0]));
    load_avx_a(b1, &(b_a[4]));
    load_avx_a(b2, &(b_a[8]));

    __m256i c0, c1, c2;
    mult_avx(c0, a0, b0);
    mult_avx(c1, a1, b1);
    mult_avx(c2, a2, b2);

    __m256i c_;
    add_avx(c_, c0, c1);
    add_avx(c, c_, c2);
}

// Sparse matrix-vector product (4x12 sparce matrix formed of four diagonal blocs 4x5 stored in a0...a3)
// c[i]=Sum_j(aj[i]*b[j*4+i]) 0<=i<4 0<=j<3
// We assume b_a aligned on a 32-byte boundary
// We assume coeficients of b_8 can be expressed with 8 bits (<256)
inline void Goldilocks::spmv_avx_4x12_8(__m256i &c, const __m256i &a0, const __m256i &a1, const __m256i &a2, const Goldilocks::Element b_8[12])
{

    // load b into avx registers, latter
    __m256i b0, b1, b2;
    load_avx(b0, &(b_8[0]));
    load_avx(b1, &(b_8[4]));
    load_avx(b2, &(b_8[8]));

    /* __m256i c0, c1, c2;
     mult_avx_8(c0, a0, b0);
     mult_avx_8(c1, a1, b1);
     mult_avx_8(c2, a2, b2);

     __m256i c_;
     add_avx(c_, c0, c1);
     add_avx(c, c_, c2);*/
    __m256i c0_h, c1_h, c2_h;
    __m256i c0_l, c1_l, c2_l;
    mult_avx_72(c0_h, c0_l, a0, b0);
    mult_avx_72(c1_h, c1_l, a1, b1);
    mult_avx_72(c2_h, c2_l, a2, b2);

    __m256i c_h, c_l, aux_h, aux_l;

    add_avx(aux_l, c0_l, c1_l);
    add_avx(c_l, aux_l, c2_l);

    aux_h = _mm256_add_epi64(c0_h, c1_h); // do with epi32?
    c_h = _mm256_add_epi64(aux_h, c2_h);

    reduce_avx_96_64(c, c_h, c_l);
}

// Dense matrix-vector product
inline void Goldilocks::mmult_avx_4x12(__m256i &b, const __m256i &a0, const __m256i &a1, const __m256i &a2, const Goldilocks::Element M[48])
{
    // Generate matrix 4x4
    __m256i r0, r1, r2, r3;
    Goldilocks::spmv_avx_4x12(r0, a0, a1, a2, &(M[0]));
    Goldilocks::spmv_avx_4x12(r1, a0, a1, a2, &(M[12]));
    Goldilocks::spmv_avx_4x12(r2, a0, a1, a2, &(M[24]));
    Goldilocks::spmv_avx_4x12(r3, a0, a1, a2, &(M[36]));

    // Transpose: transform de 4x4 matrix stored in rows r0...r3 to the columns c0...c3
    __m256i t0 = _mm256_permute2f128_si256(r0, r2, 0b00100000);
    __m256i t1 = _mm256_permute2f128_si256(r1, r3, 0b00100000);
    __m256i t2 = _mm256_permute2f128_si256(r0, r2, 0b00110001);
    __m256i t3 = _mm256_permute2f128_si256(r1, r3, 0b00110001);
    __m256i c0 = _mm256_castpd_si256(_mm256_unpacklo_pd(_mm256_castsi256_pd(t0), _mm256_castsi256_pd(t1)));
    __m256i c1 = _mm256_castpd_si256(_mm256_unpackhi_pd(_mm256_castsi256_pd(t0), _mm256_castsi256_pd(t1)));
    __m256i c2 = _mm256_castpd_si256(_mm256_unpacklo_pd(_mm256_castsi256_pd(t2), _mm256_castsi256_pd(t3)));
    __m256i c3 = _mm256_castpd_si256(_mm256_unpackhi_pd(_mm256_castsi256_pd(t2), _mm256_castsi256_pd(t3)));

    // Add columns to obtain result
    __m256i sum0, sum1;
    add_avx(sum0, c0, c1);
    add_avx(sum1, c2, c3);
    add_avx(b, sum0, sum1);
}

// Dense matrix-vector product, we assume that M_a aligned on a 32-byte boundary
inline void Goldilocks::mmult_avx_4x12_a(__m256i &b, const __m256i &a0, const __m256i &a1, const __m256i &a2, const Goldilocks::Element M_a[48])
{
    // Generate matrix 4x4
    __m256i r0, r1, r2, r3;
    Goldilocks::spmv_avx_4x12_a(r0, a0, a1, a2, &(M_a[0]));
    Goldilocks::spmv_avx_4x12_a(r1, a0, a1, a2, &(M_a[12]));
    Goldilocks::spmv_avx_4x12_a(r2, a0, a1, a2, &(M_a[24]));
    Goldilocks::spmv_avx_4x12_a(r3, a0, a1, a2, &(M_a[36]));

    // Transpose: transform de 4x4 matrix stored in rows r0...r3 to the columns c0...c3
    __m256i t0 = _mm256_permute2f128_si256(r0, r2, 0b00100000);
    __m256i t1 = _mm256_permute2f128_si256(r1, r3, 0b00100000);
    __m256i t2 = _mm256_permute2f128_si256(r0, r2, 0b00110001);
    __m256i t3 = _mm256_permute2f128_si256(r1, r3, 0b00110001);
    __m256i c0 = _mm256_castpd_si256(_mm256_unpacklo_pd(_mm256_castsi256_pd(t0), _mm256_castsi256_pd(t1)));
    __m256i c1 = _mm256_castpd_si256(_mm256_unpackhi_pd(_mm256_castsi256_pd(t0), _mm256_castsi256_pd(t1)));
    __m256i c2 = _mm256_castpd_si256(_mm256_unpacklo_pd(_mm256_castsi256_pd(t2), _mm256_castsi256_pd(t3)));
    __m256i c3 = _mm256_castpd_si256(_mm256_unpackhi_pd(_mm256_castsi256_pd(t2), _mm256_castsi256_pd(t3)));

    // Add columns to obtain result
    __m256i sum0, sum1;
    add_avx(sum0, c0, c1);
    add_avx(sum1, c2, c3);
    add_avx(b, sum0, sum1);
}

// Dense matrix-vector product
// We assume coeficients of M_8 can be expressed with 8 bits (<256)
inline void Goldilocks::mmult_avx_4x12_8(__m256i &b, const __m256i &a0, const __m256i &a1, const __m256i &a2, const Goldilocks::Element M_8[48])
{
    // Generate matrix 4x4
    __m256i r0, r1, r2, r3;
    Goldilocks::spmv_avx_4x12_8(r0, a0, a1, a2, &(M_8[0]));
    Goldilocks::spmv_avx_4x12_8(r1, a0, a1, a2, &(M_8[12]));
    Goldilocks::spmv_avx_4x12_8(r2, a0, a1, a2, &(M_8[24]));
    Goldilocks::spmv_avx_4x12_8(r3, a0, a1, a2, &(M_8[36]));

    // Transpose: transform de 4x4 matrix stored in rows r0...r3 to the columns c0...c3
    __m256i t0 = _mm256_permute2f128_si256(r0, r2, 0b00100000);
    __m256i t1 = _mm256_permute2f128_si256(r1, r3, 0b00100000);
    __m256i t2 = _mm256_permute2f128_si256(r0, r2, 0b00110001);
    __m256i t3 = _mm256_permute2f128_si256(r1, r3, 0b00110001);
    __m256i c0 = _mm256_castpd_si256(_mm256_unpacklo_pd(_mm256_castsi256_pd(t0), _mm256_castsi256_pd(t1)));
    __m256i c1 = _mm256_castpd_si256(_mm256_unpackhi_pd(_mm256_castsi256_pd(t0), _mm256_castsi256_pd(t1)));
    __m256i c2 = _mm256_castpd_si256(_mm256_unpacklo_pd(_mm256_castsi256_pd(t2), _mm256_castsi256_pd(t3)));
    __m256i c3 = _mm256_castpd_si256(_mm256_unpackhi_pd(_mm256_castsi256_pd(t2), _mm256_castsi256_pd(t3)));

    // Add columns to obtain result
    __m256i sum0, sum1;
    add_avx(sum0, c0, c1);
    add_avx(sum1, c2, c3);
    add_avx(b, sum0, sum1);
}

inline void Goldilocks::mmult_avx(__m256i &a0, __m256i &a1, __m256i &a2, const Goldilocks::Element M[144])
{
    __m256i b0, b1, b2;
    Goldilocks::mmult_avx_4x12(b0, a0, a1, a2, &(M[0]));
    Goldilocks::mmult_avx_4x12(b1, a0, a1, a2, &(M[48]));
    Goldilocks::mmult_avx_4x12(b2, a0, a1, a2, &(M[96]));
    a0 = b0;
    a1 = b1;
    a2 = b2;
}
// we assume that M_a aligned on a 32-byte boundary
inline void Goldilocks::mmult_avx_a(__m256i &a0, __m256i &a1, __m256i &a2, const Goldilocks::Element M_a[144])
{
    __m256i b0, b1, b2;
    Goldilocks::mmult_avx_4x12_a(b0, a0, a1, a2, &(M_a[0]));
    Goldilocks::mmult_avx_4x12_a(b1, a0, a1, a2, &(M_a[48]));
    Goldilocks::mmult_avx_4x12_a(b2, a0, a1, a2, &(M_a[96]));
    a0 = b0;
    a1 = b1;
    a2 = b2;
}
// We assume coeficients of M_8 can be expressed with 8 bits (<256)
inline void Goldilocks::mmult_avx_8(__m256i &a0, __m256i &a1, __m256i &a2, const Goldilocks::Element M_8[144])
{
    __m256i b0, b1, b2;
    Goldilocks::mmult_avx_4x12_8(b0, a0, a1, a2, &(M_8[0]));
    Goldilocks::mmult_avx_4x12_8(b1, a0, a1, a2, &(M_8[48]));
    Goldilocks::mmult_avx_4x12_8(b2, a0, a1, a2, &(M_8[96]));
    a0 = b0;
    a1 = b1;
    a2 = b2;
}

/*
    Implementations for expressions:
*/

inline void Goldilocks::copy_avx(__m256i &dst_, const __m256i &src_)
{
    dst_ = src_;
}


inline void Goldilocks::op_avx(uint64_t op, __m256i &c_, const __m256i &a_, const __m256i &b_)
{
    switch (op)
    {
    case 0:
        add_avx(c_, a_, b_);
        break;
    case 1:
        sub_avx(c_, a_, b_);
        break;
    case 2:
        mult_avx(c_, a_, b_);
        break;
    case 3:
        sub_avx(c_, b_, a_);
        break;
    default:
        assert(0);
        break;
    }
};
#endif