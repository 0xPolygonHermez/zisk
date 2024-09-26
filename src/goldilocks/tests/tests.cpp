#include <gtest/gtest.h>
#include <iostream>

#include "../src/goldilocks_base_field.hpp"
#include "../src/goldilocks_cubic_extension.hpp"
#include "../src/poseidon_goldilocks.hpp"
#include "../src/ntt_goldilocks.hpp"
#include "../src/merklehash_goldilocks.hpp"
#include <immintrin.h>

#define FFT_SIZE (1 << 4)
#define NUM_REPS 5
#define BLOWUP_FACTOR 1
#define NUM_COLUMNS 8
#define NPHASES 4
#define NCOLS_HASH 128
#define NROWS_HASH (1 << 6)

TEST(GOLDILOCKS_TEST, one)
{
    uint64_t a = 1;
    uint64_t b = 1 + GOLDILOCKS_PRIME;
    std::string c = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1

    Goldilocks::Element ina1 = Goldilocks::fromU64(a);
    Goldilocks::Element ina2 = Goldilocks::fromS32(a);
    Goldilocks::Element ina3 = Goldilocks::fromString(std::to_string(a));
    Goldilocks::Element inb1 = Goldilocks::fromU64(b);
    Goldilocks::Element inc1 = Goldilocks::fromString(c);

    ASSERT_EQ(Goldilocks::toU64(ina1), a);
    ASSERT_EQ(Goldilocks::toU64(ina2), a);
    ASSERT_EQ(Goldilocks::toU64(ina3), a);
    ASSERT_EQ(Goldilocks::toU64(inb1), a);
    ASSERT_EQ(Goldilocks::toU64(inc1), a);
}

TEST(GOLDILOCKS_TEST, add)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element p_1 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    ASSERT_EQ(Goldilocks::toU64(p_1 + p_1), 2);

    Goldilocks::Element max = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFF);
    ASSERT_EQ(Goldilocks::toU64(max + max), 0X1FFFFFFFC);

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);

    ASSERT_EQ(Goldilocks::toU64(inE1 + inE2), in1 + in2);
    ASSERT_EQ(Goldilocks::toU64(inE1 + inE2 + inE3), in1 + in2 + 1);
    ASSERT_EQ(Goldilocks::toU64(inE1 + inE2 + inE3 + inE4), 1);

    // Edge case (double carry)
    Goldilocks::Element a1 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element a2 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF));
    Goldilocks::Element b1 = (a1 + a2);
    Goldilocks::Element b2 = (b1 + b1);
    ASSERT_EQ(Goldilocks::toU64(b2), 0x200000002);
}
TEST(GOLDILOCKS_TEST, add_avx)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element p_1 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    Goldilocks::Element max = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFFULL);

    Goldilocks::Element a1 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element a2 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF));

    Goldilocks::Element *a = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *b = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *c = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));

    a[0] = p_1;
    a[1] = a1;
    a[2] = inE1;
    a[3] = max;

    b[0] = p_1;
    b[1] = a2;
    b[2] = inE2;
    b[3] = max;

    __m256i a_;
    __m256i b_;
    __m256i c_;

    Goldilocks::load_avx(a_, a);
    Goldilocks::set_avx(b_, b[0], b[1], b[2], b[3]); // equivalent to load
    Goldilocks::add_avx(c_, a_, b_);
    Goldilocks::store_avx(c, c_);

    ASSERT_EQ(Goldilocks::toU64(a[0] + b[0]), Goldilocks::toU64(c[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] + b[1]), Goldilocks::toU64(c[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] + b[2]), Goldilocks::toU64(c[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] + b[3]), Goldilocks::toU64(c[3]));

    a[0] = inE3;
    a[1] = c[1];
    a[2] = inE4;
    a[3] = max;

    Goldilocks::load_avx(a_, a);
    Goldilocks::add_avx(b_, a_, c_);
    Goldilocks::store_avx(b, b_);

    ASSERT_EQ(Goldilocks::toU64(a[0] + c[0]), Goldilocks::toU64(b[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] + c[1]), Goldilocks::toU64(b[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] + c[2]), Goldilocks::toU64(b[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] + c[3]), Goldilocks::toU64(b[3]));

    free(a);
    free(b);
    free(c);
}
#ifdef __AVX512__
TEST(GOLDILOCKS_TEST, add_avx512)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element p_1 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    Goldilocks::Element max = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFFULL);
    Goldilocks::Element a1 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element a2 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF));

    Goldilocks::Element *a = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *b = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *c = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));

    a[0] = p_1;
    a[1] = a1;
    a[2] = inE1;
    a[3] = max;
    a[4] = max;
    a[5] = inE4;
    a[6] = inE1;
    a[7] = inE3;

    b[0] = p_1;
    b[1] = a2;
    b[2] = inE2;
    b[3] = max;
    b[4] = inE1;
    b[5] = inE2;
    b[6] = inE3;
    b[7] = inE4;

    __m512i a_;
    __m512i b_;
    __m512i c_;

    Goldilocks::load_avx512(a_, a);
    Goldilocks::load_avx512(b_, b);
    Goldilocks::add_avx512(c_, a_, b_);
    Goldilocks::store_avx512(c, c_);

    ASSERT_EQ(Goldilocks::toU64(a[0] + b[0]), Goldilocks::toU64(c[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] + b[1]), Goldilocks::toU64(c[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] + b[2]), Goldilocks::toU64(c[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] + b[3]), Goldilocks::toU64(c[3]));
    ASSERT_EQ(Goldilocks::toU64(a[4] + b[4]), Goldilocks::toU64(c[4]));
    ASSERT_EQ(Goldilocks::toU64(a[5] + b[5]), Goldilocks::toU64(c[5]));
    ASSERT_EQ(Goldilocks::toU64(a[6] + b[6]), Goldilocks::toU64(c[6]));
    ASSERT_EQ(Goldilocks::toU64(a[7] + b[7]), Goldilocks::toU64(c[7]));

    free(a);
    free(b);
    free(c);
}
#endif

TEST(GOLDILOCKS_TEST, sub)
{

    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);

    ASSERT_EQ(Goldilocks::toU64(inE1 - inE2), GOLDILOCKS_PRIME + in1 - in2);
    ASSERT_EQ(Goldilocks::toU64(inE1 - inE2 - inE3), GOLDILOCKS_PRIME + in1 - in2 - 1);
    ASSERT_EQ(Goldilocks::toU64(inE1 - inE2 - inE3 - inE4), 5);

    Goldilocks::Element a1 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000LL));
    Goldilocks::Element a2 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFFLL));

    Goldilocks::Element a3 = (a1 + a2);
    Goldilocks::Element b2 = Goldilocks::zero() - a3;
    ASSERT_EQ(Goldilocks::toU64(b2), Goldilocks::from_montgomery(0XFFFFFFFE00000003LL));
}
TEST(GOLDILOCKS_TEST, sub_avx)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element p_1 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    Goldilocks::Element max = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFFULL);
    Goldilocks::Element a1 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element a2 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF));

    Goldilocks::Element *a = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *b = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *c = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));

    a[0] = inE1;
    a[1] = inE2;
    a[2] = Goldilocks::zero();
    a[3] = a1;

    b[0] = inE1;
    b[1] = inE3;
    b[2] = inE4;
    b[3] = max;

    __m256i a_;
    __m256i b_;
    __m256i c_;

    Goldilocks::load_avx(a_, a);
    Goldilocks::set_avx(b_, b[0], b[1], b[2], b[3]); // equivalent to load
    Goldilocks::sub_avx(c_, a_, b_);
    Goldilocks::store_avx(c, c_);

    ASSERT_EQ(Goldilocks::toU64(a[0] - b[0]), Goldilocks::toU64(c[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] - b[1]), Goldilocks::toU64(c[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] - b[2]), Goldilocks::toU64(c[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] - b[3]), Goldilocks::toU64(c[3]));

    a[0] = p_1;
    a[1] = a2;
    a[2] = Goldilocks::zero();
    a[3] = max;

    Goldilocks::load_avx(a_, a);
    Goldilocks::sub_avx(b_, a_, c_);
    Goldilocks::store_avx(b, b_);

    ASSERT_EQ(Goldilocks::toU64(a[0] - c[0]), Goldilocks::toU64(b[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] - c[1]), Goldilocks::toU64(b[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] - c[2]), Goldilocks::toU64(b[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] - c[3]), Goldilocks::toU64(b[3]));

    // edge case:
    Goldilocks::Element a0 = Goldilocks::fromU64(1);
    Goldilocks::Element b0 = Goldilocks::fromString("6824165416642549846");
    Goldilocks::Element b1 = Goldilocks::fromString("13754891152847927955");
    Goldilocks::Element b2 = Goldilocks::fromString("17916068787382203463");
    Goldilocks::Element b3 = Goldilocks::fromU64(18446744071248801682ULL);

    a[0] = a0;
    a[1] = a0;
    a[2] = a0;
    a[3] = a0;

    b[0] = b0;
    b[1] = b1;
    b[2] = b2;
    b[3] = b3;

    Goldilocks::load_avx(a_, a);
    Goldilocks::load_avx(b_, b);
    Goldilocks::sub_avx(c_, a_, b_);
    Goldilocks::store_avx(c, c_);

    ASSERT_EQ(Goldilocks::toU64(a[0] - b[0]), Goldilocks::toU64(c[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] - b[1]), Goldilocks::toU64(c[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] - b[2]), Goldilocks::toU64(c[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] - b[3]), Goldilocks::toU64(c[3]));

    free(a);
    free(b);
    free(c);
}
#ifdef __AVX512__
TEST(GOLDILOCKS_TEST, sub_avx512)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element inE5 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    Goldilocks::Element inE6 = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFFULL);
    Goldilocks::Element inE7 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element inE8 = Goldilocks::fromU64(1);
    Goldilocks::Element inE9 = Goldilocks::fromString("6824165416642549846");
    Goldilocks::Element inE10 = Goldilocks::fromString("13754891152847927955");
    Goldilocks::Element inE11 = Goldilocks::fromString("17916068787382203463");
    Goldilocks::Element inE12 = Goldilocks::fromU64(18446744071248801682ULL);

    Goldilocks::Element *a = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *b = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *c = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));

    a[0] = inE1;
    a[1] = inE2;
    a[2] = inE5;
    a[3] = inE7;
    a[4] = inE8;
    a[5] = inE8;
    a[6] = inE8;
    a[7] = inE8;

    b[0] = inE1;
    b[1] = inE3;
    b[2] = inE4;
    b[3] = inE6;
    b[4] = inE9;
    b[5] = inE10;
    b[6] = inE11;
    b[7] = inE12;

    __m512i a_;
    __m512i b_;
    __m512i c_;

    Goldilocks::load_avx512(a_, a);
    Goldilocks::load_avx512(b_, b);
    Goldilocks::sub_avx512(c_, a_, b_);
    Goldilocks::store_avx512(c, c_);

    ASSERT_EQ(Goldilocks::toU64(a[0] - b[0]), Goldilocks::toU64(c[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] - b[1]), Goldilocks::toU64(c[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] - b[2]), Goldilocks::toU64(c[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] - b[3]), Goldilocks::toU64(c[3]));
    ASSERT_EQ(Goldilocks::toU64(a[4] - b[4]), Goldilocks::toU64(c[4]));
    ASSERT_EQ(Goldilocks::toU64(a[5] - b[5]), Goldilocks::toU64(c[5]));
    ASSERT_EQ(Goldilocks::toU64(a[6] - b[6]), Goldilocks::toU64(c[6]));
    ASSERT_EQ(Goldilocks::toU64(a[7] - b[7]), Goldilocks::toU64(c[7]));

    free(a);
    free(b);
    free(c);
}
#endif

TEST(GOLDILOCKS_TEST, mul)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);

    ASSERT_EQ(Goldilocks::toU64(inE1 * inE2), in1 * in2);
    ASSERT_EQ(Goldilocks::toU64(inE1 * inE2 * inE3), in1 * in2);
    ASSERT_EQ(Goldilocks::toU64(inE1 * inE2 * inE3 * inE4), 0XFFFFFFFEFFFFFEBDLL);
}
TEST(GOLDILOCKS_TEST, mul_avx)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element p_1 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    Goldilocks::Element max = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFFULL);
    Goldilocks::Element a1 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element a2 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF));

    Goldilocks::Element *a = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *b = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *c = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));

    a[0] = inE1;
    a[1] = inE2;
    a[2] = Goldilocks::zero();
    a[3] = inE4;

    b[0] = a1;
    b[1] = inE3;
    b[2] = inE4;
    b[3] = p_1;

    __m256i a_;
    __m256i b_;
    __m256i c_;

    Goldilocks::load_avx(a_, a);
    Goldilocks::set_avx(b_, b[0], b[1], b[2], b[3]); // equivalent to load
    Goldilocks::mult_avx(c_, a_, b_);
    Goldilocks::store_avx(c, c_);

    ASSERT_EQ(Goldilocks::toU64(a[0] * b[0]), Goldilocks::toU64(c[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] * b[1]), Goldilocks::toU64(c[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] * b[2]), Goldilocks::toU64(c[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] * b[3]), Goldilocks::toU64(c[3]));

    a[0] = p_1;
    a[1] = a2;
    a[2] = Goldilocks::zero();
    a[3] = max;

    Goldilocks::load_avx(a_, a);
    Goldilocks::mult_avx(b_, a_, c_);
    Goldilocks::store_avx(b, b_);

    ASSERT_EQ(Goldilocks::toU64(a[0] * c[0]), Goldilocks::toU64(b[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] * c[1]), Goldilocks::toU64(b[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] * c[2]), Goldilocks::toU64(b[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] * c[3]), Goldilocks::toU64(b[3]));

    free(a);
    free(b);
    free(c);
}
#ifdef __AVX512__
TEST(GOLDILOCKS_TEST, mul_avx512)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element inE5 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    Goldilocks::Element inE6 = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFFULL);
    Goldilocks::Element inE7 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element inE8 = Goldilocks::fromU64(1);
    Goldilocks::Element inE9 = Goldilocks::fromString("6824165416642549846");
    Goldilocks::Element inE10 = Goldilocks::fromString("13754891152847927955");
    Goldilocks::Element inE11 = Goldilocks::fromString("17916068787382203463");
    Goldilocks::Element inE12 = Goldilocks::fromU64(18446744071248801682ULL);
    Goldilocks::Element inE13 = Goldilocks::zero();

    Goldilocks::Element *a = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *b = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *c = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));

    a[0] = inE1;
    a[1] = inE2;
    a[2] = inE13;
    a[3] = inE4;
    a[4] = inE9;
    a[5] = inE10;
    a[6] = inE11;
    a[7] = inE12;

    b[0] = inE5;
    b[1] = inE3;
    b[2] = inE4;
    b[3] = inE6;
    b[4] = inE7;
    b[5] = inE3;
    b[6] = inE4;
    b[7] = inE8;

    __m512i a_;
    __m512i b_;
    __m512i c_;

    Goldilocks::load_avx512(a_, a);
    Goldilocks::load_avx512(b_, b);
    Goldilocks::mult_avx512(c_, a_, b_);
    Goldilocks::store_avx512(c, c_);

    ASSERT_EQ(Goldilocks::toU64(a[0] * b[0]), Goldilocks::toU64(c[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] * b[1]), Goldilocks::toU64(c[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] * b[2]), Goldilocks::toU64(c[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] * b[3]), Goldilocks::toU64(c[3]));
    ASSERT_EQ(Goldilocks::toU64(a[4] * b[4]), Goldilocks::toU64(c[4]));
    ASSERT_EQ(Goldilocks::toU64(a[5] * b[5]), Goldilocks::toU64(c[5]));
    ASSERT_EQ(Goldilocks::toU64(a[6] * b[6]), Goldilocks::toU64(c[6]));
    ASSERT_EQ(Goldilocks::toU64(a[7] * b[7]), Goldilocks::toU64(c[7]));

    free(a);
    free(b);
    free(c);
}
#endif

