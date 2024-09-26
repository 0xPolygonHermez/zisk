#ifndef GOLDILOCKS_AVX512
#define GOLDILOCKS_AVX512
#ifdef __AVX512__
#include "goldilocks_base_field.hpp"
#include <immintrin.h>

// NOTATION:
// _c value is in canonical form
// _n negative P_n = -P
// _l low part of a variable: uint64 [31:0] or uint128 [63:0]
// _h high part of a variable: uint64 [63:32] or uint128 [127:64]
// _a alingned pointer
// _8 variable can be expressed in 8 bits (<256)

// OBSERVATIONS:
// 1.  a + b overflows iff (a + b) < a (AVX does not suport carry, this is the way to check)
// 2.  a - b underflows iff (a - b) > a (AVX does not suport carry, this is the way to check)

const __m512i P8 = _mm512_set_epi64(GOLDILOCKS_PRIME, GOLDILOCKS_PRIME, GOLDILOCKS_PRIME, GOLDILOCKS_PRIME, GOLDILOCKS_PRIME, GOLDILOCKS_PRIME, GOLDILOCKS_PRIME, GOLDILOCKS_PRIME);
const __m512i P8_n = _mm512_set_epi64(GOLDILOCKS_PRIME_NEG, GOLDILOCKS_PRIME_NEG, GOLDILOCKS_PRIME_NEG, GOLDILOCKS_PRIME_NEG, GOLDILOCKS_PRIME_NEG, GOLDILOCKS_PRIME_NEG, GOLDILOCKS_PRIME_NEG, GOLDILOCKS_PRIME_NEG);
const __m512i sqmask8 = _mm512_set_epi64(0x1FFFFFFFF, 0x1FFFFFFFF, 0x1FFFFFFFF, 0x1FFFFFFFF, 0x1FFFFFFFF, 0x1FFFFFFFF, 0x1FFFFFFFF, 0x1FFFFFFFF);

inline void Goldilocks::load_avx512(__m512i &a_, const Goldilocks::Element *a8)
{
    a_ = _mm512_loadu_si512((__m512i *)(a8));
}

inline void Goldilocks::load_avx512_a(__m512i &a_, const Goldilocks::Element *a8_a)
{
    a_ = _mm512_load_si512((__m512i *)(a8_a));
}

inline void Goldilocks::store_avx512(Goldilocks::Element *a8, const __m512i &a_)
{
    _mm512_storeu_si512((__m512i *)a8, a_);
}

inline void Goldilocks::store_avx512_a(Goldilocks::Element *a8_a, const __m512i &a_)
{
    _mm512_store_si512((__m512i *)a8_a, a_);
}

inline void Goldilocks::store_avx512(Goldilocks::Element *a8, uint64_t stride_a, const __m512i &a_){
    Goldilocks::Element a8_[8];
    _mm512_storeu_si512((__m512i *)a8_, a_);
    a8[0] = a8_[0];
    a8[stride_a] = a8_[1];
    a8[2*stride_a] = a8_[2];
    a8[3*stride_a] = a8_[3];
    a8[4*stride_a] = a8_[4];
    a8[5*stride_a] = a8_[5];
    a8[6*stride_a] = a8_[6];
    a8[7*stride_a] = a8_[7];
}

// Obtain cannonical representative of a,
// We assume a <= a_c+P
inline void Goldilocks::toCanonical_avx512(__m512i &a_c, const __m512i &a)
{
    __mmask8 result_mask = _mm512_cmpge_epu64_mask(a, P8);
    a_c = _mm512_mask_add_epi64(a, result_mask, a, P8_n);
}

inline void Goldilocks::add_avx512(__m512i &c, const __m512i &a, const __m512i &b)
{
    // Evaluate a_c
    __m512i a_c;
    toCanonical_avx512(a_c, a);

    // Addition
    __m512i c0 = _mm512_add_epi64(a_c, b);

    // correction if overflow (iff a_c > a_c+b )
    // if correction is necessari, note that as a_c+b <= P+2^64-1, a_c+b-P <= 2^64-1
    __mmask8 result_mask = _mm512_cmpgt_epu64_mask(a_c, c0);
    c = _mm512_mask_add_epi64(c0, result_mask, c0, P8_n);
}