TEST(GOLDILOCKS_TEST, mul_avx_8)
{
    int32_t in1 = 3;
    int32_t in2 = 9;
    int32_t in3 = 9;
    int32_t in4 = 100;
    int32_t in5 = 3;
    int32_t in6 = 9;
    int32_t in7 = 9;
    int32_t in8 = 100;

    Goldilocks::Element inE1 = Goldilocks::fromS32(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromS32(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element inE5 = Goldilocks::fromS32(in5);
    Goldilocks::Element inE6 = Goldilocks::fromS32(in6);
    Goldilocks::Element inE7 = Goldilocks::fromS32(in7);
    Goldilocks::Element inE8 = Goldilocks::fromS32(in8);

    Goldilocks::Element *a = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *b = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *c = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));

    a[0] = inE1;
    a[1] = inE2;
    a[2] = inE3;
    a[3] = inE4;

    b[0] = inE5;
    b[1] = inE6;
    b[2] = inE7;
    b[3] = inE8;

    __m256i a_;
    __m256i b_;
    __m256i c_;

    Goldilocks::load_avx(a_, a);
    Goldilocks::set_avx(b_, b[0], b[1], b[2], b[3]); // equivalent to load
    Goldilocks::mult_avx_8(c_, a_, b_);
    Goldilocks::store_avx(c, c_);

    ASSERT_EQ(Goldilocks::toU64(a[0] * b[0]), Goldilocks::toU64(c[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] * b[1]), Goldilocks::toU64(c[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] * b[2]), Goldilocks::toU64(c[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] * b[3]), Goldilocks::toU64(c[3]));

    free(a);
    free(b);
    free(c);
}
#ifdef __AVX512__
TEST(GOLDILOCKS_TEST, mul_avx512_8)
{
    int32_t in1 = 3;
    int32_t in2 = 9;
    int32_t in3 = 9;
    int32_t in4 = 100;
    int32_t in5 = 0;
    int32_t in6 = 1;
    int32_t in7 = 64;
    int32_t in8 = 2;

    Goldilocks::Element inE1 = Goldilocks::fromS32(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromS32(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element inE5 = Goldilocks::fromS32(in5);
    Goldilocks::Element inE6 = Goldilocks::fromS32(in6);
    Goldilocks::Element inE7 = Goldilocks::fromS32(in7);
    Goldilocks::Element inE8 = Goldilocks::fromS32(in8);

    Goldilocks::Element *a = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *b = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *c = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));

    a[0] = inE1;
    a[1] = inE2;
    a[2] = inE3;
    a[3] = inE4;
    a[4] = inE5;
    a[5] = inE7;
    a[6] = inE8;
    a[7] = inE2;

    b[0] = inE5;
    b[1] = inE6;
    b[2] = inE7;
    b[3] = inE8;
    b[4] = inE6;
    b[5] = inE1;
    b[6] = inE2;
    b[7] = inE3;

    __m512i a_;
    __m512i b_;
    __m512i c_;

    Goldilocks::load_avx512(a_, a);
    Goldilocks::load_avx512(b_, b);
    Goldilocks::mult_avx512_8(c_, a_, b_);
    Goldilocks::store_avx512(c, c_);

    ASSERT_EQ(Goldilocks::toU64(a[0] * b[0]), Goldilocks::toU64(c[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] * b[1]), Goldilocks::toU64(c[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] * b[2]), Goldilocks::toU64(c[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] * b[3]), Goldilocks::toU64(c[3]));
    ASSERT_EQ(Goldilocks::toU64(a[4] * b[4]), Goldilocks::toU64(c[4]));
    ASSERT_EQ(Goldilocks::toU64(a[5] * b[5]), Goldilocks::toU64(c[5]));
    ASSERT_EQ(Goldilocks::toU64(a[6] * b[6]), Goldilocks::toU64(c[6]));
    ASSERT_EQ(Goldilocks::toU64(a[7] * b[7]), Goldilocks::toU64(c[7]));

    free(a);
    free(b);
    free(c);
}
#endif

TEST(GOLDILOCKS_TEST, square_avx)
{
    uint64_t in1 = 3;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element a1 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));

    Goldilocks::Element *a = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *c = (Goldilocks::Element *)malloc(4 * (sizeof(Goldilocks::Element)));

    a[0] = inE1;
    a[1] = inE3;
    a[2] = Goldilocks::zero();
    a[3] = a1;

    __m256i a_;
    __m256i c_;

    Goldilocks::load_avx(a_, a);
    Goldilocks::square_avx(c_, a_);
    Goldilocks::store_avx(c, c_);

    ASSERT_EQ(Goldilocks::toU64(a[0] * a[0]), Goldilocks::toU64(c[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] * a[1]), Goldilocks::toU64(c[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] * a[2]), Goldilocks::toU64(c[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] * a[3]), Goldilocks::toU64(c[3]));

    Goldilocks::square_avx(a_, c_);
    Goldilocks::store_avx(a, a_);

    ASSERT_EQ(Goldilocks::toU64(c[0] * c[0]), Goldilocks::toU64(a[0]));
    ASSERT_EQ(Goldilocks::toU64(c[1] * c[1]), Goldilocks::toU64(a[1]));
    ASSERT_EQ(Goldilocks::toU64(c[2] * c[2]), Goldilocks::toU64(a[2]));
    ASSERT_EQ(Goldilocks::toU64(c[3] * c[3]), Goldilocks::toU64(a[3]));

    free(a);
    free(c);
}
#ifdef __AVX512__
TEST(GOLDILOCKS_TEST, square_avx512)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element inE5 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    Goldilocks::Element inE6 = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFFULL);
    Goldilocks::Element inE7 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element inE8 = Goldilocks::fromU64(1);
    Goldilocks::Element inE9 = Goldilocks::fromString("6824165416642549846");
    Goldilocks::Element inE10 = Goldilocks::fromString("13754891152847927955");
    Goldilocks::Element inE11 = Goldilocks::fromString("17916068787382203463");
    Goldilocks::Element inE12 = Goldilocks::fromU64(18446744071248801682ULL);
    Goldilocks::Element inE13 = Goldilocks::zero();

    Goldilocks::Element *a = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *b = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));
    Goldilocks::Element *c = (Goldilocks::Element *)malloc(8 * (sizeof(Goldilocks::Element)));

    a[0] = inE1;
    a[1] = inE2;
    a[2] = inE13;
    a[3] = inE4;
    a[4] = inE9;
    a[5] = inE10;
    a[6] = inE11;
    a[7] = inE12;

    b[0] = inE5;
    b[1] = inE3;
    b[2] = inE4;
    b[3] = inE6;
    b[4] = inE7;
    b[5] = inE3;
    b[6] = inE4;
    b[7] = inE8;

    __m512i a_;
    __m512i b_;
    __m512i c_;

    Goldilocks::load_avx512(a_, a);
    Goldilocks::square_avx512(c_, a_);
    Goldilocks::store_avx512(c, c_);

    ASSERT_EQ(Goldilocks::toU64(a[0] * a[0]), Goldilocks::toU64(c[0]));
    ASSERT_EQ(Goldilocks::toU64(a[1] * a[1]), Goldilocks::toU64(c[1]));
    ASSERT_EQ(Goldilocks::toU64(a[2] * a[2]), Goldilocks::toU64(c[2]));
    ASSERT_EQ(Goldilocks::toU64(a[3] * a[3]), Goldilocks::toU64(c[3]));
    ASSERT_EQ(Goldilocks::toU64(a[4] * a[4]), Goldilocks::toU64(c[4]));
    ASSERT_EQ(Goldilocks::toU64(a[5] * a[5]), Goldilocks::toU64(c[5]));
    ASSERT_EQ(Goldilocks::toU64(a[6] * a[6]), Goldilocks::toU64(c[6]));
    ASSERT_EQ(Goldilocks::toU64(a[7] * a[7]), Goldilocks::toU64(c[7]));

    Goldilocks::load_avx512(b_, b);
    Goldilocks::square_avx512(c_, b_);
    Goldilocks::store_avx512(c, c_);

    ASSERT_EQ(Goldilocks::toU64(b[0] * b[0]), Goldilocks::toU64(c[0]));
    ASSERT_EQ(Goldilocks::toU64(b[1] * b[1]), Goldilocks::toU64(c[1]));
    ASSERT_EQ(Goldilocks::toU64(b[2] * b[2]), Goldilocks::toU64(c[2]));
    ASSERT_EQ(Goldilocks::toU64(b[3] * b[3]), Goldilocks::toU64(c[3]));
    ASSERT_EQ(Goldilocks::toU64(b[4] * b[4]), Goldilocks::toU64(c[4]));
    ASSERT_EQ(Goldilocks::toU64(b[5] * b[5]), Goldilocks::toU64(c[5]));
    ASSERT_EQ(Goldilocks::toU64(b[6] * b[6]), Goldilocks::toU64(c[6]));
    ASSERT_EQ(Goldilocks::toU64(b[7] * b[7]), Goldilocks::toU64(c[7]));

    free(a);
    free(b);
    free(c);
}
#endif

TEST(GOLDILOCKS_TEST, dot_avx)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element p_1 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    Goldilocks::Element max = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFFULL);
    Goldilocks::Element a1 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element a2 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF));

    Goldilocks::Element *a = (Goldilocks::Element *)aligned_alloc(32, 12 * sizeof(Goldilocks::Element));
    Goldilocks::Element *b = (Goldilocks::Element *)aligned_alloc(32, 12 * sizeof(Goldilocks::Element));

    a[0] = inE1;
    a[1] = inE2;
    a[2] = Goldilocks::one();
    a[3] = inE3;
    a[4] = inE4;
    a[5] = max;
    a[6] = a1;
    a[7] = a2;
    a[8] = max * max;
    a[9] = p_1;
    a[10] = a1 * a1;
    a[11] = inE4 * p_1;

    b[0] = max;
    b[1] = a1;
    b[2] = inE4;
    b[3] = p_1;
    b[4] = Goldilocks::zero();
    b[5] = inE3;
    b[6] = inE1;
    b[7] = (a1 * inE1);
    b[8] = max;
    b[9] = inE4;
    b[10] = p_1;
    b[11] = Goldilocks::one();

    Goldilocks::Element dotp1 = Goldilocks::zero();
    for (int i = 0; i < 12; ++i)
    {
        dotp1 = dotp1 + a[i] * b[i];
    }

    __m256i a0_;
    __m256i a1_;
    __m256i a2_;

    Goldilocks::load_avx_a(a0_, &(a[0]));
    Goldilocks::load_avx_a(a1_, &(a[4]));
    Goldilocks::load_avx_a(a2_, &(a[8]));

    Goldilocks::Element dotp2 = Goldilocks::dot_avx(a0_, a1_, a2_, b);
    ASSERT_EQ(Goldilocks::toU64(dotp1), Goldilocks::toU64(dotp2));
    free(a);
    free(b);
}
#ifdef __AVX512__
TEST(GOLDILOCKS_TEST, dot_avx512)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element p_1 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    Goldilocks::Element max = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFFULL);
    Goldilocks::Element a1 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element a2 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF));

    Goldilocks::Element *a = (Goldilocks::Element *)aligned_alloc(64, 24 * sizeof(Goldilocks::Element));
    Goldilocks::Element *b = (Goldilocks::Element *)aligned_alloc(64, 12 * sizeof(Goldilocks::Element));

    a[0] = inE1;
    a[1] = inE2;
    a[2] = Goldilocks::one();
    a[3] = inE3;
    a[4] = inE1;
    a[5] = inE2;
    a[6] = Goldilocks::one();
    a[7] = inE3;
    a[8] = inE4;
    a[9] = max;
    a[10] = a1;
    a[11] = a2;
    a[12] = inE4;
    a[13] = max;
    a[14] = a1;
    a[15] = a2;
    a[16] = max * max;
    a[17] = p_1;
    a[18] = a1 * a1;
    a[19] = inE4 * p_1;
    a[20] = max * max;
    a[21] = p_1;
    a[22] = a1 * a1;
    a[23] = inE4 * p_1;

    b[0] = max;
    b[1] = a1;
    b[2] = inE4;
    b[3] = p_1;
    b[4] = Goldilocks::zero();
    b[5] = inE3;
    b[6] = inE1;
    b[7] = (a1 * inE1);
    b[8] = max;
    b[9] = inE4;
    b[10] = p_1;
    b[11] = Goldilocks::one();

    Goldilocks::Element dotp1 = Goldilocks::zero();
    for (int k = 0; k < 3; k += 1)
    {
        for (int i = 0; i < 4; ++i)
        {
            dotp1 = dotp1 + a[k * 8 + i] * b[k * 4 + i];
        }
    }

    __m512i a0_;
    __m512i a1_;
    __m512i a2_;

    // Not aligned
    Goldilocks::load_avx512(a0_, &(a[0]));
    Goldilocks::load_avx512(a1_, &(a[8]));
    Goldilocks::load_avx512(a2_, &(a[16]));

    Goldilocks::Element dotp2[2];

    Goldilocks::dot_avx512(dotp2, a0_, a1_, a2_, b);
    ASSERT_EQ(Goldilocks::toU64(dotp2[0]), Goldilocks::toU64(dotp2[1]));
    ASSERT_EQ(Goldilocks::toU64(dotp1), Goldilocks::toU64(dotp2[0]));

    free(a);
    free(b);
}
#endif

TEST(GOLDILOCKS_TEST, mult_avx_4x12)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element p_1 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    Goldilocks::Element max = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFFULL);
    Goldilocks::Element a1 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element a2 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF));

    Goldilocks::Element *a = (Goldilocks::Element *)aligned_alloc(32, 12 * sizeof(Goldilocks::Element));
    Goldilocks::Element *Mat = (Goldilocks::Element *)aligned_alloc(32, 48 * sizeof(Goldilocks::Element));
    Goldilocks::Element *b1 = (Goldilocks::Element *)aligned_alloc(32, 4 * sizeof(Goldilocks::Element));
    Goldilocks::Element *b2 = (Goldilocks::Element *)aligned_alloc(32, 4 * sizeof(Goldilocks::Element));

    a[0] = inE1;
    a[1] = inE2;
    a[2] = Goldilocks::one();
    a[3] = inE3;
    a[4] = inE4;
    a[5] = max;
    a[6] = a1;
    a[7] = a2;
    a[8] = max * max;
    a[9] = p_1;
    a[10] = a1 * a1;
    a[11] = inE4 * p_1;

    for (int i = 0; i < 4; ++i)
    {
        for (int j = 0; j < 12; ++j)
        {
            Mat[i * 12 + j] = PoseidonGoldilocksConstants::M[i][j];
        }
    }

    // product
    for (int i = 0; i < 4; ++i)
    {
        Goldilocks::Element sum = Goldilocks::zero();
        for (int j = 0; j < 12; ++j)
        {
            sum = sum + (Mat[i * 12 + j] * a[j]);
        }
        b1[i] = sum;
    }

    // avx product
    __m256i a0_;
    __m256i a1_;
    __m256i a2_;

    Goldilocks::load_avx(a0_, &(a[0]));
    Goldilocks::load_avx(a1_, &(a[4]));
    Goldilocks::load_avx(a2_, &(a[8]));
    __m256i b_;
    Goldilocks::mmult_avx_4x12(b_, a0_, a1_, a2_, &(Mat[0]));
    Goldilocks::store_avx(b2, b_);

    ASSERT_EQ(Goldilocks::toU64(b1[0]), Goldilocks::toU64(b2[0]));
    ASSERT_EQ(Goldilocks::toU64(b1[1]), Goldilocks::toU64(b2[1]));
    ASSERT_EQ(Goldilocks::toU64(b1[2]), Goldilocks::toU64(b2[2]));
    ASSERT_EQ(Goldilocks::toU64(b1[3]), Goldilocks::toU64(b2[3]));
    free(a);
    free(Mat);
    free(b1);
    free(b2);
}
#ifdef __AVX512__
TEST(GOLDILOCKS_TEST, mult_avx512_4x12)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element p_1 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    Goldilocks::Element max = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFFULL);
    Goldilocks::Element a1 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element a2 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF));

    Goldilocks::Element *a = (Goldilocks::Element *)aligned_alloc(64, 24 * sizeof(Goldilocks::Element));
    Goldilocks::Element *Mat = (Goldilocks::Element *)aligned_alloc(64, 48 * sizeof(Goldilocks::Element));
    Goldilocks::Element *b1 = (Goldilocks::Element *)aligned_alloc(64, 8 * sizeof(Goldilocks::Element));
    Goldilocks::Element *b2 = (Goldilocks::Element *)aligned_alloc(64, 8 * sizeof(Goldilocks::Element));

    a[0] = inE1;
    a[1] = inE2;
    a[2] = Goldilocks::one();
    a[3] = inE3;
    a[4] = inE1;
    a[5] = inE2;
    a[6] = Goldilocks::one();
    a[7] = inE3;

    a[8] = inE4;
    a[9] = max;
    a[10] = a1;
    a[11] = a2;
    a[12] = inE4;
    a[13] = max;
    a[14] = a1;
    a[15] = a2;

    a[16] = max * max;
    a[17] = p_1;
    a[18] = a1 * a1;
    a[19] = inE4 * p_1;
    a[20] = max * max;
    a[21] = p_1;
    a[22] = a1 * a1;
    a[23] = inE4 * p_1;

    for (int i = 0; i < 4; ++i)
    {
        for (int j = 0; j < 12; ++j)
        {
            Mat[i * 12 + j] = PoseidonGoldilocksConstants::M[i][j];
        }
    }

    // product
    for (int i = 0; i < 4; ++i)
    {
        Goldilocks::Element sum = Goldilocks::zero();
        for (int k = 0; k < 3; ++k)
        {
            for (int j = 0; j < 4; ++j)
            {
                sum = sum + (Mat[i * 12 + k * 4 + j] * a[k * 8 + j]);
            }
        }
        b1[i] = sum;
        b1[i + 4] = sum;
    }

    // avx product
    __m512i a0_;
    __m512i a1_;
    __m512i a2_;

    Goldilocks::load_avx512(a0_, &(a[0]));
    Goldilocks::load_avx512(a1_, &(a[8]));
    Goldilocks::load_avx512(a2_, &(a[16]));
    __m512i b_;
    Goldilocks::mmult_avx512_4x12(b_, a0_, a1_, a2_, &(Mat[0]));
    Goldilocks::store_avx512(b2, b_);

    ASSERT_EQ(Goldilocks::toU64(b1[0]), Goldilocks::toU64(b2[0]));
    ASSERT_EQ(Goldilocks::toU64(b1[1]), Goldilocks::toU64(b2[1]));
    ASSERT_EQ(Goldilocks::toU64(b1[2]), Goldilocks::toU64(b2[2]));
    ASSERT_EQ(Goldilocks::toU64(b1[3]), Goldilocks::toU64(b2[3]));
    ASSERT_EQ(Goldilocks::toU64(b1[4]), Goldilocks::toU64(b2[4]));
    ASSERT_EQ(Goldilocks::toU64(b1[5]), Goldilocks::toU64(b2[5]));
    ASSERT_EQ(Goldilocks::toU64(b1[6]), Goldilocks::toU64(b2[6]));
    ASSERT_EQ(Goldilocks::toU64(b1[7]), Goldilocks::toU64(b2[7]));

    // avx product small coeficients
    for (int i = 0; i < 48; ++i)
    {
        Mat[i].fe = Mat[i].fe % 256;
    }
    for (int i = 0; i < 4; ++i)
    {
        Goldilocks::Element sum = Goldilocks::zero();
        for (int k = 0; k < 3; ++k)
        {
            for (int j = 0; j < 4; ++j)
            {
                sum = sum + (Mat[i * 12 + k * 4 + j] * a[k * 8 + j]);
            }
        }
        b1[i] = sum;
        b1[i + 4] = sum;
    }

    Goldilocks::mmult_avx512_4x12_8(b_, a0_, a1_, a2_, &(Mat[0]));
    Goldilocks::store_avx512(b2, b_);

    ASSERT_EQ(Goldilocks::toU64(b1[0]), Goldilocks::toU64(b2[0]));
    ASSERT_EQ(Goldilocks::toU64(b1[1]), Goldilocks::toU64(b2[1]));
    ASSERT_EQ(Goldilocks::toU64(b1[2]), Goldilocks::toU64(b2[2]));
    ASSERT_EQ(Goldilocks::toU64(b1[3]), Goldilocks::toU64(b2[3]));
    ASSERT_EQ(Goldilocks::toU64(b2[4]), Goldilocks::toU64(b2[4]));
    ASSERT_EQ(Goldilocks::toU64(b2[5]), Goldilocks::toU64(b2[5]));
    ASSERT_EQ(Goldilocks::toU64(b2[6]), Goldilocks::toU64(b2[6]));
    ASSERT_EQ(Goldilocks::toU64(b2[7]), Goldilocks::toU64(b2[7]));

    free(a);
    free(Mat);
    free(b1);
    free(b2);
}
#endif