// Assume b is in canonical form
inline void Goldilocks::add_avx512_b_c(__m512i &c, const __m512i &a, const __m512i &b_c)
{
    const __m512i c0 = _mm512_add_epi64(a, b_c);
    __mmask8 result_mask = _mm512_cmpgt_epu64_mask(a, c0);
    c = _mm512_mask_add_epi64(c0, result_mask, c0, P8_n);
}

inline void Goldilocks::sub_avx512(__m512i &c, const __m512i &a, const __m512i &b)
{
    __m512i b_c;
    toCanonical_avx512(b_c, b);
    const __m512i c0 = _mm512_sub_epi64(a, b_c);

    // correction if underflow (iff a < b_c)
    // if correction is necessari:
    // P > b_c > a =>  {(a-b_c) < 0 and  P+(a-b_c)< P } => 0  < (P-b_c)+a < P
    __mmask8 result_mask = _mm512_cmpgt_epu64_mask(b_c, a);
    c = _mm512_mask_add_epi64(c0, result_mask, c0, P8);
}

inline void Goldilocks::sub_avx512_b_c(__m512i &c, const __m512i &a, const __m512i &b_c)
{
    const __m512i c0 = _mm512_sub_epi64(a, b_c);
    __mmask8 result_mask = _mm512_cmpgt_epu64_mask(b_c, a);
    c = _mm512_mask_add_epi64(c0, result_mask, c0, P8);
}

inline void Goldilocks::mult_avx512(__m512i &c, const __m512i &a, const __m512i &b)
{
    __m512i c_h, c_l;
    mult_avx512_128(c_h, c_l, a, b);
    reduce_avx512_128_64(c, c_h, c_l);
}

// We assume coeficients of b_8 can be expressed with 8 bits (<256)
inline void Goldilocks::mult_avx512_8(__m512i &c, const __m512i &a, const __m512i &b_8)
{
    __m512i c_h, c_l;
    mult_avx512_72(c_h, c_l, a, b_8);
    reduce_avx512_96_64(c, c_h, c_l);
}

// The 128 bits of the result are stored in c_h[64:0]| c_l[64:0]
inline void Goldilocks::mult_avx512_128(__m512i &c_h, __m512i &c_l, const __m512i &a, const __m512i &b)
{

    // Obtain a_h and b_h in the lower 32 bits
    __m512i a_h = _mm512_castps_si512(_mm512_movehdup_ps(_mm512_castsi512_ps(a)));
    __m512i b_h = _mm512_castps_si512(_mm512_movehdup_ps(_mm512_castsi512_ps(b)));

    // c = (a_h+a_l)*(b_h+b_l)=a_h*b_h+a_h*b_l+a_l*b_h+a_l*b_l=c_hh+c_hl+cl_h+c_ll
    // note: _mm512_mul_epu32 uses only the lower 32bits of each chunk so a=a_l and b=b_l
    __m512i c_hh = _mm512_mul_epu32(a_h, b_h);
    __m512i c_hl = _mm512_mul_epu32(a_h, b);
    __m512i c_lh = _mm512_mul_epu32(a, b_h);
    __m512i c_ll = _mm512_mul_epu32(a, b);

    // Bignum addition
    // Ranges: c_hh[127:64], c_lh[95:32], c_hl[95:32], c_ll[63:0]
    // parts that intersect must be added

    // LOW PART:
    // 1: r0 = c_hl + c_ll_h
    //    does not overflow: c_hl <= (2^32-1)*(2^32-1)=2^64-2*2^32+1
    //                       c_ll_h <= 2^32-1
    //                       c_hl + c_ll_h <= 2^64-2^32
    __m512i c_ll_h = _mm512_srli_epi64(c_ll, 32);
    __m512i r0 = _mm512_add_epi64(c_hl, c_ll_h);

    // 2: r1 = r0_l + c_lh //does not overflow
    __m512i r0_l = _mm512_and_si512(r0, P8_n);
    __m512i r1 = _mm512_add_epi64(c_lh, r0_l);

    // 3: c_l = r1_l | c_ll_l
    __m512i r1_l = _mm512_castps_si512(_mm512_moveldup_ps(_mm512_castsi512_ps(r1)));
    __mmask16 mask = 0xAAAA;
    c_l = _mm512_mask_blend_epi32(mask, c_ll, r1_l);

    // HIGH PART: c_h = c_hh + r0_h + r1_h
    // 1: r2 = r0_h + c_hh
    //    does not overflow: c_hh <= (2^32-1)*(2^32-1)=2^64-2*2^32+1
    //                       r0_h <= 2^32-1
    //                       r0_h + c_hh <= 2^64-2^32
    __m512i r0_h = _mm512_srli_epi64(r0, 32);
    __m512i r2 = _mm512_add_epi64(c_hh, r0_h);

    // 2: c_h = r3 + r1_h
    //    does not overflow: r2 <= 2^64-2^32
    //                       r1_h <= 2^32-1
    //                       r2 + r1_h <= 2^64-1
    __m512i r1_h = _mm512_srli_epi64(r1, 32);
    c_h = _mm512_add_epi64(r2, r1_h);
}