TEST(GOLDILOCKS_TEST, mmult_avx)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element p_1 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    Goldilocks::Element max = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFFULL);
    Goldilocks::Element a1 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element a2 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF));

    Goldilocks::Element *a = (Goldilocks::Element *)aligned_alloc(32, 12 * sizeof(Goldilocks::Element));
    Goldilocks::Element *Mat = (Goldilocks::Element *)aligned_alloc(32, 144 * sizeof(Goldilocks::Element));
    Goldilocks::Element *b = (Goldilocks::Element *)aligned_alloc(32, 12 * sizeof(Goldilocks::Element));

    a[0] = inE1;
    a[1] = inE2;
    a[2] = Goldilocks::one();
    a[3] = inE3;
    a[4] = inE4;
    a[5] = max;
    a[6] = a1;
    a[7] = a2;
    a[8] = max * max;
    a[9] = p_1;
    a[10] = a1 * a1;
    a[11] = inE4 * p_1;

    for (int i = 0; i < 12; ++i)
    {
        for (int j = 0; j < 12; ++j)
        {
            Mat[i * 12 + j] = PoseidonGoldilocksConstants::M[i][j];
        }
    }

    // product
    for (int i = 0; i < 12; ++i)
    {
        Goldilocks::Element sum = Goldilocks::zero();
        for (int j = 0; j < 12; ++j)
        {
            sum = sum + (Mat[i * 12 + j] * a[j]);
        }
        b[i] = sum;
    }

    // avx product
    __m256i a0_;
    __m256i a1_;
    __m256i a2_;

    Goldilocks::load_avx(a0_, &(a[0]));
    Goldilocks::load_avx(a1_, &(a[4]));
    Goldilocks::load_avx(a2_, &(a[8]));

    Goldilocks::mmult_avx(a0_, a1_, a2_, &(Mat[0]));

    Goldilocks::store_avx(&(a[0]), a0_);
    Goldilocks::store_avx(&(a[4]), a1_);
    Goldilocks::store_avx(&(a[8]), a2_);

    ASSERT_EQ(Goldilocks::toU64(b[0]), Goldilocks::toU64(a[0]));
    ASSERT_EQ(Goldilocks::toU64(b[1]), Goldilocks::toU64(a[1]));
    ASSERT_EQ(Goldilocks::toU64(b[2]), Goldilocks::toU64(a[2]));
    ASSERT_EQ(Goldilocks::toU64(b[3]), Goldilocks::toU64(a[3]));
    ASSERT_EQ(Goldilocks::toU64(b[4]), Goldilocks::toU64(a[4]));
    ASSERT_EQ(Goldilocks::toU64(b[5]), Goldilocks::toU64(a[5]));
    ASSERT_EQ(Goldilocks::toU64(b[6]), Goldilocks::toU64(a[6]));
    ASSERT_EQ(Goldilocks::toU64(b[7]), Goldilocks::toU64(a[7]));
    ASSERT_EQ(Goldilocks::toU64(b[8]), Goldilocks::toU64(a[8]));
    ASSERT_EQ(Goldilocks::toU64(b[9]), Goldilocks::toU64(a[9]));
    ASSERT_EQ(Goldilocks::toU64(b[10]), Goldilocks::toU64(a[10]));
    ASSERT_EQ(Goldilocks::toU64(b[11]), Goldilocks::toU64(a[11]));

    // avx product aligned
    a[0] = inE1;
    a[1] = inE2;
    a[2] = Goldilocks::one();
    a[3] = inE3;
    a[4] = inE4;
    a[5] = max;
    a[6] = a1;
    a[7] = a2;
    a[8] = max * max;
    a[9] = p_1;
    a[10] = a1 * a1;
    a[11] = inE4 * p_1;
    Goldilocks::load_avx_a(a0_, &(a[0]));
    Goldilocks::load_avx_a(a1_, &(a[4]));
    Goldilocks::load_avx_a(a2_, &(a[8]));
    Goldilocks::mmult_avx_a(a0_, a1_, a2_, &(Mat[0]));
    Goldilocks::store_avx_a(&(a[0]), a0_);
    Goldilocks::store_avx_a(&(a[4]), a1_);
    Goldilocks::store_avx_a(&(a[8]), a2_);

    ASSERT_EQ(Goldilocks::toU64(b[0]), Goldilocks::toU64(a[0]));
    ASSERT_EQ(Goldilocks::toU64(b[1]), Goldilocks::toU64(a[1]));
    ASSERT_EQ(Goldilocks::toU64(b[2]), Goldilocks::toU64(a[2]));
    ASSERT_EQ(Goldilocks::toU64(b[3]), Goldilocks::toU64(a[3]));
    ASSERT_EQ(Goldilocks::toU64(b[4]), Goldilocks::toU64(a[4]));
    ASSERT_EQ(Goldilocks::toU64(b[5]), Goldilocks::toU64(a[5]));
    ASSERT_EQ(Goldilocks::toU64(b[6]), Goldilocks::toU64(a[6]));
    ASSERT_EQ(Goldilocks::toU64(b[7]), Goldilocks::toU64(a[7]));
    ASSERT_EQ(Goldilocks::toU64(b[8]), Goldilocks::toU64(a[8]));
    ASSERT_EQ(Goldilocks::toU64(b[9]), Goldilocks::toU64(a[9]));
    ASSERT_EQ(Goldilocks::toU64(b[10]), Goldilocks::toU64(a[10]));
    ASSERT_EQ(Goldilocks::toU64(b[11]), Goldilocks::toU64(a[11]));

    // avx product_8
    a[0] = inE1;
    a[1] = inE2;
    a[2] = Goldilocks::one();
    a[3] = inE3;
    a[4] = inE4;
    a[5] = max;
    a[6] = a1;
    a[7] = a2;
    a[8] = max * max;
    a[9] = p_1;
    a[10] = a1 * a1;
    a[11] = inE4 * p_1;
    Goldilocks::load_avx(a0_, &(a[0]));
    Goldilocks::load_avx(a1_, &(a[4]));
    Goldilocks::load_avx(a2_, &(a[8]));
    Goldilocks::mmult_avx_8(a0_, a1_, a2_, &(Mat[0]));
    Goldilocks::store_avx(&(a[0]), a0_);
    Goldilocks::store_avx(&(a[4]), a1_);
    Goldilocks::store_avx(&(a[8]), a2_);

    ASSERT_EQ(Goldilocks::toU64(b[0]), Goldilocks::toU64(a[0]));
    ASSERT_EQ(Goldilocks::toU64(b[1]), Goldilocks::toU64(a[1]));
    ASSERT_EQ(Goldilocks::toU64(b[2]), Goldilocks::toU64(a[2]));
    ASSERT_EQ(Goldilocks::toU64(b[3]), Goldilocks::toU64(a[3]));
    ASSERT_EQ(Goldilocks::toU64(b[4]), Goldilocks::toU64(a[4]));
    ASSERT_EQ(Goldilocks::toU64(b[5]), Goldilocks::toU64(a[5]));
    ASSERT_EQ(Goldilocks::toU64(b[6]), Goldilocks::toU64(a[6]));
    ASSERT_EQ(Goldilocks::toU64(b[7]), Goldilocks::toU64(a[7]));
    ASSERT_EQ(Goldilocks::toU64(b[8]), Goldilocks::toU64(a[8]));
    ASSERT_EQ(Goldilocks::toU64(b[9]), Goldilocks::toU64(a[9]));
    ASSERT_EQ(Goldilocks::toU64(b[10]), Goldilocks::toU64(a[10]));
    ASSERT_EQ(Goldilocks::toU64(b[11]), Goldilocks::toU64(a[11]));

    free(a);
    free(Mat);
    free(b);
}
#ifdef __AVX512__
TEST(GOLDILOCKS_TEST, mmult_avx512)
{
    uint64_t in1 = 3;
    int32_t in2 = 9;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element p_1 = Goldilocks::fromU64(0XFFFFFFFF00000002LL);
    Goldilocks::Element max = Goldilocks::fromU64(0XFFFFFFFFFFFFFFFFULL);
    Goldilocks::Element a1 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF00000000));
    Goldilocks::Element a2 = Goldilocks::fromU64(Goldilocks::from_montgomery(0xFFFFFFFF));

    Goldilocks::Element *a = (Goldilocks::Element *)aligned_alloc(64, 24 * sizeof(Goldilocks::Element));
    Goldilocks::Element *Mat = (Goldilocks::Element *)aligned_alloc(64, 144 * sizeof(Goldilocks::Element));
    Goldilocks::Element *b = (Goldilocks::Element *)aligned_alloc(64, 24 * sizeof(Goldilocks::Element));

    a[0] = inE1;
    a[1] = inE2;
    a[2] = Goldilocks::one();
    a[3] = inE3;
    a[4] = inE1;
    a[5] = inE2;
    a[6] = Goldilocks::one();
    a[7] = inE3;

    a[8] = inE4;
    a[9] = max;
    a[10] = a1;
    a[11] = a2;
    a[12] = inE4;
    a[13] = max;
    a[14] = a1;
    a[15] = a2;

    a[16] = max * max;
    a[17] = p_1;
    a[18] = a1 * a1;
    a[19] = inE4 * p_1;
    a[20] = max * max;
    a[21] = p_1;
    a[22] = a1 * a1;
    a[23] = inE4 * p_1;

    for (int i = 0; i < 12; ++i)
    {
        for (int j = 0; j < 12; ++j)
        {
            Mat[i * 12 + j] = PoseidonGoldilocksConstants::M[i][j];
        }
    }

    // product
    for (int l = 0; l < 3; ++l)
    {
        for (int i = 0; i < 4; ++i)
        {

            Goldilocks::Element sum = Goldilocks::zero();
            for (int k = 0; k < 3; ++k)
            {
                for (int j = 0; j < 4; ++j)
                {
                    sum = sum + (Mat[(l * 4 + i) * 12 + k * 4 + j] * a[k * 8 + j]);
                }
            }
            b[l * 8 + i] = sum;
            b[l * 8 + 4 + i] = sum;
        }
    }

    // avx product
    __m512i a0_;
    __m512i a1_;
    __m512i a2_;

    Goldilocks::load_avx512(a0_, &(a[0]));
    Goldilocks::load_avx512(a1_, &(a[8]));
    Goldilocks::load_avx512(a2_, &(a[16]));
    Goldilocks::mmult_avx512(a0_, a1_, a2_, &(Mat[0]));
    Goldilocks::store_avx512(&(a[0]), a0_);
    Goldilocks::store_avx512(&(a[8]), a1_);
    Goldilocks::store_avx512(&(a[16]), a2_);

    ASSERT_EQ(Goldilocks::toU64(b[0]), Goldilocks::toU64(a[0]));
    ASSERT_EQ(Goldilocks::toU64(b[1]), Goldilocks::toU64(a[1]));
    ASSERT_EQ(Goldilocks::toU64(b[2]), Goldilocks::toU64(a[2]));
    ASSERT_EQ(Goldilocks::toU64(b[3]), Goldilocks::toU64(a[3]));
    ASSERT_EQ(Goldilocks::toU64(b[4]), Goldilocks::toU64(a[4]));
    ASSERT_EQ(Goldilocks::toU64(b[5]), Goldilocks::toU64(a[5]));
    ASSERT_EQ(Goldilocks::toU64(b[6]), Goldilocks::toU64(a[6]));
    ASSERT_EQ(Goldilocks::toU64(b[7]), Goldilocks::toU64(a[7]));
    ASSERT_EQ(Goldilocks::toU64(b[8]), Goldilocks::toU64(a[8]));
    ASSERT_EQ(Goldilocks::toU64(b[9]), Goldilocks::toU64(a[9]));
    ASSERT_EQ(Goldilocks::toU64(b[10]), Goldilocks::toU64(a[10]));
    ASSERT_EQ(Goldilocks::toU64(b[11]), Goldilocks::toU64(a[11]));
    ASSERT_EQ(Goldilocks::toU64(b[12]), Goldilocks::toU64(a[12]));
    ASSERT_EQ(Goldilocks::toU64(b[13]), Goldilocks::toU64(a[13]));
    ASSERT_EQ(Goldilocks::toU64(b[14]), Goldilocks::toU64(a[14]));
    ASSERT_EQ(Goldilocks::toU64(b[15]), Goldilocks::toU64(a[15]));
    ASSERT_EQ(Goldilocks::toU64(b[16]), Goldilocks::toU64(a[16]));
    ASSERT_EQ(Goldilocks::toU64(b[17]), Goldilocks::toU64(a[17]));
    ASSERT_EQ(Goldilocks::toU64(b[18]), Goldilocks::toU64(a[18]));
    ASSERT_EQ(Goldilocks::toU64(b[19]), Goldilocks::toU64(a[19]));
    ASSERT_EQ(Goldilocks::toU64(b[20]), Goldilocks::toU64(a[20]));
    ASSERT_EQ(Goldilocks::toU64(b[21]), Goldilocks::toU64(a[21]));
    ASSERT_EQ(Goldilocks::toU64(b[22]), Goldilocks::toU64(a[22]));
    ASSERT_EQ(Goldilocks::toU64(b[23]), Goldilocks::toU64(a[23]));

    // avx product coefs small
    a[0] = inE1;
    a[1] = inE2;
    a[2] = Goldilocks::one();
    a[3] = inE3;
    a[4] = inE1;
    a[5] = inE2;
    a[6] = Goldilocks::one();
    a[7] = inE3;

    a[8] = inE4;
    a[9] = max;
    a[10] = a1;
    a[11] = a2;
    a[12] = inE4;
    a[13] = max;
    a[14] = a1;
    a[15] = a2;

    a[16] = max * max;
    a[17] = p_1;
    a[18] = a1 * a1;
    a[19] = inE4 * p_1;
    a[20] = max * max;
    a[21] = p_1;
    a[22] = a1 * a1;
    a[23] = inE4 * p_1;

    Goldilocks::load_avx512(a0_, &(a[0]));
    Goldilocks::load_avx512(a1_, &(a[8]));
    Goldilocks::load_avx512(a2_, &(a[16]));
    Goldilocks::mmult_avx512_8(a0_, a1_, a2_, &(Mat[0]));
    Goldilocks::store_avx512(&(a[0]), a0_);
    Goldilocks::store_avx512(&(a[8]), a1_);
    Goldilocks::store_avx512(&(a[16]), a2_);

    ASSERT_EQ(Goldilocks::toU64(b[0]), Goldilocks::toU64(a[0]));
    ASSERT_EQ(Goldilocks::toU64(b[1]), Goldilocks::toU64(a[1]));
    ASSERT_EQ(Goldilocks::toU64(b[2]), Goldilocks::toU64(a[2]));
    ASSERT_EQ(Goldilocks::toU64(b[3]), Goldilocks::toU64(a[3]));
    ASSERT_EQ(Goldilocks::toU64(b[4]), Goldilocks::toU64(a[4]));
    ASSERT_EQ(Goldilocks::toU64(b[5]), Goldilocks::toU64(a[5]));
    ASSERT_EQ(Goldilocks::toU64(b[6]), Goldilocks::toU64(a[6]));
    ASSERT_EQ(Goldilocks::toU64(b[7]), Goldilocks::toU64(a[7]));
    ASSERT_EQ(Goldilocks::toU64(b[8]), Goldilocks::toU64(a[8]));
    ASSERT_EQ(Goldilocks::toU64(b[9]), Goldilocks::toU64(a[9]));
    ASSERT_EQ(Goldilocks::toU64(b[10]), Goldilocks::toU64(a[10]));
    ASSERT_EQ(Goldilocks::toU64(b[11]), Goldilocks::toU64(a[11]));
    ASSERT_EQ(Goldilocks::toU64(b[12]), Goldilocks::toU64(a[12]));
    ASSERT_EQ(Goldilocks::toU64(b[13]), Goldilocks::toU64(a[13]));
    ASSERT_EQ(Goldilocks::toU64(b[14]), Goldilocks::toU64(a[14]));
    ASSERT_EQ(Goldilocks::toU64(b[15]), Goldilocks::toU64(a[15]));
    ASSERT_EQ(Goldilocks::toU64(b[16]), Goldilocks::toU64(a[16]));
    ASSERT_EQ(Goldilocks::toU64(b[17]), Goldilocks::toU64(a[17]));
    ASSERT_EQ(Goldilocks::toU64(b[18]), Goldilocks::toU64(a[18]));
    ASSERT_EQ(Goldilocks::toU64(b[19]), Goldilocks::toU64(a[19]));
    ASSERT_EQ(Goldilocks::toU64(b[20]), Goldilocks::toU64(a[20]));
    ASSERT_EQ(Goldilocks::toU64(b[21]), Goldilocks::toU64(a[21]));
    ASSERT_EQ(Goldilocks::toU64(b[22]), Goldilocks::toU64(a[22]));
    ASSERT_EQ(Goldilocks::toU64(b[23]), Goldilocks::toU64(a[23]));

    free(a);
    free(Mat);
    free(b);
}
#endif

TEST(GOLDILOCKS_TEST, div)
{
    uint64_t in1 = 10;
    int32_t in2 = 5;
    std::string in3 = "92233720347072921606"; // GOLDILOCKS_PRIME * 5 + 1
    int32_t in4 = -12;

    Goldilocks::Element inE1 = Goldilocks::fromU64(in1);
    Goldilocks::Element inE2 = Goldilocks::fromS32(in2);
    Goldilocks::Element inE3 = Goldilocks::fromString(in3);
    Goldilocks::Element inE4 = Goldilocks::fromS32(in4);
    Goldilocks::Element inE5 = Goldilocks::fromS32(in4);
    Goldilocks::Element inE6 = Goldilocks::fromS32(in4);

    ASSERT_EQ(Goldilocks::toU64(inE1 / inE2), in1 / in2);
    ASSERT_EQ(Goldilocks::toU64(inE1 / inE2 / inE3), in1 / in2); // 10 / 2 / ( 0 + 1 ) = 10 / 2
    ASSERT_EQ(Goldilocks::toU64(inE1 / inE2 / inE3 / inE4), 0X2AAAAAAA80000000);
    ASSERT_EQ(Goldilocks::toU64(inE5 / inE6), 1);
    ASSERT_EQ(Goldilocks::toU64(Goldilocks::one() / inE6), 0X1555555540000000);
}
TEST(GOLDILOCKS_TEST, inv)
{
    uint64_t in1 = 5;
    std::string in2 = "18446744069414584326"; // 0xFFFFFFFF00000001n + 5n

    Goldilocks::Element input1 = Goldilocks::one();
    Goldilocks::Element inv1 = Goldilocks::inv(input1);
    Goldilocks::Element res1 = input1 * inv1;

    Goldilocks::Element input5 = Goldilocks::fromU64(in1);
    Goldilocks::Element inv5 = Goldilocks::inv(input5);
    Goldilocks::Element res5 = input5 * inv5;

    ASSERT_EQ(res1, Goldilocks::one());
    ASSERT_EQ(res5, Goldilocks::one());

    Goldilocks::Element inE1 = Goldilocks::fromString(std::to_string(in1));
    Goldilocks::Element inE1_plus_p = Goldilocks::fromString(in2);

    ASSERT_EQ(Goldilocks::inv(inE1_plus_p) * inE1, Goldilocks::one());
    ASSERT_EQ(Goldilocks::inv(inE1), Goldilocks::inv(inE1_plus_p));
}

TEST(GOLDILOCKS_TEST, poseidon_avx_seq)
{

    Goldilocks::Element fibonacci[SPONGE_WIDTH];
    Goldilocks::Element result[CAPACITY];

    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();

    for (uint64_t i = 2; i < SPONGE_WIDTH; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
    }

    PoseidonGoldilocks::hash_seq(result, fibonacci);

    ASSERT_EQ(Goldilocks::toU64(result[0]), 0X3095570037F4605D);
    ASSERT_EQ(Goldilocks::toU64(result[1]), 0X3D561B5EF1BC8B58);
    ASSERT_EQ(Goldilocks::toU64(result[2]), 0X8129DB5EC75C3226);
    ASSERT_EQ(Goldilocks::toU64(result[3]), 0X8EC2B67AFB6B87ED);

    Goldilocks::Element zero[SPONGE_WIDTH] = {Goldilocks::zero()};
    Goldilocks::Element result0[CAPACITY];

    PoseidonGoldilocks::hash_seq(result0, zero);

    ASSERT_EQ(Goldilocks::toU64(result0[0]), 0X3C18A9786CB0B359);
    ASSERT_EQ(Goldilocks::toU64(result0[1]), 0XC4055E3364A246C3);
    ASSERT_EQ(Goldilocks::toU64(result0[2]), 0X7953DB0AB48808F4);
    ASSERT_EQ(Goldilocks::toU64(result0[3]), 0XC71603F33A1144CA);
}
TEST(GOLDILOCKS_TEST, poseidon_avx)
{

    Goldilocks::Element fibonacci[SPONGE_WIDTH];
    Goldilocks::Element result[CAPACITY];

    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();

    for (uint64_t i = 2; i < SPONGE_WIDTH; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
    }

    PoseidonGoldilocks::hash(result, fibonacci);

    ASSERT_EQ(Goldilocks::toU64(result[0]), 0X3095570037F4605D);
    ASSERT_EQ(Goldilocks::toU64(result[1]), 0X3D561B5EF1BC8B58);
    ASSERT_EQ(Goldilocks::toU64(result[2]), 0X8129DB5EC75C3226);
    ASSERT_EQ(Goldilocks::toU64(result[3]), 0X8EC2B67AFB6B87ED);

    Goldilocks::Element zero[SPONGE_WIDTH] = {Goldilocks::zero()};
    Goldilocks::Element result0[CAPACITY];

    PoseidonGoldilocks::hash(result0, zero);

    ASSERT_EQ(Goldilocks::toU64(result0[0]), 0X3C18A9786CB0B359);
    ASSERT_EQ(Goldilocks::toU64(result0[1]), 0XC4055E3364A246C3);
    ASSERT_EQ(Goldilocks::toU64(result0[2]), 0X7953DB0AB48808F4);
    ASSERT_EQ(Goldilocks::toU64(result0[3]), 0XC71603F33A1144CA);
}
#ifdef __AVX512__
TEST(GOLDILOCKS_TEST, poseidon_avx512)
{

    Goldilocks::Element input[2 * SPONGE_WIDTH];
    Goldilocks::Element fibonacci[SPONGE_WIDTH];
    Goldilocks::Element zero[SPONGE_WIDTH];
    Goldilocks::Element result[2 * CAPACITY];
    Goldilocks::Element result0[CAPACITY];
    Goldilocks::Element result1[CAPACITY];

    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();
    zero[0] = Goldilocks::zero();
    zero[1] = Goldilocks::zero();

    for (uint64_t i = 2; i < SPONGE_WIDTH; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
        zero[i] = Goldilocks::zero();
    }

    for (int k = 0; k < 3; ++k)
    {
        for (int i = 0; i < 4; ++i)
        {
            input[k * 8 + i] = fibonacci[k * 4 + i];
            input[k * 8 + i + 4] = Goldilocks::zero();
        }
    }

    PoseidonGoldilocks::hash_avx512(result, input);
    PoseidonGoldilocks::hash(result0, fibonacci);
    PoseidonGoldilocks::hash(result1, zero);

    ASSERT_EQ(Goldilocks::toU64(result[0]), Goldilocks::toU64(result0[0]));
    ASSERT_EQ(Goldilocks::toU64(result[1]), Goldilocks::toU64(result0[1]));
    ASSERT_EQ(Goldilocks::toU64(result[2]), Goldilocks::toU64(result0[2]));
    ASSERT_EQ(Goldilocks::toU64(result[3]), Goldilocks::toU64(result0[3]));
    ASSERT_EQ(Goldilocks::toU64(result[4]), Goldilocks::toU64(result1[0]));
    ASSERT_EQ(Goldilocks::toU64(result[5]), Goldilocks::toU64(result1[1]));
    ASSERT_EQ(Goldilocks::toU64(result[6]), Goldilocks::toU64(result1[2]));
    ASSERT_EQ(Goldilocks::toU64(result[7]), Goldilocks::toU64(result1[3]));
}
#endif

TEST(GOLDILOCKS_TEST, poseidon_full_seq)
{

    Goldilocks::Element fibonacci[SPONGE_WIDTH];
    Goldilocks::Element result[SPONGE_WIDTH];

    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();

    for (uint64_t i = 2; i < SPONGE_WIDTH; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
    }

    PoseidonGoldilocks::hash_full_result_seq(result, fibonacci);

    ASSERT_EQ(Goldilocks::toU64(result[0]), 0X3095570037F4605D);
    ASSERT_EQ(Goldilocks::toU64(result[1]), 0X3D561B5EF1BC8B58);
    ASSERT_EQ(Goldilocks::toU64(result[2]), 0X8129DB5EC75C3226);
    ASSERT_EQ(Goldilocks::toU64(result[3]), 0X8EC2B67AFB6B87ED);
    ASSERT_EQ(Goldilocks::toU64(result[4]), 0XFC591F17D0FAB161);
    ASSERT_EQ(Goldilocks::toU64(result[5]), 0X1D2B045CC2FEA1AD);
    ASSERT_EQ(Goldilocks::toU64(result[6]), 0X8A4E3B0CB12D4527);
    ASSERT_EQ(Goldilocks::toU64(result[7]), 0XFF217A756AE2211);
    ASSERT_EQ(Goldilocks::toU64(result[8]), 0X78F6E79CFC407293);
    ASSERT_EQ(Goldilocks::toU64(result[9]), 0X3DE827E086AE61C9);
    ASSERT_EQ(Goldilocks::toU64(result[10]), 0X921456F6D2D11E27);
    ASSERT_EQ(Goldilocks::toU64(result[11]), 0XF58A41D4028C66A5);

    Goldilocks::Element zero[SPONGE_WIDTH] = {Goldilocks::zero()};
    Goldilocks::Element result0[SPONGE_WIDTH];

    PoseidonGoldilocks::hash_full_result_seq(result0, zero);

    ASSERT_EQ(Goldilocks::toU64(result0[0]), 0X3C18A9786CB0B359);
    ASSERT_EQ(Goldilocks::toU64(result0[1]), 0XC4055E3364A246C3);
    ASSERT_EQ(Goldilocks::toU64(result0[2]), 0X7953DB0AB48808F4);
    ASSERT_EQ(Goldilocks::toU64(result0[3]), 0XC71603F33A1144CA);
    ASSERT_EQ(Goldilocks::toU64(result0[4]), 0XD7709673896996DC);
    ASSERT_EQ(Goldilocks::toU64(result0[5]), 0X46A84E87642F44ED);
    ASSERT_EQ(Goldilocks::toU64(result0[6]), 0XD032648251EE0B3C);
    ASSERT_EQ(Goldilocks::toU64(result0[7]), 0X1C687363B207DF62);
    ASSERT_EQ(Goldilocks::toU64(result0[8]), 0XDF8565563E8045FE);
    ASSERT_EQ(Goldilocks::toU64(result0[9]), 0X40F5B37FF4254DAE);
    ASSERT_EQ(Goldilocks::toU64(result0[10]), 0XD070F637B431067C);
    ASSERT_EQ(Goldilocks::toU64(result0[11]), 0X1792B1C4342109D7);
}
TEST(GOLDILOCKS_TEST, poseidon_full_avx)
{

    Goldilocks::Element fibonacci[SPONGE_WIDTH];
    Goldilocks::Element result[SPONGE_WIDTH];

    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();

    for (uint64_t i = 2; i < SPONGE_WIDTH; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
    }

    PoseidonGoldilocks::hash_full_result(result, fibonacci);

    ASSERT_EQ(Goldilocks::toU64(result[0]), 0X3095570037F4605D);
    ASSERT_EQ(Goldilocks::toU64(result[1]), 0X3D561B5EF1BC8B58);
    ASSERT_EQ(Goldilocks::toU64(result[2]), 0X8129DB5EC75C3226);
    ASSERT_EQ(Goldilocks::toU64(result[3]), 0X8EC2B67AFB6B87ED);
    ASSERT_EQ(Goldilocks::toU64(result[4]), 0XFC591F17D0FAB161);
    ASSERT_EQ(Goldilocks::toU64(result[5]), 0X1D2B045CC2FEA1AD);
    ASSERT_EQ(Goldilocks::toU64(result[6]), 0X8A4E3B0CB12D4527);
    ASSERT_EQ(Goldilocks::toU64(result[7]), 0XFF217A756AE2211);
    ASSERT_EQ(Goldilocks::toU64(result[8]), 0X78F6E79CFC407293);
    ASSERT_EQ(Goldilocks::toU64(result[9]), 0X3DE827E086AE61C9);
    ASSERT_EQ(Goldilocks::toU64(result[10]), 0X921456F6D2D11E27);
    ASSERT_EQ(Goldilocks::toU64(result[11]), 0XF58A41D4028C66A5);

    Goldilocks::Element zero[SPONGE_WIDTH] = {Goldilocks::zero()};
    Goldilocks::Element result0[SPONGE_WIDTH];

    PoseidonGoldilocks::hash_full_result(result0, zero);

    ASSERT_EQ(Goldilocks::toU64(result0[0]), 0X3C18A9786CB0B359);
    ASSERT_EQ(Goldilocks::toU64(result0[1]), 0XC4055E3364A246C3);
    ASSERT_EQ(Goldilocks::toU64(result0[2]), 0X7953DB0AB48808F4);
    ASSERT_EQ(Goldilocks::toU64(result0[3]), 0XC71603F33A1144CA);
    ASSERT_EQ(Goldilocks::toU64(result0[4]), 0XD7709673896996DC);
    ASSERT_EQ(Goldilocks::toU64(result0[5]), 0X46A84E87642F44ED);
    ASSERT_EQ(Goldilocks::toU64(result0[6]), 0XD032648251EE0B3C);
    ASSERT_EQ(Goldilocks::toU64(result0[7]), 0X1C687363B207DF62);
    ASSERT_EQ(Goldilocks::toU64(result0[8]), 0XDF8565563E8045FE);
    ASSERT_EQ(Goldilocks::toU64(result0[9]), 0X40F5B37FF4254DAE);
    ASSERT_EQ(Goldilocks::toU64(result0[10]), 0XD070F637B431067C);
    ASSERT_EQ(Goldilocks::toU64(result0[11]), 0X1792B1C4342109D7);
}
#ifdef __AVX512__
TEST(GOLDILOCKS_TEST, poseidon_full_avx512)
{

    Goldilocks::Element fibonacci[SPONGE_WIDTH];
    Goldilocks::Element fibonacciAndZero[2 * SPONGE_WIDTH];
    Goldilocks::Element result[2 * SPONGE_WIDTH];

    // Evaluate fibonacci
    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();
    for (uint64_t i = 2; i < SPONGE_WIDTH; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
    }

    // Interleave both inputs
    for (int k = 0; k < 3; ++k)
    {
        for (int i = 0; i < 4; ++i)
        {
            fibonacciAndZero[8 * k + i] = fibonacci[k * 4 + i];
            fibonacciAndZero[8 * k + i + 4] = Goldilocks::zero();
        }
    }

    PoseidonGoldilocks::hash_full_result_avx512(result, fibonacciAndZero);

    // Ouptputs are also interleaved
    ASSERT_EQ(Goldilocks::toU64(result[0]), 0X3095570037F4605D);
    ASSERT_EQ(Goldilocks::toU64(result[1]), 0X3D561B5EF1BC8B58);
    ASSERT_EQ(Goldilocks::toU64(result[2]), 0X8129DB5EC75C3226);
    ASSERT_EQ(Goldilocks::toU64(result[3]), 0X8EC2B67AFB6B87ED);
    ASSERT_EQ(Goldilocks::toU64(result[4]), 0X3C18A9786CB0B359);
    ASSERT_EQ(Goldilocks::toU64(result[5]), 0XC4055E3364A246C3);
    ASSERT_EQ(Goldilocks::toU64(result[6]), 0X7953DB0AB48808F4);
    ASSERT_EQ(Goldilocks::toU64(result[7]), 0XC71603F33A1144CA);
    ASSERT_EQ(Goldilocks::toU64(result[8]), 0XFC591F17D0FAB161);
    ASSERT_EQ(Goldilocks::toU64(result[9]), 0X1D2B045CC2FEA1AD);
    ASSERT_EQ(Goldilocks::toU64(result[10]), 0X8A4E3B0CB12D4527);
    ASSERT_EQ(Goldilocks::toU64(result[11]), 0XFF217A756AE2211);
    ASSERT_EQ(Goldilocks::toU64(result[12]), 0XD7709673896996DC);
    ASSERT_EQ(Goldilocks::toU64(result[13]), 0X46A84E87642F44ED);
    ASSERT_EQ(Goldilocks::toU64(result[14]), 0XD032648251EE0B3C);
    ASSERT_EQ(Goldilocks::toU64(result[15]), 0X1C687363B207DF62);
    ASSERT_EQ(Goldilocks::toU64(result[16]), 0X78F6E79CFC407293);
    ASSERT_EQ(Goldilocks::toU64(result[17]), 0X3DE827E086AE61C9);
    ASSERT_EQ(Goldilocks::toU64(result[18]), 0X921456F6D2D11E27);
    ASSERT_EQ(Goldilocks::toU64(result[19]), 0XF58A41D4028C66A5);
    ASSERT_EQ(Goldilocks::toU64(result[20]), 0XDF8565563E8045FE);
    ASSERT_EQ(Goldilocks::toU64(result[21]), 0X40F5B37FF4254DAE);
    ASSERT_EQ(Goldilocks::toU64(result[22]), 0XD070F637B431067C);
    ASSERT_EQ(Goldilocks::toU64(result[23]), 0X1792B1C4342109D7);
}
#endif