// The 72 bits the result are stored in c_h[32:0] | c_l[64:0]
inline void Goldilocks::mult_avx512_72(__m512i &c_h, __m512i &c_l, const __m512i &a, const __m512i &b)
{
    // Obtain a_h in the lower 32 bits
    __m512i a_h = _mm512_castps_si512(_mm512_movehdup_ps(_mm512_castsi512_ps(a)));

    // c = (a_h+a_l)*(b_l)=a_h*b_l+a_l*b_l=c_hl+c_ll
    // note: _mm512_mul_epu32 uses only the lower 32bits of each chunk so a=a_l and b=b_l
    __m512i c_hl = _mm512_mul_epu32(a_h, b);
    __m512i c_ll = _mm512_mul_epu32(a, b);

    // Bignum addition
    // Ranges: c_hl[95:32], c_ll[63:0]
    // parts that intersect must be added

    // LOW PART:
    // 1: r0 = c_hl + c_ll_h
    //    does not overflow: c_hl <= (2^32-1)*(2^8-1)< 2^40
    //                       c_ll_h <= 2^32-1
    //                       c_hl + c_ll_h <= 2^41
    __m512i c_ll_h = _mm512_srli_epi64(c_ll, 32);
    __m512i r0 = _mm512_add_epi64(c_hl, c_ll_h);

    // 2: c_l = r0_l | c_ll_l
    __m512i r0_l = _mm512_castps_si512(_mm512_moveldup_ps(_mm512_castsi512_ps(r0)));
    __mmask16 mask = 0xAAAA;
    c_l = _mm512_mask_blend_epi32(mask, c_ll, r0_l);

    // HIGH PART: c_h =  r0_h
    c_h = _mm512_srli_epi64(r0, 32);
}