TEST(GOLDILOCKS_TEST, linear_hash_seq)
{

    Goldilocks::Element fibonacci[NCOLS_HASH];
    Goldilocks::Element result[CAPACITY];

    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();
    for (uint64_t i = 2; i < NCOLS_HASH; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
    }

    PoseidonGoldilocks::linear_hash_seq(result, fibonacci, NCOLS_HASH);

    ASSERT_EQ(Goldilocks::toU64(result[0]), 0XB214FEA22C79AE3C);
    ASSERT_EQ(Goldilocks::toU64(result[1]), 0X49DA61DEED54466A);
    ASSERT_EQ(Goldilocks::toU64(result[2]), 0X7338CC9DBA8256FD);
    ASSERT_EQ(Goldilocks::toU64(result[3]), 0XC1043293021620CE);
}
TEST(GOLDILOCKS_TEST, linear_hash_avx)
{

    Goldilocks::Element fibonacci[NCOLS_HASH];
    Goldilocks::Element result[CAPACITY];

    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();
    for (uint64_t i = 2; i < NCOLS_HASH; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
    }

    PoseidonGoldilocks::linear_hash(result, fibonacci, NCOLS_HASH);

    ASSERT_EQ(Goldilocks::toU64(result[0]), 0XB214FEA22C79AE3C);
    ASSERT_EQ(Goldilocks::toU64(result[1]), 0X49DA61DEED54466A);
    ASSERT_EQ(Goldilocks::toU64(result[2]), 0X7338CC9DBA8256FD);
    ASSERT_EQ(Goldilocks::toU64(result[3]), 0XC1043293021620CE);
}
#ifdef __AVX512__
TEST(GOLDILOCKS_TEST, linear_hash_avx512)
{

    Goldilocks::Element fibonacci[2 * NCOLS_HASH];
    Goldilocks::Element result[2 * CAPACITY];

    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();
    fibonacci[NCOLS_HASH] = Goldilocks::zero();
    fibonacci[NCOLS_HASH + 1] = Goldilocks::one();
    for (uint64_t i = 2; i < NCOLS_HASH; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
        fibonacci[i + NCOLS_HASH] = fibonacci[i];
    }

    PoseidonGoldilocks::linear_hash_avx512(result, fibonacci, NCOLS_HASH);

    ASSERT_EQ(Goldilocks::toU64(result[0]), 0XB214FEA22C79AE3C);
    ASSERT_EQ(Goldilocks::toU64(result[1]), 0X49DA61DEED54466A);
    ASSERT_EQ(Goldilocks::toU64(result[2]), 0X7338CC9DBA8256FD);
    ASSERT_EQ(Goldilocks::toU64(result[3]), 0XC1043293021620CE);
    ASSERT_EQ(Goldilocks::toU64(result[4]), 0XB214FEA22C79AE3C);
    ASSERT_EQ(Goldilocks::toU64(result[5]), 0X49DA61DEED54466A);
    ASSERT_EQ(Goldilocks::toU64(result[6]), 0X7338CC9DBA8256FD);
    ASSERT_EQ(Goldilocks::toU64(result[7]), 0XC1043293021620CE);
}
#endif

TEST(GOLDILOCKS_TEST, merkletree_seq)
{
    uint64_t ncols_hash = 128;
    uint64_t nrows_hash = (1 << 6);
    Goldilocks::Element *cols = (Goldilocks::Element *)malloc((uint64_t)ncols_hash * (uint64_t)nrows_hash * sizeof(Goldilocks::Element));
#pragma omp parallel for
    for (uint64_t i = 0; i < ncols_hash; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + ncols_hash] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < nrows_hash; j++)
    {
#pragma omp parallel for
        for (uint64_t i = 0; i < ncols_hash; i++)
        {
            cols[j * ncols_hash + i] = cols[(j - 2) * ncols_hash + i] + cols[(j - 1) * ncols_hash + i];
        }
    }

    uint64_t numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    Goldilocks::Element *tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));

    PoseidonGoldilocks::merkletree_seq(tree, cols, ncols_hash, nrows_hash);
    Goldilocks::Element root[4];
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X918F7CD0C3E8701F);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X83A130E00F961B02);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0X6921497B364123F8);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0XBD2B98A57B748BF4);

    free(cols);
    free(tree);

    // Edge case, ncols_hash =0
    ncols_hash = 0;
    nrows_hash = (1 << 6);

    numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));
    cols = NULL;
    PoseidonGoldilocks::merkletree_seq(tree, cols, ncols_hash, nrows_hash);
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X25225F1A5D49614A);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X5A1D2A648EEE8F03);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0xDDA8F741C47DFB10);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0X49561260080D30C3);

    free(tree);
    ncols_hash = 0;
    nrows_hash = (1 << 17);
    numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));
    cols = NULL;
    PoseidonGoldilocks::merkletree_seq(tree, cols, ncols_hash, nrows_hash);
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X5587AD00B6DDF0CB);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X279949E14530C250);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0x2F8E22C79467775);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0XAA45BE01F9E1610);

    free(tree);
}
TEST(GOLDILOCKS_TEST, merkletree_avx)
{
    uint64_t ncols_hash = 128;
    uint64_t nrows_hash = (1 << 6);
    Goldilocks::Element *cols = (Goldilocks::Element *)malloc((uint64_t)ncols_hash * (uint64_t)nrows_hash * sizeof(Goldilocks::Element));
#pragma omp parallel for
    for (uint64_t i = 0; i < ncols_hash; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + ncols_hash] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < nrows_hash; j++)
    {
#pragma omp parallel for
        for (uint64_t i = 0; i < ncols_hash; i++)
        {
            cols[j * ncols_hash + i] = cols[(j - 2) * ncols_hash + i] + cols[(j - 1) * ncols_hash + i];
        }
    }

    uint64_t numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    Goldilocks::Element *tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));

    PoseidonGoldilocks::merkletree_avx(tree, cols, ncols_hash, nrows_hash);
    Goldilocks::Element root[4];
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X918F7CD0C3E8701F);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X83A130E00F961B02);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0X6921497B364123F8);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0XBD2B98A57B748BF4);

    free(cols);
    free(tree);

    // Edge case, nrows_hash =0
    ncols_hash = 0;
    nrows_hash = (1 << 6);

    numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));
    cols = NULL;
    PoseidonGoldilocks::merkletree_avx(tree, cols, ncols_hash, nrows_hash);
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X25225F1A5D49614A);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X5A1D2A648EEE8F03);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0xDDA8F741C47DFB10);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0X49561260080D30C3);

    free(tree);

    // Edge case
    ncols_hash = 0;
    nrows_hash = (1 << 17);

    numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));
    cols = NULL;
    PoseidonGoldilocks::merkletree_avx(tree, cols, ncols_hash, nrows_hash);
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X5587AD00B6DDF0CB);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X279949E14530C250);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0x2F8E22C79467775);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0XAA45BE01F9E1610);

    free(tree);
}
#ifdef __AVX512__
TEST(GOLDILOCKS_TEST, merkletree_avx512)
{
    uint64_t ncols_hash = 128;
    uint64_t nrows_hash = (1 << 6);
    Goldilocks::Element *cols = (Goldilocks::Element *)malloc((uint64_t)ncols_hash * (uint64_t)nrows_hash * sizeof(Goldilocks::Element));
#pragma omp parallel for
    for (uint64_t i = 0; i < ncols_hash; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + ncols_hash] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < nrows_hash; j++)
    {
#pragma omp parallel for
        for (uint64_t i = 0; i < ncols_hash; i++)
        {
            cols[j * ncols_hash + i] = cols[(j - 2) * ncols_hash + i] + cols[(j - 1) * ncols_hash + i];
        }
    }

    uint64_t numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    Goldilocks::Element *tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));

    PoseidonGoldilocks::merkletree_seq(tree, cols, ncols_hash, nrows_hash);
    Goldilocks::Element root[4];
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X918F7CD0C3E8701F);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X83A130E00F961B02);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0X6921497B364123F8);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0XBD2B98A57B748BF4);

    free(cols);
    free(tree);

    // Edge case, ncols_hash =0
    ncols_hash = 0;
    nrows_hash = (1 << 6);

    numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));
    cols = NULL;
    PoseidonGoldilocks::merkletree_seq(tree, cols, ncols_hash, nrows_hash);
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X25225F1A5D49614A);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X5A1D2A648EEE8F03);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0xDDA8F741C47DFB10);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0X49561260080D30C3);

    free(tree);
    ncols_hash = 0;
    nrows_hash = (1 << 17);
    numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));
    cols = NULL;
    PoseidonGoldilocks::merkletree_seq(tree, cols, ncols_hash, nrows_hash);
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X5587AD00B6DDF0CB);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X279949E14530C250);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0x2F8E22C79467775);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0XAA45BE01F9E1610);

    free(tree);
}
#endif

TEST(GOLDILOCKS_TEST, merkletree_batch_seq)
{
    uint64_t ncols_hash = 128;
    uint64_t nrows_hash = (1 << 6);
    Goldilocks::Element *cols = (Goldilocks::Element *)malloc((uint64_t)ncols_hash * (uint64_t)nrows_hash * sizeof(Goldilocks::Element));
#pragma omp parallel for
    for (uint64_t i = 0; i < ncols_hash; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + ncols_hash] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < nrows_hash; j++)
    {
#pragma omp parallel for
        for (uint64_t i = 0; i < ncols_hash; i++)
        {
            cols[j * ncols_hash + i] = cols[(j - 2) * ncols_hash + i] + cols[(j - 1) * ncols_hash + i];
        }
    }

    uint64_t numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    Goldilocks::Element *tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));

    PoseidonGoldilocks::merkletree_batch_seq(tree, cols, ncols_hash, nrows_hash, (ncols_hash + 3) / 4);

    Goldilocks::Element root[4];
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0xb2597514367e69fd);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0x1083bd8754affcb8);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0x6ad216b78faa6470);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0x3e8670a179011526);

    free(cols);
    free(tree);

    // Edge case, ncols_hash =0
    ncols_hash = 0;
    nrows_hash = (1 << 6);

    numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));
    cols = NULL;
    PoseidonGoldilocks::merkletree_batch_seq(tree, cols, ncols_hash, nrows_hash, (ncols_hash + 3) / 4);
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X25225F1A5D49614A);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X5A1D2A648EEE8F03);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0xDDA8F741C47DFB10);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0X49561260080D30C3);

    free(tree);
    ncols_hash = 0;
    nrows_hash = (1 << 17);
    numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));
    cols = NULL;
    PoseidonGoldilocks::merkletree_batch_seq(tree, cols, ncols_hash, nrows_hash, (ncols_hash + 3) / 4);
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X5587AD00B6DDF0CB);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X279949E14530C250);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0x2F8E22C79467775);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0XAA45BE01F9E1610);

    free(tree);
}
TEST(GOLDILOCKS_TEST, merkletree_batch)
{
    uint64_t ncols_hash = 128;
    uint64_t nrows_hash = (1 << 6);
    Goldilocks::Element *cols = (Goldilocks::Element *)malloc((uint64_t)ncols_hash * (uint64_t)nrows_hash * sizeof(Goldilocks::Element));
#pragma omp parallel for
    for (uint64_t i = 0; i < ncols_hash; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + ncols_hash] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < nrows_hash; j++)
    {
#pragma omp parallel for
        for (uint64_t i = 0; i < ncols_hash; i++)
        {
            cols[j * ncols_hash + i] = cols[(j - 2) * ncols_hash + i] + cols[(j - 1) * ncols_hash + i];
        }
    }

    uint64_t numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    Goldilocks::Element *tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));

    PoseidonGoldilocks::merkletree_batch_avx(tree, cols, ncols_hash, nrows_hash, (ncols_hash + 3) / 4);

    Goldilocks::Element root[4];
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0xb2597514367e69fd);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0x1083bd8754affcb8);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0x6ad216b78faa6470);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0x3e8670a179011526);

    free(cols);
    free(tree);

    // Edge case, ncols_hash =0
    ncols_hash = 0;
    nrows_hash = (1 << 6);

    numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));
    cols = NULL;
    PoseidonGoldilocks::merkletree_batch_avx(tree, cols, ncols_hash, nrows_hash, (ncols_hash + 3) / 4);
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X25225F1A5D49614A);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X5A1D2A648EEE8F03);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0xDDA8F741C47DFB10);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0X49561260080D30C3);

    free(tree);
    ncols_hash = 0;
    nrows_hash = (1 << 17);
    numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));
    cols = NULL;
    PoseidonGoldilocks::merkletree_batch_avx(tree, cols, ncols_hash, nrows_hash, (ncols_hash + 3) / 4);
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X5587AD00B6DDF0CB);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X279949E14530C250);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0x2F8E22C79467775);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0XAA45BE01F9E1610);

    free(tree);
}
#ifdef __AVX512__
TEST(GOLDILOCKS_TEST, merkletree_batch_avx512)
{
    uint64_t ncols_hash = 128;
    uint64_t nrows_hash = (1 << 6);
    Goldilocks::Element *cols = (Goldilocks::Element *)malloc((uint64_t)ncols_hash * (uint64_t)nrows_hash * sizeof(Goldilocks::Element));
#pragma omp parallel for
    for (uint64_t i = 0; i < ncols_hash; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + ncols_hash] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < nrows_hash; j++)
    {
#pragma omp parallel for
        for (uint64_t i = 0; i < ncols_hash; i++)
        {
            cols[j * ncols_hash + i] = cols[(j - 2) * ncols_hash + i] + cols[(j - 1) * ncols_hash + i];
        }
    }

    uint64_t numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    Goldilocks::Element *tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));

    PoseidonGoldilocks::merkletree_batch_avx512(tree, cols, ncols_hash, nrows_hash, (ncols_hash + 3) / 4);

    Goldilocks::Element root[4];
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0xb2597514367e69fd);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0x1083bd8754affcb8);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0x6ad216b78faa6470);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0x3e8670a179011526);

    free(cols);
    free(tree);

    // Edge case, ncols_hash =0
    ncols_hash = 0;
    nrows_hash = (1 << 6);

    numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));
    cols = NULL;
    PoseidonGoldilocks::merkletree_batch_avx512(tree, cols, ncols_hash, nrows_hash, (ncols_hash + 3) / 4);
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X25225F1A5D49614A);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X5A1D2A648EEE8F03);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0xDDA8F741C47DFB10);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0X49561260080D30C3);

    free(tree);
    ncols_hash = 0;
    nrows_hash = (1 << 17);
    numElementsTree = MerklehashGoldilocks::getTreeNumElements(nrows_hash);
    tree = (Goldilocks::Element *)malloc(numElementsTree * sizeof(Goldilocks::Element));
    cols = NULL;
    PoseidonGoldilocks::merkletree_batch_avx512(tree, cols, ncols_hash, nrows_hash, (ncols_hash + 3) / 4);
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    ASSERT_EQ(Goldilocks::toU64(root[0]), 0X5587AD00B6DDF0CB);
    ASSERT_EQ(Goldilocks::toU64(root[1]), 0X279949E14530C250);
    ASSERT_EQ(Goldilocks::toU64(root[2]), 0x2F8E22C79467775);
    ASSERT_EQ(Goldilocks::toU64(root[3]), 0XAA45BE01F9E1610);

    free(tree);
}
#endif

TEST(GOLDILOCKS_TEST, ntt)
{
    Goldilocks::Element *a = (Goldilocks::Element *)malloc(FFT_SIZE * sizeof(Goldilocks::Element));
    Goldilocks::Element *initial = (Goldilocks::Element *)malloc(FFT_SIZE * sizeof(Goldilocks::Element));
    NTT_Goldilocks gntt(FFT_SIZE);

    a[0] = Goldilocks::one();
    a[1] = Goldilocks::one();
    for (uint64_t i = 2; i < FFT_SIZE; i++)
    {
        a[i] = a[i - 1] + a[i - 2];
    }

    std::memcpy(initial, a, FFT_SIZE * sizeof(Goldilocks::Element));

    for (int i = 0; i < NUM_REPS; i++)
    {
        gntt.NTT(a, a, FFT_SIZE);
        gntt.INTT(a, a, FFT_SIZE);
    }

    for (int i = 0; i < FFT_SIZE; i++)
    {
        ASSERT_EQ(Goldilocks::toU64(a[i]), Goldilocks::toU64(initial[i]));
    }
    free(a);
    free(initial);
}
TEST(GOLDILOCKS_TEST, ntt_block)
{
    Goldilocks::Element *a = (Goldilocks::Element *)malloc(FFT_SIZE * NUM_COLUMNS * sizeof(Goldilocks::Element));
    Goldilocks::Element *initial = (Goldilocks::Element *)malloc(FFT_SIZE * NUM_COLUMNS * sizeof(Goldilocks::Element));
    NTT_Goldilocks gntt(FFT_SIZE);

    for (uint i = 0; i < 2; i++)
    {
        for (uint j = 0; j < NUM_COLUMNS; j++)
        {
            Goldilocks::add(a[i * NUM_COLUMNS + j], Goldilocks::one(), Goldilocks::fromU64(j));
        }
    }

    for (uint64_t i = 2; i < FFT_SIZE; i++)
    {
        for (uint j = 0; j < NUM_COLUMNS; j++)
        {
            a[i * NUM_COLUMNS + j] = a[NUM_COLUMNS * (i - 1) + j] + a[NUM_COLUMNS * (i - 2) + j];
        }
    }

    std::memcpy(initial, a, FFT_SIZE * NUM_COLUMNS * sizeof(Goldilocks::Element));

    // Option 1: dst is a NULL pointer
    for (int i = 0; i < NUM_REPS; i++)
    {
        gntt.NTT(NULL, a, FFT_SIZE, NUM_COLUMNS);
        gntt.INTT(NULL, a, FFT_SIZE, NUM_COLUMNS);
    }

    for (int i = 0; i < FFT_SIZE * NUM_COLUMNS; i++)
    {
        ASSERT_EQ(Goldilocks::toU64(a[i]), Goldilocks::toU64(initial[i]));
    }

    // Option 2: dst = src
    for (int i = 0; i < NUM_REPS; i++)
    {
        gntt.NTT(a, a, FFT_SIZE, NUM_COLUMNS);
        gntt.INTT(a, a, FFT_SIZE, NUM_COLUMNS);
    }

    for (int i = 0; i < FFT_SIZE * NUM_COLUMNS; i++)
    {
        ASSERT_EQ(Goldilocks::toU64(a[i]), Goldilocks::toU64(initial[i]));
    }

    // Option 3: dst != src
    Goldilocks::Element *dst = (Goldilocks::Element *)malloc(FFT_SIZE * NUM_COLUMNS * sizeof(Goldilocks::Element));
    for (int i = 0; i < NUM_REPS; i++)
    {
        gntt.NTT(dst, a, FFT_SIZE, NUM_COLUMNS);
        for (uint64_t k = 0; k < FFT_SIZE * NUM_COLUMNS; ++k)
            a[k] = Goldilocks::zero();
        gntt.INTT(a, dst, FFT_SIZE, NUM_COLUMNS);
    }

    for (int i = 0; i < FFT_SIZE * NUM_COLUMNS; i++)
    {
        ASSERT_EQ(Goldilocks::toU64(a[i]), Goldilocks::toU64(initial[i]));
    }

    // Option 4: different configurations of phases and blocks
    for (int i = 0; i < NUM_REPS; i++)
    {
        gntt.NTT(a, a, FFT_SIZE, NUM_COLUMNS, NULL, 3, 5);
        gntt.INTT(a, a, FFT_SIZE, NUM_COLUMNS, NULL, 4, 3);
    }

    for (int i = 0; i < FFT_SIZE * NUM_COLUMNS; i++)
    {
        ASSERT_EQ(Goldilocks::toU64(a[i]), Goldilocks::toU64(initial[i]));
    }
    
    // Option 5: same than 4 but with different src and dst buffers
    for (int i = 0; i < NUM_REPS; i++)
    {
        gntt.NTT(dst, a, FFT_SIZE, NUM_COLUMNS, NULL, 3, 5);
        gntt.INTT(a, dst, FFT_SIZE, NUM_COLUMNS, NULL, 4, 3);
    }

    for (int i = 0; i < FFT_SIZE * NUM_COLUMNS; i++)
    {
        ASSERT_EQ(Goldilocks::toU64(a[i]), Goldilocks::toU64(initial[i]));
    }

    // Option 6: out of range parameters
    for (int i = 0; i < NUM_REPS; i++)
    {
        gntt.NTT(a, a, FFT_SIZE, NUM_COLUMNS, NULL, 3, 3000);
        gntt.INTT(a, a, FFT_SIZE, NUM_COLUMNS, NULL, 4, -1);
    }

    for (int i = 0; i < FFT_SIZE * NUM_COLUMNS; i++)
    {
        ASSERT_EQ(Goldilocks::toU64(a[i]), Goldilocks::toU64(initial[i]));
    }
    free(a);
    free(initial);

    // Edge case:Try to call ntt with FFT_SIZE = 1 ncols=3
    uint64_t fft_size = 1;
    uint64_t ncols = 3;
    Goldilocks::Element a1[3] = {1, 2, 3};
    Goldilocks::Element b1[3];

    gntt.NTT(b1, a1, fft_size, ncols);
    ASSERT_EQ(Goldilocks::toU64(b1[0]), 1);
    ASSERT_EQ(Goldilocks::toU64(b1[1]), 2);
    ASSERT_EQ(Goldilocks::toU64(b1[2]), 3);

    gntt.INTT(a1, b1, fft_size, ncols);

    ASSERT_EQ(Goldilocks::toU64(a1[0]), 1);
    ASSERT_EQ(Goldilocks::toU64(a1[1]), 2);
    ASSERT_EQ(Goldilocks::toU64(a1[2]), 3);

    // Edge case:Try to call ntt with FFT_SIZE = 2 ncols=3
    fft_size = 2;
    ncols = 3;
    Goldilocks::Element a2[6] = {1, 2, 3, 4, 5, 6};
    Goldilocks::Element b2[6];

    gntt.NTT(b2, a2, fft_size, ncols);
    gntt.INTT(a2, b2, fft_size, ncols);

    ASSERT_EQ(Goldilocks::toU64(a2[0]), 1);
    ASSERT_EQ(Goldilocks::toU64(a2[1]), 2);
    ASSERT_EQ(Goldilocks::toU64(a2[2]), 3);
    ASSERT_EQ(Goldilocks::toU64(a2[3]), 4);
    ASSERT_EQ(Goldilocks::toU64(a2[4]), 5);
    ASSERT_EQ(Goldilocks::toU64(a2[5]), 6);

    // Edge case: It does not crash with size==0 or ncols==0
    fft_size = 0;
    ncols = 3;
    gntt.NTT(b2, a2, fft_size, ncols);
    gntt.INTT(a2, b2, fft_size, ncols);
    fft_size = 1;
    ncols = 0;
    gntt.NTT(b2, a2, fft_size, ncols);
    gntt.INTT(a2, b2, fft_size, ncols);
}
TEST(GOLDILOCKS_TEST, LDE)
{
    Goldilocks::Element *a = (Goldilocks::Element *)malloc((FFT_SIZE << BLOWUP_FACTOR) * sizeof(Goldilocks::Element));
    NTT_Goldilocks gntt(FFT_SIZE);
    NTT_Goldilocks gntt_extension((FFT_SIZE << BLOWUP_FACTOR));

    Goldilocks::Element *zeros_array = (Goldilocks::Element *)malloc(((FFT_SIZE << BLOWUP_FACTOR) - FFT_SIZE) * sizeof(Goldilocks::Element));
#pragma omp parallel for
    for (uint i = 0; i < ((FFT_SIZE << BLOWUP_FACTOR) - FFT_SIZE); i++)
    {
        zeros_array[i] = Goldilocks::zero();
    }

    a[0] = Goldilocks::one();
    a[1] = Goldilocks::one();
    for (uint64_t i = 2; i < FFT_SIZE; i++)
    {
        a[i] = a[i - 1] + a[i - 2];
    }

    Goldilocks::Element shift = Goldilocks::fromU64(49); // TODO: ask for this number, where to put it how to calculate it
    gntt.INTT(a, a, FFT_SIZE);

    // TODO: This can be pre-generated
    Goldilocks::Element *r = (Goldilocks::Element *)malloc(FFT_SIZE * sizeof(Goldilocks::Element));
    r[0] = Goldilocks::one();
    for (int i = 1; i < FFT_SIZE; i++)
    {
        r[i] = r[i - 1] * shift;
    }

#pragma omp parallel for
    for (int i = 0; i < FFT_SIZE; i++)
    {
        a[i] = a[i] * r[i];
    }

    std::memcpy(&a[FFT_SIZE], zeros_array, ((FFT_SIZE << BLOWUP_FACTOR) - FFT_SIZE) * sizeof(Goldilocks::Element));

    gntt_extension.NTT(a, a, (FFT_SIZE << BLOWUP_FACTOR));

    /*for (int k = 0; k < 32; ++k)
    {
        std::cout << std::showbase << std::hex << std::uppercase << Goldilocks::toU64(a[k]) << std::endl;
    }*/
    ASSERT_EQ(Goldilocks::toU64(a[0]), 0XCBA857825D02DA98);
    ASSERT_EQ(Goldilocks::toU64(a[1]), 0X46B25F2EB8DC45C6);
    ASSERT_EQ(Goldilocks::toU64(a[2]), 0X53CD52572B82CE93);
    ASSERT_EQ(Goldilocks::toU64(a[3]), 0X6A1C4033524890BC);
    ASSERT_EQ(Goldilocks::toU64(a[4]), 0XA9103D6B086AC1F6);
    ASSERT_EQ(Goldilocks::toU64(a[5]), 0XF9EDB8DE1C59C93D);
    ASSERT_EQ(Goldilocks::toU64(a[6]), 0XDAF72007263AED14);
    ASSERT_EQ(Goldilocks::toU64(a[7]), 0X4761FD742111A2C6);
    ASSERT_EQ(Goldilocks::toU64(a[8]), 0X91998C571BDAFBFE);
    ASSERT_EQ(Goldilocks::toU64(a[9]), 0X89B28028BF5894EC);
    ASSERT_EQ(Goldilocks::toU64(a[10]), 0XDD2FD6CB9F5A0A28);
    ASSERT_EQ(Goldilocks::toU64(a[11]), 0X43C4A931E1A7D68B);
    ASSERT_EQ(Goldilocks::toU64(a[12]), 0X88EB7870B0E49F21);
    ASSERT_EQ(Goldilocks::toU64(a[13]), 0X99A28535EABA76E9);
    ASSERT_EQ(Goldilocks::toU64(a[14]), 0XC05CC85A86046420);
    ASSERT_EQ(Goldilocks::toU64(a[15]), 0XE1DED0726EC6AB22);
    ASSERT_EQ(Goldilocks::toU64(a[16]), 0XFF4F0AFB9C48AA53);
    ASSERT_EQ(Goldilocks::toU64(a[17]), 0X2B3524757554A236);
    ASSERT_EQ(Goldilocks::toU64(a[18]), 0XB867D06B39F63E5B);
    ASSERT_EQ(Goldilocks::toU64(a[19]), 0X9D65B701D0DC0203);
    ASSERT_EQ(Goldilocks::toU64(a[20]), 0XDB653DED8EB0E8B1);
    ASSERT_EQ(Goldilocks::toU64(a[21]), 0X6431B1E66D89DEB8);
    ASSERT_EQ(Goldilocks::toU64(a[22]), 0XF1CB543225A25142);
    ASSERT_EQ(Goldilocks::toU64(a[23]), 0X199DD3926164C43A);
    ASSERT_EQ(Goldilocks::toU64(a[24]), 0XA7B8E1EFC3CFBBF5);
    ASSERT_EQ(Goldilocks::toU64(a[25]), 0X186D4972B303DB54);
    ASSERT_EQ(Goldilocks::toU64(a[26]), 0X249276F9AF9641DF);
    ASSERT_EQ(Goldilocks::toU64(a[27]), 0X2B1235BB52390A00);
    ASSERT_EQ(Goldilocks::toU64(a[28]), 0XEE3147DB1601B67B);
    ASSERT_EQ(Goldilocks::toU64(a[29]), 0XB8B579BA5E655721);
    ASSERT_EQ(Goldilocks::toU64(a[30]), 0X650D467042BCD196);
    ASSERT_EQ(Goldilocks::toU64(a[31]), 0X8249D169442CB677);

    free(a);
    free(zeros_array);
    free(r);
}
TEST(GOLDILOCKS_TEST, LDE_block)
{
    Goldilocks::Element *a = (Goldilocks::Element *)malloc((FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));
    NTT_Goldilocks gntt(FFT_SIZE);
    NTT_Goldilocks gntt_extension((FFT_SIZE << BLOWUP_FACTOR));

    for (uint i = 0; i < 2; i++)
    {
        for (uint j = 0; j < NUM_COLUMNS; j++)
        {
            Goldilocks::add(a[i * NUM_COLUMNS + j], Goldilocks::one(), Goldilocks::fromU64(j));
        }
    }

    for (uint64_t i = 2; i < FFT_SIZE; i++)
    {
        for (uint j = 0; j < NUM_COLUMNS; j++)
        {
            a[i * NUM_COLUMNS + j] = a[NUM_COLUMNS * (i - 1) + j] + a[NUM_COLUMNS * (i - 2) + j];
        }
    }

    gntt.INTT(a, a, FFT_SIZE, NUM_COLUMNS, NULL, NPHASES);

    // TODO: This can be pre-generated
    Goldilocks::Element *r = (Goldilocks::Element *)malloc(FFT_SIZE * sizeof(Goldilocks::Element));
    r[0] = Goldilocks::one();
    for (int i = 1; i < FFT_SIZE; i++)
    {
        r[i] = r[i - 1] * Goldilocks::shift();
    }

#pragma omp parallel for
    for (uint64_t i = 0; i < FFT_SIZE; i++)
    {
        for (uint j = 0; j < NUM_COLUMNS; j++)
        {
            a[i * NUM_COLUMNS + j] = a[NUM_COLUMNS * i + j] * r[i];
        }
    }
#pragma omp parallel for schedule(static)
    for (uint i = FFT_SIZE * NUM_COLUMNS; i < (FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS; i++)
    {
        a[i] = Goldilocks::zero();
    }

    gntt_extension.NTT(a, a, (FFT_SIZE << BLOWUP_FACTOR), NUM_COLUMNS, NULL, NUM_PHASES);
    /*for (int k = 0; k < 32; ++k)
    {
        std::cout << std::showbase << std::hex << std::uppercase << Goldilocks::toU64(a[k * NUM_COLUMNS]) << std::endl;
    }*/
    ASSERT_EQ(Goldilocks::toU64(a[0 * NUM_COLUMNS]), 0X3E7CA26D67147C31);
    ASSERT_EQ(Goldilocks::toU64(a[1 * NUM_COLUMNS]), 0X1310720153E0ABE4);
    ASSERT_EQ(Goldilocks::toU64(a[2 * NUM_COLUMNS]), 0X20446D2EA50E8F96);
    ASSERT_EQ(Goldilocks::toU64(a[3 * NUM_COLUMNS]), 0XEAB91008C3444102);
    ASSERT_EQ(Goldilocks::toU64(a[4 * NUM_COLUMNS]), 0X68523AC1294A2);
    ASSERT_EQ(Goldilocks::toU64(a[5 * NUM_COLUMNS]), 0X8A0BB8A3EBA8260A);
    ASSERT_EQ(Goldilocks::toU64(a[6 * NUM_COLUMNS]), 0X515CEC478A438B2);
    ASSERT_EQ(Goldilocks::toU64(a[7 * NUM_COLUMNS]), 0XA087431602851263);
    ASSERT_EQ(Goldilocks::toU64(a[8 * NUM_COLUMNS]), 0XF09629139EA12C82);
    ASSERT_EQ(Goldilocks::toU64(a[9 * NUM_COLUMNS]), 0X175DC5A131392734);
    ASSERT_EQ(Goldilocks::toU64(a[10 * NUM_COLUMNS]), 0X72991CA43B50D824);
    ASSERT_EQ(Goldilocks::toU64(a[11 * NUM_COLUMNS]), 0XDE85A385ABE2A817);
    ASSERT_EQ(Goldilocks::toU64(a[12 * NUM_COLUMNS]), 0X281F1BF7178650C);
    ASSERT_EQ(Goldilocks::toU64(a[13 * NUM_COLUMNS]), 0XA0C663876DFF41A7);
    ASSERT_EQ(Goldilocks::toU64(a[14 * NUM_COLUMNS]), 0XD49C07EA43D3806C);
    ASSERT_EQ(Goldilocks::toU64(a[15 * NUM_COLUMNS]), 0XBCEB714F2E6B299A);
    ASSERT_EQ(Goldilocks::toU64(a[16 * NUM_COLUMNS]), 0XC46EE848F93207D8);
    ASSERT_EQ(Goldilocks::toU64(a[17 * NUM_COLUMNS]), 0XF70EC69883DEE2A);
    ASSERT_EQ(Goldilocks::toU64(a[18 * NUM_COLUMNS]), 0XEE28CDAF6C30F9D9);
    ASSERT_EQ(Goldilocks::toU64(a[19 * NUM_COLUMNS]), 0X6356B93C02C259B3);
    ASSERT_EQ(Goldilocks::toU64(a[20 * NUM_COLUMNS]), 0XD19A89639BC31A16);
    ASSERT_EQ(Goldilocks::toU64(a[21 * NUM_COLUMNS]), 0XB097AE217FC93344);
    ASSERT_EQ(Goldilocks::toU64(a[22 * NUM_COLUMNS]), 0X29BB681AF743F8F6);
    ASSERT_EQ(Goldilocks::toU64(a[23 * NUM_COLUMNS]), 0X8E874011A158B00B);
    ASSERT_EQ(Goldilocks::toU64(a[24 * NUM_COLUMNS]), 0XC95F0B718235B6D7);
    ASSERT_EQ(Goldilocks::toU64(a[25 * NUM_COLUMNS]), 0XFE51B4A575AFECA0);
    ASSERT_EQ(Goldilocks::toU64(a[26 * NUM_COLUMNS]), 0XC68CF305A6F17F4F);
    ASSERT_EQ(Goldilocks::toU64(a[27 * NUM_COLUMNS]), 0XC7912AE75E2DD36D);
    ASSERT_EQ(Goldilocks::toU64(a[28 * NUM_COLUMNS]), 0X6EFC40795CF38959);
    ASSERT_EQ(Goldilocks::toU64(a[29 * NUM_COLUMNS]), 0X6BD4745D238824D9);
    ASSERT_EQ(Goldilocks::toU64(a[30 * NUM_COLUMNS]), 0XB4FF76AAC16372AA);
    ASSERT_EQ(Goldilocks::toU64(a[31 * NUM_COLUMNS]), 0XA0705C72DD9F9A2F);

    free(a);
    free(r);
}
TEST(GOLDILOCKS_TEST, extendePol)
{

    Goldilocks::Element *a = (Goldilocks::Element *)malloc((FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));
    Goldilocks::Element *b = (Goldilocks::Element *)malloc((FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));

    NTT_Goldilocks ntt(FFT_SIZE);

    for (uint i = 0; i < 2; i++)
    {
        for (uint j = 0; j < NUM_COLUMNS; j++)
        {
            Goldilocks::add(a[i * NUM_COLUMNS + j], Goldilocks::one(), Goldilocks::fromU64(j));
        }
    }

    for (uint64_t i = 2; i < FFT_SIZE; i++)
    {
        for (uint j = 0; j < NUM_COLUMNS; j++)
        {
            a[i * NUM_COLUMNS + j] = a[NUM_COLUMNS * (i - 1) + j] + a[NUM_COLUMNS * (i - 2) + j];
        }
    }

    ntt.extendPol(a, a, FFT_SIZE << BLOWUP_FACTOR, FFT_SIZE, NUM_COLUMNS, b);

    ASSERT_EQ(Goldilocks::toU64(a[0 * NUM_COLUMNS]), 0X3E7CA26D67147C31);
    ASSERT_EQ(Goldilocks::toU64(a[1 * NUM_COLUMNS]), 0X1310720153E0ABE4);
    ASSERT_EQ(Goldilocks::toU64(a[2 * NUM_COLUMNS]), 0X20446D2EA50E8F96);
    ASSERT_EQ(Goldilocks::toU64(a[3 * NUM_COLUMNS]), 0XEAB91008C3444102);
    ASSERT_EQ(Goldilocks::toU64(a[4 * NUM_COLUMNS]), 0X68523AC1294A2);
    ASSERT_EQ(Goldilocks::toU64(a[5 * NUM_COLUMNS]), 0X8A0BB8A3EBA8260A);
    ASSERT_EQ(Goldilocks::toU64(a[6 * NUM_COLUMNS]), 0X515CEC478A438B2);
    ASSERT_EQ(Goldilocks::toU64(a[7 * NUM_COLUMNS]), 0XA087431602851263);
    ASSERT_EQ(Goldilocks::toU64(a[8 * NUM_COLUMNS]), 0XF09629139EA12C82);
    ASSERT_EQ(Goldilocks::toU64(a[9 * NUM_COLUMNS]), 0X175DC5A131392734);
    ASSERT_EQ(Goldilocks::toU64(a[10 * NUM_COLUMNS]), 0X72991CA43B50D824);
    ASSERT_EQ(Goldilocks::toU64(a[11 * NUM_COLUMNS]), 0XDE85A385ABE2A817);
    ASSERT_EQ(Goldilocks::toU64(a[12 * NUM_COLUMNS]), 0X281F1BF7178650C);
    ASSERT_EQ(Goldilocks::toU64(a[13 * NUM_COLUMNS]), 0XA0C663876DFF41A7);
    ASSERT_EQ(Goldilocks::toU64(a[14 * NUM_COLUMNS]), 0XD49C07EA43D3806C);
    ASSERT_EQ(Goldilocks::toU64(a[15 * NUM_COLUMNS]), 0XBCEB714F2E6B299A);
    ASSERT_EQ(Goldilocks::toU64(a[16 * NUM_COLUMNS]), 0XC46EE848F93207D8);
    ASSERT_EQ(Goldilocks::toU64(a[17 * NUM_COLUMNS]), 0XF70EC69883DEE2A);
    ASSERT_EQ(Goldilocks::toU64(a[18 * NUM_COLUMNS]), 0XEE28CDAF6C30F9D9);
    ASSERT_EQ(Goldilocks::toU64(a[19 * NUM_COLUMNS]), 0X6356B93C02C259B3);
    ASSERT_EQ(Goldilocks::toU64(a[20 * NUM_COLUMNS]), 0XD19A89639BC31A16);
    ASSERT_EQ(Goldilocks::toU64(a[21 * NUM_COLUMNS]), 0XB097AE217FC93344);
    ASSERT_EQ(Goldilocks::toU64(a[22 * NUM_COLUMNS]), 0X29BB681AF743F8F6);
    ASSERT_EQ(Goldilocks::toU64(a[23 * NUM_COLUMNS]), 0X8E874011A158B00B);
    ASSERT_EQ(Goldilocks::toU64(a[24 * NUM_COLUMNS]), 0XC95F0B718235B6D7);
    ASSERT_EQ(Goldilocks::toU64(a[25 * NUM_COLUMNS]), 0XFE51B4A575AFECA0);
    ASSERT_EQ(Goldilocks::toU64(a[26 * NUM_COLUMNS]), 0XC68CF305A6F17F4F);
    ASSERT_EQ(Goldilocks::toU64(a[27 * NUM_COLUMNS]), 0XC7912AE75E2DD36D);
    ASSERT_EQ(Goldilocks::toU64(a[28 * NUM_COLUMNS]), 0X6EFC40795CF38959);
    ASSERT_EQ(Goldilocks::toU64(a[29 * NUM_COLUMNS]), 0X6BD4745D238824D9);
    ASSERT_EQ(Goldilocks::toU64(a[30 * NUM_COLUMNS]), 0XB4FF76AAC16372AA);
    ASSERT_EQ(Goldilocks::toU64(a[31 * NUM_COLUMNS]), 0XA0705C72DD9F9A2F);

    free(a);
    free(b);
}

TEST(GOLDILOCKS_CUBIC_TEST, one)
{
    uint64_t a[3] = {1, 1, 1};
    int32_t b[3] = {1, 1, 1};
    std::string c[3] = {"92233720347072921606", "92233720347072921606", "92233720347072921606"}; // GOLDILOCKS_PRIME * 5 + 1
    uint64_t d[3] = {1 + GOLDILOCKS_PRIME, 1 + GOLDILOCKS_PRIME, 1 + GOLDILOCKS_PRIME};

    Goldilocks3::Element ina1;
    Goldilocks3::Element ina2;
    Goldilocks3::Element ina3;
    Goldilocks3::Element inb1;
    Goldilocks3::Element inc1;

    Goldilocks3::fromU64(ina1, a);
    Goldilocks3::fromS32(ina2, b);
    Goldilocks3::fromString(ina3, c);
    Goldilocks3::fromU64(inb1, d);
    Goldilocks3::fromString(inc1, c);

    uint64_t ina1_res[3];
    uint64_t ina2_res[3];
    uint64_t ina3_res[3];
    uint64_t inb1_res[3];
    uint64_t inc1_res[3];

    Goldilocks3::toU64(ina1_res, ina1);
    Goldilocks3::toU64(ina2_res, ina2);
    Goldilocks3::toU64(ina3_res, ina3);
    Goldilocks3::toU64(inb1_res, inb1);
    Goldilocks3::toU64(inc1_res, inc1);

    ASSERT_EQ(ina1_res[0], a[0]);
    ASSERT_EQ(ina1_res[1], a[1]);
    ASSERT_EQ(ina1_res[2], a[2]);

    ASSERT_EQ(ina2_res[0], a[0]);
    ASSERT_EQ(ina2_res[1], a[1]);
    ASSERT_EQ(ina2_res[2], a[2]);

    ASSERT_EQ(ina3_res[0], a[0]);
    ASSERT_EQ(ina3_res[1], a[1]);
    ASSERT_EQ(ina3_res[2], a[2]);

    ASSERT_EQ(inb1_res[0], a[0]);
    ASSERT_EQ(inb1_res[1], a[1]);
    ASSERT_EQ(inb1_res[2], a[2]);

    ASSERT_EQ(inc1_res[0], a[0]);
    ASSERT_EQ(inc1_res[1], a[1]);
    ASSERT_EQ(inc1_res[2], a[2]);
}

int main(int argc, char **argv)
{
    ::testing::InitGoogleTest(&argc, argv);
    return RUN_ALL_TESTS();
}

// Build commands AVX:

// g++:
//  g++ tests/tests.cpp src/* -lgtest -lgmp -lomp -o test -g  -Wall -pthread -fopenmp -mavx2 -L$(find /usr/lib/llvm-* -name "libomp.so" | sed 's/libomp.so//')
//  Intel:
//  icpx tests/tests.cpp src/*.cpp -o test -lgtest -lgmp  -pthread -fopenmp -mavx2

// Build commands AVX512:

// g++:
//  g++ tests/tests.cpp src/* -lgtest -lgmp -lomp -o test -g  -Wall -pthread -fopenmp -mavx2  -mavx512f -L$(find /usr/lib/llvm-* -name "libomp.so" | sed 's/libomp.so//') -D__AVX512__
//  Intel:
//  icpx tests/tests.cpp src/*.cpp -o test -lgtest -lgmp  -pthread -fopenmp -mavx2 -mavx512f -D__AVX512__