// notes:
// P = 2^64-2^32+1
// P_n = 2^32-1
// 2^32*P_n = 2^32*(2^32-1) = 2^64-2^32 = P-1
// 2^64 = P+P_n => [2^64]=[P_n]
// process:
// c % P = [c] = [c_h*2^64+c_l] = [c_h*P_n+c_l] = [c_hh*2^32*P_n+c_hl*P_n+c_l] =
//             = [c_hh(P-1) +c_hl*P_n+c_l] = [c_l-c_hh+c_hl*P_n]
inline void Goldilocks::reduce_avx512_128_64(__m512i &c, const __m512i &c_h, const __m512i &c_l)
{
    __m512i c_hh = _mm512_srli_epi64(c_h, 32);
    __m512i c1;
    sub_avx512_b_c(c1, c_l, c_hh);
    __m512i c2 = _mm512_mul_epu32(c_h, P8_n); // c_hl*P_n (only 32bits of c_h useds)
    add_avx512_b_c(c, c1, c2);
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
inline void Goldilocks::reduce_avx512_96_64(__m512i &c, const __m512i &c_h, const __m512i &c_l)
{
    __m512i c1 = _mm512_mul_epu32(c_h, P8_n); // c_hl*P_n (only 32bits of c_h useds)
    add_avx512_b_c(c, c_l, c1);               // c2 = c_hl*P_n <= (2^32-1)*(2^32-1) <= 2^64 -2^33+1 < P
}

inline void Goldilocks::square_avx512(__m512i &c, __m512i &a)
{
    __m512i c_h, c_l;
    square_avx512_128(c_h, c_l, a);
    reduce_avx512_128_64(c, c_h, c_l);
}

inline void Goldilocks::square_avx512_128(__m512i &c_h, __m512i &c_l, const __m512i &a)
{
    // Obtain a_h
    __m512i a_h = _mm512_castps_si512(_mm512_movehdup_ps(_mm512_castsi512_ps(a)));

    // c = (a_h+a_l)*(a_h*a_l)=a_h*a_h+2*a_h*a_l+a_l*a_l=c_hh+2*c_hl+c_ll
    // note: _mm256_mul_epu32 uses only the lower 32bits of each chunk so a=a_l
    __m512i c_hh = _mm512_mul_epu32(a_h, a_h);
    __m512i c_lh = _mm512_mul_epu32(a, a_h); // used as 2^c_lh
    __m512i c_ll = _mm512_mul_epu32(a, a);

    // Bignum addition
    // Ranges: c_hh[127:64], c_lh[95:32], 2*c_lh[96:33],c_ll[63:0]
    //         c_ll_h[63:33]
    // parts that intersect must be added

    // LOW PART:
    // 1: r0 = c_lh + c_ll_h (31 bits)
    // Does not overflow c_lh <= (2^32-1)*(2^32-1)=2^64-2*2^32+1
    //                   c_ll_h <= 2^31-1
    //                   r0 <= 2^64-2^33+2^31
    __m512i c_ll_h = _mm512_srli_epi64(c_ll, 33); // yes 33, low part of 2*c_lh is [31:0]
    __m512i r0 = _mm512_add_epi64(c_lh, c_ll_h);

    // 2: c_l = r0_l (31 bits) | c_ll_l (33 bits)
    __m512i r0_l = _mm512_slli_epi64(r0, 33);
    __m512i c_ll_l = _mm512_and_si512(c_ll, sqmask8);
    c_l = _mm512_add_epi64(r0_l, c_ll_l);

    // HIGH PART:
    // 1: c_h = r0_h (33 bits) + c_hh (64 bits)
    // Does not overflow c_hh <= (2^32-1)*(2^32-1)=2^64-2*2^32+1
    //                   r0 <s= 2^64-2^33+2^31 => r0_h <= 2^33-2 (_h means 33 bits here!)
    //                   Dem: r0_h=2^33-1 => r0 >= r0_h*2^31=2^64-2^31!!
    //                                  contradiction with what we saw above
    //                   c_hh + c0_h <= 2^64-2^33+1+2^33-2 <= 2^64-1
    __m512i r0_h = _mm512_srli_epi64(r0, 31);
    c_h = _mm512_add_epi64(c_hh, r0_h);
}

// Data for two arrays of 12 compoments is interleaved: b1=[b[0..3]|b[8..11]|b[16..18]], b2=[b[4..7]|b[12..15]|b[19..23]], first half of a0,a1,a2 is operated with b1, second half with b2.
inline void Goldilocks::dot_avx512(Element c[2], const __m512i &a0, const __m512i &a1, const __m512i &a2, const Element b[12])
{
    __m512i c_;
    spmv_avx512_4x12(c_, a0, a1, a2, b);
    Goldilocks::Element cc[8];
    store_avx512(cc, c_);
    c[0] = (cc[0] + cc[1]) + (cc[2] + cc[3]);
    c[1] = (cc[4] + cc[5]) + (cc[6] + cc[7]);
}

// Sparse matrix-vector product (8x24 sparce matrix formed of three diagonal blocks os size 8x8)
// c[i]=Sum_j(aj[i]*b[j*4+i]) 0<=i<8 0<=j<3
// Data for two arrays of 12 compoments is interleaved: b1=[b[0..3]|b[8..11]|b[16..18]], b2=[b[4..7]|b[12..15]|b[19..23]], first half of a0,a1,a2 is operated with b1, second half with b2.
inline void Goldilocks::spmv_avx512_4x12(__m512i &c, const __m512i &a0, const __m512i &a1, const __m512i &a2, const Goldilocks::Element b[12])
{

    // load b into avx registers, latter
    __m512i b0 = _mm512_set4_epi64(b[3].fe, b[2].fe, b[1].fe, b[0].fe);
    __m512i b1 = _mm512_set4_epi64(b[7].fe, b[6].fe, b[5].fe, b[4].fe);
    __m512i b2 = _mm512_set4_epi64(b[11].fe, b[10].fe, b[9].fe, b[8].fe);

    __m512i c0, c1, c2;
    mult_avx512(c0, a0, b0);
    mult_avx512(c1, a1, b1);
    mult_avx512(c2, a2, b2);

    __m512i c_;
    add_avx512_b_c(c_, c0, c1);
    add_avx512_b_c(c, c_, c2);
}

// Sparse matrix-vector product (8x24 sparce matrix formed of three diagonal blocks os size 8x8)
// c[i]=Sum_j(aj[i]*b[j*4+i]) 0<=i<4 0<=j<3
// We assume b_a aligned on a 64-byte boundary
inline void Goldilocks::spmv_avx512_4x12_8(__m512i &c, const __m512i &a0, const __m512i &a1, const __m512i &a2, const Goldilocks::Element b[12])
{

    // load b into avx registers
    __m512i b0 = _mm512_set4_epi64(b[3].fe, b[2].fe, b[1].fe, b[0].fe);
    __m512i b1 = _mm512_set4_epi64(b[7].fe, b[6].fe, b[5].fe, b[4].fe);
    __m512i b2 = _mm512_set4_epi64(b[11].fe, b[10].fe, b[9].fe, b[8].fe);

    __m512i c0_h, c1_h, c2_h;
    __m512i c0_l, c1_l, c2_l;
    mult_avx512_72(c0_h, c0_l, a0, b0);
    mult_avx512_72(c1_h, c1_l, a1, b1);
    mult_avx512_72(c2_h, c2_l, a2, b2);

    __m512i c_h, c_l, aux_h, aux_l;

    add_avx512(aux_l, c0_l, c1_l);
    add_avx512(c_l, aux_l, c2_l);

    aux_h = _mm512_add_epi64(c0_h, c1_h);
    c_h = _mm512_add_epi64(aux_h, c2_h);

    reduce_avx512_96_64(c, c_h, c_l);
}

// Dense matrix-vector product
inline void Goldilocks::mmult_avx512_4x12(__m512i &b, const __m512i &a0, const __m512i &a1, const __m512i &a2, const Goldilocks::Element M[48])
{
    // Generate matrix 4x4
    __m512i r0, r1, r2, r3;
    Goldilocks::spmv_avx512_4x12(r0, a0, a1, a2, &(M[0]));
    Goldilocks::spmv_avx512_4x12(r1, a0, a1, a2, &(M[12]));
    Goldilocks::spmv_avx512_4x12(r2, a0, a1, a2, &(M[24]));
    Goldilocks::spmv_avx512_4x12(r3, a0, a1, a2, &(M[36]));

    __m512i indx1 = _mm512_set_epi64(13, 12, 5, 4, 9, 8, 1, 0);
    __m512i indx2 = _mm512_set_epi64(15, 14, 7, 6, 11, 10, 3, 2);

    __m512i t0 = _mm512_permutex2var_epi64(r0, indx1, r2);
    __m512i t1 = _mm512_permutex2var_epi64(r1, indx1, r3);
    __m512i t2 = _mm512_permutex2var_epi64(r0, indx2, r2);
    __m512i t3 = _mm512_permutex2var_epi64(r1, indx2, r3);

    __m512i c0 = _mm512_castpd_si512(_mm512_unpacklo_pd(_mm512_castsi512_pd(t0), _mm512_castsi512_pd(t1)));
    __m512i c1 = _mm512_castpd_si512(_mm512_unpackhi_pd(_mm512_castsi512_pd(t0), _mm512_castsi512_pd(t1)));
    __m512i c2 = _mm512_castpd_si512(_mm512_unpacklo_pd(_mm512_castsi512_pd(t2), _mm512_castsi512_pd(t3)));
    __m512i c3 = _mm512_castpd_si512(_mm512_unpackhi_pd(_mm512_castsi512_pd(t2), _mm512_castsi512_pd(t3)));

    // Add columns to obtain result
    __m512i sum0, sum1;
    add_avx512_b_c(sum0, c0, c1);
    add_avx512_b_c(sum1, c2, c3);
    add_avx512_b_c(b, sum0, sum1);
}

// Dense matrix-vector product
inline void Goldilocks::mmult_avx512_4x12_8(__m512i &b, const __m512i &a0, const __m512i &a1, const __m512i &a2, const Goldilocks::Element M[48])
{
    // Generate matrix 4x4
    __m512i r0, r1, r2, r3;
    Goldilocks::spmv_avx512_4x12_8(r0, a0, a1, a2, &(M[0]));
    Goldilocks::spmv_avx512_4x12_8(r1, a0, a1, a2, &(M[12]));
    Goldilocks::spmv_avx512_4x12_8(r2, a0, a1, a2, &(M[24]));
    Goldilocks::spmv_avx512_4x12_8(r3, a0, a1, a2, &(M[36]));

    __m512i indx1 = _mm512_set_epi64(13, 12, 5, 4, 9, 8, 1, 0);
    __m512i indx2 = _mm512_set_epi64(15, 14, 7, 6, 11, 10, 3, 2);

    __m512i t0 = _mm512_permutex2var_epi64(r0, indx1, r2);
    __m512i t1 = _mm512_permutex2var_epi64(r1, indx1, r3);
    __m512i t2 = _mm512_permutex2var_epi64(r0, indx2, r2);
    __m512i t3 = _mm512_permutex2var_epi64(r1, indx2, r3);

    __m512i c0 = _mm512_castpd_si512(_mm512_unpacklo_pd(_mm512_castsi512_pd(t0), _mm512_castsi512_pd(t1)));
    __m512i c1 = _mm512_castpd_si512(_mm512_unpackhi_pd(_mm512_castsi512_pd(t0), _mm512_castsi512_pd(t1)));
    __m512i c2 = _mm512_castpd_si512(_mm512_unpacklo_pd(_mm512_castsi512_pd(t2), _mm512_castsi512_pd(t3)));
    __m512i c3 = _mm512_castpd_si512(_mm512_unpackhi_pd(_mm512_castsi512_pd(t2), _mm512_castsi512_pd(t3)));

    // Add columns to obtain result
    __m512i sum0, sum1;
    add_avx512_b_c(sum0, c0, c1);
    add_avx512_b_c(sum1, c2, c3);
    add_avx512_b_c(b, sum0, sum1);
}

inline void Goldilocks::mmult_avx512(__m512i &a0, __m512i &a1, __m512i &a2, const Goldilocks::Element M[144])
{
    __m512i b0, b1, b2;
    Goldilocks::mmult_avx512_4x12(b0, a0, a1, a2, &(M[0]));
    Goldilocks::mmult_avx512_4x12(b1, a0, a1, a2, &(M[48]));
    Goldilocks::mmult_avx512_4x12(b2, a0, a1, a2, &(M[96]));
    a0 = b0;
    a1 = b1;
    a2 = b2;
}

// We assume coeficients of M_8 can be expressed with 8 bits (<256)
inline void Goldilocks::mmult_avx512_8(__m512i &a0, __m512i &a1, __m512i &a2, const Goldilocks::Element M_8[144])
{
    __m512i b0, b1, b2;
    Goldilocks::mmult_avx512_4x12_8(b0, a0, a1, a2, &(M_8[0]));
    Goldilocks::mmult_avx512_4x12_8(b1, a0, a1, a2, &(M_8[48]));
    Goldilocks::mmult_avx512_4x12_8(b2, a0, a1, a2, &(M_8[96]));
    a0 = b0;
    a1 = b1;
    a2 = b2;
}

/*
    Implementations for expressions:
*/

inline void Goldilocks::copy_avx512(__m512i &dst_, const __m512i &src_)
{
    dst_ = src_;
}

inline void Goldilocks::op_avx512(uint64_t op, __m512i &c_, const __m512i &a_, const __m512i &b_)
{
    switch (op)
    {
    case 0:
        add_avx512(c_, a_, b_);
        break;
    case 1:
        sub_avx512(c_, a_, b_);
        break;
    case 2:
        mult_avx512(c_, a_, b_);
        break;
    case 3:
        sub_avx512(c_, b_, a_);
        break;
    default:
        assert(0);
        break;
    }
};
#endif
#endif
