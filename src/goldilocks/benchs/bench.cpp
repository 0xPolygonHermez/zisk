#include <benchmark/benchmark.h>
#include <iostream>

#include "../src/goldilocks_base_field.hpp"
#include "../src/poseidon_goldilocks.hpp"
#include "../src/poseidon_goldilocks_avx.hpp"
#include "../src/ntt_goldilocks.hpp"
#include "../src/merklehash_goldilocks.hpp"
#include <immintrin.h>

#include <math.h> /* ceil */
#include "omp.h"

#define FFT_SIZE (1 << 23)

#define NUM_HASHES 2097152
#define NCOLS_HASH 128
#define NROWS_HASH FFT_SIZE

#define NUM_COLUMNS 100
#define BLOWUP_FACTOR 1
#define NPHASES_NTT 2
#define NPHASES_LDE 2
#define NBLOCKS 1

static void ADD_OP_BENCH(benchmark::State &state)
{
    Goldilocks::Element term0, term1, term2;
    // Benchmark
    for (auto _ : state)
    {
        term0 = Goldilocks::one() + Goldilocks::one();
        term1 = Goldilocks::one() + Goldilocks::one() + Goldilocks::one();
        ;
        for (uint64_t i = 0; i < 1000000; i++)
        {
            Goldilocks::add(term2, term0, term1);
            term0 = term1;
            term1 = term2;
        }
    }
    assert(Goldilocks::toU64(term2) == 9315693631868018522ULL);
}
static void SUB_OP_BENCH(benchmark::State &state)
{
    Goldilocks::Element term0, term1, term2;
    // Benchmark
    for (auto _ : state)
    {
        term0 = Goldilocks::one() + Goldilocks::one();
        term1 = Goldilocks::one() + Goldilocks::one() + Goldilocks::one();
        ;
        for (uint64_t i = 0; i < 1000000; i++)
        {
            Goldilocks::sub(term2, term0, term1);
            term0 = term1;
            term1 = term2;
        }
    }
    assert(Goldilocks::toU64(term2) == 17916187359919173389ULL);
}
static void MUL_OP_BENCH(benchmark::State &state)
{
    Goldilocks::Element term0, term1, term2;
    // Benchmark
    for (auto _ : state)
    {
        term0 = Goldilocks::one() + Goldilocks::one();
        term1 = Goldilocks::one() + Goldilocks::one() + Goldilocks::one();
        ;
        for (uint64_t i = 0; i < 1000000; i++)
        {
            Goldilocks::mul(term2, term0, term1);
            term0 = term1;
            term1 = term2;
        }
    }
    assert(Goldilocks::toU64(term2) == 1922281271747280077ULL);
}
static void INV_OP_BENCH(benchmark::State &state)
{
    Goldilocks::Element term0, term1;
    // Benchmark
    for (auto _ : state)
    {
        term0 = Goldilocks::one() + Goldilocks::one();
        term1 = Goldilocks::one() + Goldilocks::one() + Goldilocks::one();

        for (uint64_t i = 0; i < 1000000; i++)
        {
            Goldilocks::inv(term0, term1);
            term1 = term0;
        }
    }
    assert(Goldilocks::toU64(term0) == 3ULL);
}

static void ADD_OP_AVX_BENCH(benchmark::State &state)
{
    Goldilocks::Element term0, term1;
    __m256i term0_, term1_, term2_ = _mm256_setzero_si256();

    // Benchmark
    for (auto _ : state)
    {
        term0 = Goldilocks::one() + Goldilocks::one();
        term1 = Goldilocks::one() + Goldilocks::one() + Goldilocks::one();
        Goldilocks::set_avx(term0_, term0, term0, term0, term0);
        Goldilocks::set_avx(term1_, term1, term1, term1, term1);
        for (uint64_t i = 0; i < 1000000; i++)
        {
            Goldilocks::add_avx(term2_, term0_, term1_);
            term0_ = term1_;
            term1_ = term2_;
        }
    }
    Goldilocks::Element res[4];
    Goldilocks::store_avx(res, term2_);
    assert(Goldilocks::toU64(res[0]) == 9315693631868018522ULL);
}
static void SUB_OP_AVX_BENCH(benchmark::State &state)
{
    Goldilocks::Element term0, term1;
    __m256i term0_, term1_, term2_ = _mm256_setzero_si256();

    // Benchmark
    for (auto _ : state)
    {
        term0 = Goldilocks::one() + Goldilocks::one();
        term1 = Goldilocks::one() + Goldilocks::one() + Goldilocks::one();
        Goldilocks::set_avx(term0_, term0, term0, term0, term0);
        Goldilocks::set_avx(term1_, term1, term1, term1, term1);
        for (uint64_t i = 0; i < 1000000; i++)
        {
            Goldilocks::sub_avx(term2_, term0_, term1_);
            term0_ = term1_;
            term1_ = term2_;
        }
    }
    Goldilocks::Element res[4];
    Goldilocks::store_avx(res, term2_);
    assert(Goldilocks::toU64(res[0]) == 17916187359919173389ULL);

}
static void MUL_OP_AVX_BENCH(benchmark::State &state)
{
    Goldilocks::Element term0, term1;
    __m256i term0_, term1_, term2_ = _mm256_setzero_si256();

    // Benchmark
    for (auto _ : state)
    {
        term0 = Goldilocks::one() + Goldilocks::one();
        term1 = Goldilocks::one() + Goldilocks::one() + Goldilocks::one();
        Goldilocks::set_avx(term0_, term0, term0, term0, term0);
        Goldilocks::set_avx(term1_, term1, term1, term1, term1);
        for (uint64_t i = 0; i < 1000000; i++)
        {
            Goldilocks::mult_avx(term2_, term0_, term1_);
            term0_ = term1_;
            term1_ = term2_;
        }
    }
    Goldilocks::Element res[4];
    Goldilocks::store_avx(res, term2_);
    assert(Goldilocks::toU64(res[0]) == 1922281271747280077ULL);

}

static void POSEIDON_BENCH_FULL(benchmark::State &state)
{
    uint64_t input_size = (uint64_t)NUM_HASHES * (uint64_t)SPONGE_WIDTH;
    Goldilocks::Element *fibonacci = new Goldilocks::Element[input_size];
    Goldilocks::Element *result = new Goldilocks::Element[input_size];

    // Test vector: Fibonacci series
    // 0 1 1 2 3 5 8 13 ... NUM_HASHES * SPONGE_WIDTH ...
    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();
    for (uint64_t i = 2; i < input_size; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
    }

    // Benchmark
    for (auto _ : state)
    {
#pragma omp parallel for num_threads(state.range(0)) schedule(static)
        for (uint64_t i = 0; i < NUM_HASHES; i++)
        {
            PoseidonGoldilocks::hash_full_result_seq((Goldilocks::Element(&)[SPONGE_WIDTH])result[i * SPONGE_WIDTH], (Goldilocks::Element(&)[SPONGE_WIDTH])fibonacci[i * SPONGE_WIDTH]);
        }
    }
    // Check poseidon results poseidon ( 0 1 1 2 3 5 8 13 21 34 55 89 )
    assert(Goldilocks::toU64(result[0]) == 0X3095570037F4605D);
    assert(Goldilocks::toU64(result[1]) == 0X3D561B5EF1BC8B58);
    assert(Goldilocks::toU64(result[2]) == 0X8129DB5EC75C3226);
    assert(Goldilocks::toU64(result[3]) == 0X8EC2B67AFB6B87ED);
    delete[] fibonacci;
    delete[] result;
    // Rate = time to process 1 posseidon per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (double)NUM_HASHES / (double)state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter(input_size * sizeof(uint64_t), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);
}
static void POSEIDON_BENCH_FULL_AVX(benchmark::State &state)
{
    uint64_t input_size = (uint64_t)NUM_HASHES * (uint64_t)SPONGE_WIDTH;
    Goldilocks::Element *fibonacci = new Goldilocks::Element[input_size];
    Goldilocks::Element *result = new Goldilocks::Element[input_size];

    // Test vector: Fibonacci series
    // 0 1 1 2 3 5 8 13 ... NUM_HASHES * SPONGE_WIDTH ...
    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();
    for (uint64_t i = 2; i < input_size; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
    }

    // Benchmark
    for (auto _ : state)
    {
#pragma omp parallel for num_threads(state.range(0)) schedule(static)
        for (uint64_t i = 0; i < NUM_HASHES; i++)
        {
            PoseidonGoldilocks::hash_full_result((Goldilocks::Element(&)[SPONGE_WIDTH])result[i * SPONGE_WIDTH], (Goldilocks::Element(&)[SPONGE_WIDTH])fibonacci[i * SPONGE_WIDTH]);
        }
    }
    // Check poseidon results poseidon ( 0 1 1 2 3 5 8 13 21 34 55 89 )
    assert(Goldilocks::toU64(result[0]) == 0X3095570037F4605D);
    assert(Goldilocks::toU64(result[1]) == 0X3D561B5EF1BC8B58);
    assert(Goldilocks::toU64(result[2]) == 0X8129DB5EC75C3226);
    assert(Goldilocks::toU64(result[3]) == 0X8EC2B67AFB6B87ED);
    delete[] fibonacci;
    delete[] result;
    // Rate = time to process 1 posseidon per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (double)NUM_HASHES / (double)state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter(input_size * sizeof(uint64_t), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);
}
#ifdef __AVX512__
static void POSEIDON_BENCH_FULL_AVX512(benchmark::State &state)
{
    uint64_t input_size = (uint64_t)NUM_HASHES * (uint64_t)SPONGE_WIDTH;
    Goldilocks::Element *fibonacci = new Goldilocks::Element[input_size];
    Goldilocks::Element *input = new Goldilocks::Element[2 * input_size];
    Goldilocks::Element *result = new Goldilocks::Element[2 * input_size];

    // Test vector: Fibonacci series
    // 0 1 1 2 3 5 8 13 ... NUM_HASHES * SPONGE_WIDTH ...
    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();
    for (uint64_t i = 2; i < input_size; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
    }
    for (uint64_t k = 0; k < input_size / 4; k++)
    {
        for (uint64_t i = 0; i < 4; i++)
        {
            input[k * 8 + i] = fibonacci[k * 4 + i];
            input[k * 8 + i + 4] = fibonacci[k * 4 + i];
        }
    }
    // Benchmark
    for (auto _ : state)
    {
#pragma omp parallel for num_threads(state.range(0)) schedule(static)
        for (uint64_t i = 0; i < NUM_HASHES; i += 2)
        {
            PoseidonGoldilocks::hash_full_result_avx512((Goldilocks::Element(&)[2 * SPONGE_WIDTH]) result[2 * i * SPONGE_WIDTH], (Goldilocks::Element(&)[2 * SPONGE_WIDTH]) input[2 * i * SPONGE_WIDTH]);
        }
    }
    // Check poseidon results poseidon ( 0 1 1 2 3 5 8 13 21 34 55 89 )
    assert(Goldilocks::toU64(result[0]) == 0X3095570037F4605D);
    assert(Goldilocks::toU64(result[1]) == 0X3D561B5EF1BC8B58);
    assert(Goldilocks::toU64(result[2]) == 0X8129DB5EC75C3226);
    assert(Goldilocks::toU64(result[3]) == 0X8EC2B67AFB6B87ED);
    delete[] fibonacci;
    delete[] result;
    delete[] input;
    // Rate = time to process 1 posseidon per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (double)NUM_HASHES / (double)state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter(input_size * sizeof(uint64_t), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);
}
#endif

static void POSEIDON_BENCH(benchmark::State &state)
{
    uint64_t input_size = (uint64_t)NUM_HASHES * (uint64_t)SPONGE_WIDTH;
    uint64_t output_size = (uint64_t)NUM_HASHES * (uint64_t)CAPACITY;
    Goldilocks::Element *fibonacci = new Goldilocks::Element[input_size];
    Goldilocks::Element *result = new Goldilocks::Element[output_size];

    // Test vector: Fibonacci series
    // 0 1 1 2 3 5 8 13 ... NUM_HASHES * SPONGE_WIDTH ...
    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();
    for (uint64_t i = 2; i < input_size; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
    }

    // Benchmark
    for (auto _ : state)
    {
#pragma omp parallel for num_threads(state.range(0)) schedule(static)
        for (uint64_t i = 0; i < NUM_HASHES; i++)
        {
            PoseidonGoldilocks::hash_seq((Goldilocks::Element(&)[CAPACITY])result[i * CAPACITY], (Goldilocks::Element(&)[SPONGE_WIDTH])fibonacci[i * SPONGE_WIDTH]);
        }
    }
    // Check poseidon results poseidon ( 0 1 1 2 3 5 8 13 21 34 55 89 )
    assert(Goldilocks::toU64(result[0]) == 0X3095570037F4605D);
    assert(Goldilocks::toU64(result[1]) == 0X3D561B5EF1BC8B58);
    assert(Goldilocks::toU64(result[2]) == 0X8129DB5EC75C3226);
    assert(Goldilocks::toU64(result[3]) == 0X8EC2B67AFB6B87ED);

    delete[] fibonacci;
    delete[] result;
    // Rate = time to process 1 posseidon per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (double)NUM_HASHES / (double)state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter(input_size * sizeof(uint64_t), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);
}
static void POSEIDON_BENCH_AVX(benchmark::State &state)
{
    uint64_t input_size = (uint64_t)NUM_HASHES * (uint64_t)SPONGE_WIDTH;
    uint64_t output_size = (uint64_t)NUM_HASHES * (uint64_t)CAPACITY;
    Goldilocks::Element *fibonacci = new Goldilocks::Element[input_size];
    Goldilocks::Element *result = new Goldilocks::Element[output_size];

    // Test vector: Fibonacci series
    // 0 1 1 2 3 5 8 13 ... NUM_HASHES * SPONGE_WIDTH ...
    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();
    for (uint64_t i = 2; i < input_size; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
    }

    // Benchmark
    for (auto _ : state)
    {
#pragma omp parallel for num_threads(state.range(0)) schedule(static)
        for (uint64_t i = 0; i < NUM_HASHES; i++)
        {
            PoseidonGoldilocks::hash((Goldilocks::Element(&)[CAPACITY])result[i * CAPACITY], (Goldilocks::Element(&)[SPONGE_WIDTH])fibonacci[i * SPONGE_WIDTH]);
        }
    }
    // Check poseidon results poseidon ( 0 1 1 2 3 5 8 13 21 34 55 89 )
    assert(Goldilocks::toU64(result[0]) == 0X3095570037F4605D);
    assert(Goldilocks::toU64(result[1]) == 0X3D561B5EF1BC8B58);
    assert(Goldilocks::toU64(result[2]) == 0X8129DB5EC75C3226);
    assert(Goldilocks::toU64(result[3]) == 0X8EC2B67AFB6B87ED);

    delete[] fibonacci;
    delete[] result;
    // Rate = time to process 1 posseidon per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (double)NUM_HASHES / (double)state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter(input_size * sizeof(uint64_t), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);
}
#ifdef __AVX512__
static void POSEIDON_BENCH_AVX512(benchmark::State &state)
{
    uint64_t input_size = (uint64_t)NUM_HASHES * (uint64_t)SPONGE_WIDTH;
    uint64_t output_size = (uint64_t)NUM_HASHES * (uint64_t)CAPACITY;
    Goldilocks::Element *fibonacci = new Goldilocks::Element[input_size];
    Goldilocks::Element *input = new Goldilocks::Element[2 * input_size];
    Goldilocks::Element *result = new Goldilocks::Element[2 * output_size];

    // Test vector: Fibonacci series
    // 0 1 1 2 3 5 8 13 ... NUM_HASHES * SPONGE_WIDTH ...
    fibonacci[0] = Goldilocks::zero();
    fibonacci[1] = Goldilocks::one();
    for (uint64_t i = 2; i < input_size; i++)
    {
        fibonacci[i] = fibonacci[i - 1] + fibonacci[i - 2];
    }
    for (uint64_t k = 0; k < input_size / 4; k++)
    {
        for (uint64_t i = 0; i < 4; i++)
        {
            input[k * 8 + i] = fibonacci[k * 4 + i];
            input[k * 8 + i + 4] = fibonacci[k * 4 + i];
        }
    }
    // Benchmark
    for (auto _ : state)
    {
#pragma omp parallel for num_threads(state.range(0)) schedule(static)
        for (uint64_t i = 0; i < NUM_HASHES; i += 2)
        {
            PoseidonGoldilocks::hash_full_result_avx512((Goldilocks::Element(&)[2 * CAPACITY]) result[2 * i * CAPACITY], (Goldilocks::Element(&)[2 * SPONGE_WIDTH]) input[2 * i * SPONGE_WIDTH]);
        }
    }
    // Check poseidon results poseidon ( 0 1 1 2 3 5 8 13 21 34 55 89 )
    assert(Goldilocks::toU64(result[0]) == 0X3095570037F4605D);
    assert(Goldilocks::toU64(result[1]) == 0X3D561B5EF1BC8B58);
    assert(Goldilocks::toU64(result[2]) == 0X8129DB5EC75C3226);
    assert(Goldilocks::toU64(result[3]) == 0X8EC2B67AFB6B87ED);
    delete[] fibonacci;
    delete[] result;
    delete[] input;
    // Rate = time to process 1 posseidon per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (double)NUM_HASHES / (double)state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter(input_size * sizeof(uint64_t), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);
}
#endif

static void LINEAR_HASH_BENCH(benchmark::State &state)
{
    Goldilocks::Element *cols = new Goldilocks::Element[(uint64_t)NCOLS_HASH * (uint64_t)NROWS_HASH];
    Goldilocks::Element *result = new Goldilocks::Element[(uint64_t)HASH_SIZE * (uint64_t)NROWS_HASH];

    // Test vector: Fibonacci series on the columns and increase the initial values to the right,
    // 1 2 3 4  5  6  ... NUM_COLS
    // 1 2 3 4  5  6  ... NUM_COLS
    // 2 4 6 8  10 12 ... NUM_COLS + NUM_COLS
    // 3 6 9 12 15 18 ... NUM_COLS + NUM_COLS + NUM_COLS
    for (uint64_t i = 0; i < NCOLS_HASH; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + NCOLS_HASH] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < NROWS_HASH; j++)
    {
        for (uint64_t i = 0; i < NCOLS_HASH; i++)
        {
            cols[j * NCOLS_HASH + i] = cols[(j - 2) * NCOLS_HASH + i] + cols[(j - 1) * NCOLS_HASH + i];
        }
    }

    // Benchmark
    for (auto _ : state)
    {
#pragma omp parallel for num_threads(state.range(0)) schedule(static)
        for (uint64_t i = 0; i < NROWS_HASH; i++)
        {
            PoseidonGoldilocks::linear_hash_seq(&result[i * HASH_SIZE], &cols[i * NCOLS_HASH], NCOLS_HASH);
        }
    }

    // Rate = time to process 1 linear hash per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (double)NROWS_HASH * (double)ceil((double)NCOLS_HASH / (double)RATE) / state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter((uint64_t)NROWS_HASH * (uint64_t)NCOLS_HASH * sizeof(Goldilocks::Element), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);

    delete[] cols;
    delete[] result;
}
static void LINEAR_HASH_BENCH_AVX(benchmark::State &state)
{
    Goldilocks::Element *cols = new Goldilocks::Element[(uint64_t)NCOLS_HASH * (uint64_t)NROWS_HASH];
    Goldilocks::Element *result = new Goldilocks::Element[(uint64_t)HASH_SIZE * (uint64_t)NROWS_HASH];

    // Test vector: Fibonacci series on the columns and increase the initial values to the right,
    // 1 2 3 4  5  6  ... NUM_COLS
    // 1 2 3 4  5  6  ... NUM_COLS
    // 2 4 6 8  10 12 ... NUM_COLS + NUM_COLS
    // 3 6 9 12 15 18 ... NUM_COLS + NUM_COLS + NUM_COLS
    for (uint64_t i = 0; i < NCOLS_HASH; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + NCOLS_HASH] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < NROWS_HASH; j++)
    {
        for (uint64_t i = 0; i < NCOLS_HASH; i++)
        {
            cols[j * NCOLS_HASH + i] = cols[(j - 2) * NCOLS_HASH + i] + cols[(j - 1) * NCOLS_HASH + i];
        }
    }

    // Benchmark
    for (auto _ : state)
    {
#pragma omp parallel for num_threads(state.range(0)) schedule(static)
        for (uint64_t i = 0; i < NROWS_HASH; i++)
        {
            PoseidonGoldilocks::linear_hash(&result[i * HASH_SIZE], &cols[i * NCOLS_HASH], NCOLS_HASH);
        }
    }

    // Rate = time to process 1 linear hash per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (double)NROWS_HASH * (double)ceil((double)NCOLS_HASH / (double)RATE) / state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter((uint64_t)NROWS_HASH * (uint64_t)NCOLS_HASH * sizeof(Goldilocks::Element), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);

    delete[] cols;
    delete[] result;
}
#ifdef __AVX512__
static void LINEAR_HASH_BENCH_AVX512(benchmark::State &state)
{
    Goldilocks::Element *cols = new Goldilocks::Element[(uint64_t)NCOLS_HASH * (uint64_t)NROWS_HASH];
    Goldilocks::Element *result = new Goldilocks::Element[(uint64_t)HASH_SIZE * (uint64_t)NROWS_HASH];

    // Test vector: Fibonacci series on the columns and increase the initial values to the right,
    // 1 2 3 4  5  6  ... NUM_COLS
    // 1 2 3 4  5  6  ... NUM_COLS
    // 2 4 6 8  10 12 ... NUM_COLS + NUM_COLS
    // 3 6 9 12 15 18 ... NUM_COLS + NUM_COLS + NUM_COLS
    for (uint64_t i = 0; i < NCOLS_HASH; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + NCOLS_HASH] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < NROWS_HASH; j++)
    {
        for (uint64_t i = 0; i < NCOLS_HASH; i++)
        {
            cols[j * NCOLS_HASH + i] = cols[(j - 2) * NCOLS_HASH + i] + cols[(j - 1) * NCOLS_HASH + i];
        }
    }

    // Benchmark
    for (auto _ : state)
    {
#pragma omp parallel for num_threads(state.range(0)) schedule(static)
        for (uint64_t i = 0; i < NROWS_HASH; i += 2)
        {
            PoseidonGoldilocks::linear_hash_avx512(&result[i * HASH_SIZE], &cols[i * NCOLS_HASH], NCOLS_HASH);
        }
    }

    // Rate = time to process 1 linear hash per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (double)NROWS_HASH * (double)ceil((double)NCOLS_HASH / (double)RATE) / state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter((uint64_t)NROWS_HASH * (uint64_t)NCOLS_HASH * sizeof(Goldilocks::Element), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);

    delete[] cols;
    delete[] result;
}
#endif

static void MERKLETREE_BENCH(benchmark::State &state)
{
    Goldilocks::Element *cols = new Goldilocks::Element[(uint64_t)NCOLS_HASH * (uint64_t)NROWS_HASH];

    // Test vector: Fibonacci series on the columns and increase the initial values to the right,
    // 1 2 3 4  5  6  ... NUM_COLS
    // 1 2 3 4  5  6  ... NUM_COLS
    // 2 4 6 8  10 12 ... NUM_COLS + NUM_COLS
    // 3 6 9 12 15 18 ... NUM_COLS + NUM_COLS + NUM_COLS
    for (uint64_t i = 0; i < NCOLS_HASH; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + NCOLS_HASH] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < NROWS_HASH; j++)
    {
        for (uint64_t i = 0; i < NCOLS_HASH; i++)
        {
            cols[j * NCOLS_HASH + i] = cols[(j - 2) * NCOLS_HASH + i] + cols[(j - 1) * NCOLS_HASH + i];
        }
    }

    uint64_t numElementsTree = MerklehashGoldilocks::getTreeNumElements(NROWS_HASH);
    Goldilocks::Element *tree = new Goldilocks::Element[numElementsTree];

    // Benchmark
    for (auto _ : state)
    {
        PoseidonGoldilocks::merkletree_seq(tree, cols, NCOLS_HASH, NROWS_HASH, state.range(0));
    }
    Goldilocks::Element root[4];
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    // check results
    assert(Goldilocks::toU64(root[0]) == 0Xc935fb33cd86c0b8);
    assert(Goldilocks::toU64(root[1]) == 0X906753f66aa2791d);
    assert(Goldilocks::toU64(root[2]) == 0X3f6163b1b58a6ed7);
    assert(Goldilocks::toU64(root[3]) == 0Xbd575d9ed19d18c2);

    // Rate = time to process 1 linear hash per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (((double)NROWS_HASH * (double)ceil((double)NCOLS_HASH / (double)RATE)) + log2(NROWS_HASH)) / state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter((uint64_t)NROWS_HASH * (uint64_t)NCOLS_HASH * sizeof(Goldilocks::Element), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);
    delete[] cols;
    delete[] tree;
}
static void MERKLETREE_BENCH_AVX(benchmark::State &state)
{
    Goldilocks::Element *cols = new Goldilocks::Element[(uint64_t)NCOLS_HASH * (uint64_t)NROWS_HASH];

    // Test vector: Fibonacci series on the columns and increase the initial values to the right,
    // 1 2 3 4  5  6  ... NUM_COLS
    // 1 2 3 4  5  6  ... NUM_COLS
    // 2 4 6 8  10 12 ... NUM_COLS + NUM_COLS
    // 3 6 9 12 15 18 ... NUM_COLS + NUM_COLS + NUM_COLS
    for (uint64_t i = 0; i < NCOLS_HASH; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + NCOLS_HASH] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < NROWS_HASH; j++)
    {
        for (uint64_t i = 0; i < NCOLS_HASH; i++)
        {
            cols[j * NCOLS_HASH + i] = cols[(j - 2) * NCOLS_HASH + i] + cols[(j - 1) * NCOLS_HASH + i];
        }
    }

    uint64_t numElementsTree = MerklehashGoldilocks::getTreeNumElements(NROWS_HASH);
    Goldilocks::Element *tree = new Goldilocks::Element[numElementsTree];

    // Benchmark
    for (auto _ : state)
    {
        PoseidonGoldilocks::merkletree_avx(tree, cols, NCOLS_HASH, NROWS_HASH, state.range(0));
    }
    Goldilocks::Element root[4];
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    // check results
    assert(Goldilocks::toU64(root[0]) == 0Xc935fb33cd86c0b8);
    assert(Goldilocks::toU64(root[1]) == 0X906753f66aa2791d);
    assert(Goldilocks::toU64(root[2]) == 0X3f6163b1b58a6ed7);
    assert(Goldilocks::toU64(root[3]) == 0Xbd575d9ed19d18c2);

    // Rate = time to process 1 linear hash per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (((double)NROWS_HASH * (double)ceil((double)NCOLS_HASH / (double)RATE)) + log2(NROWS_HASH)) / state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter((uint64_t)NROWS_HASH * (uint64_t)NCOLS_HASH * sizeof(Goldilocks::Element), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);
    delete[] cols;
    delete[] tree;
}
#ifdef __AVX512__
static void MERKLETREE_BENCH_AVX512(benchmark::State &state)
{
    Goldilocks::Element *cols = new Goldilocks::Element[(uint64_t)NCOLS_HASH * (uint64_t)NROWS_HASH];

    // Test vector: Fibonacci series on the columns and increase the initial values to the right,
    // 1 2 3 4  5  6  ... NUM_COLS
    // 1 2 3 4  5  6  ... NUM_COLS
    // 2 4 6 8  10 12 ... NUM_COLS + NUM_COLS
    // 3 6 9 12 15 18 ... NUM_COLS + NUM_COLS + NUM_COLS
    for (uint64_t i = 0; i < NCOLS_HASH; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + NCOLS_HASH] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < NROWS_HASH; j++)
    {
        for (uint64_t i = 0; i < NCOLS_HASH; i++)
        {
            cols[j * NCOLS_HASH + i] = cols[(j - 2) * NCOLS_HASH + i] + cols[(j - 1) * NCOLS_HASH + i];
        }
    }

    uint64_t numElementsTree = MerklehashGoldilocks::getTreeNumElements(NROWS_HASH);
    Goldilocks::Element *tree = new Goldilocks::Element[numElementsTree];

    // Benchmark
    for (auto _ : state)
    {
        PoseidonGoldilocks::merkletree_avx512(tree, cols, NCOLS_HASH, NROWS_HASH, state.range(0));
    }
    Goldilocks::Element root[4];
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    // check results
    assert(Goldilocks::toU64(root[0]) == 0Xc935fb33cd86c0b8);
    assert(Goldilocks::toU64(root[1]) == 0X906753f66aa2791d);
    assert(Goldilocks::toU64(root[2]) == 0X3f6163b1b58a6ed7);
    assert(Goldilocks::toU64(root[3]) == 0Xbd575d9ed19d18c2);

    // Rate = time to process 1 linear hash per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (((double)NROWS_HASH * (double)ceil((double)NCOLS_HASH / (double)RATE)) + log2(NROWS_HASH)) / state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter((uint64_t)NROWS_HASH * (uint64_t)NCOLS_HASH * sizeof(Goldilocks::Element), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);
    delete[] cols;
    delete[] tree;
}
#endif

static void MERKLETREE_BATCH_BENCH(benchmark::State &state)
{
    Goldilocks::Element *cols = new Goldilocks::Element[(uint64_t)NCOLS_HASH * (uint64_t)NROWS_HASH];

    // Test vector: Fibonacci series on the columns and increase the initial values to the right,
    // 1 2 3 4  5  6  ... NUM_COLS
    // 1 2 3 4  5  6  ... NUM_COLS
    // 2 4 6 8  10 12 ... NUM_COLS + NUM_COLS
    // 3 6 9 12 15 18 ... NUM_COLS + NUM_COLS + NUM_COLS
    for (uint64_t i = 0; i < NCOLS_HASH; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + NCOLS_HASH] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < NROWS_HASH; j++)
    {
        for (uint64_t i = 0; i < NCOLS_HASH; i++)
        {
            cols[j * NCOLS_HASH + i] = cols[(j - 2) * NCOLS_HASH + i] + cols[(j - 1) * NCOLS_HASH + i];
        }
    }

    uint64_t numElementsTree = MerklehashGoldilocks::getTreeNumElements(NROWS_HASH);
    Goldilocks::Element *tree = new Goldilocks::Element[numElementsTree];

    // Benchmark
    for (auto _ : state)
    {
        PoseidonGoldilocks::merkletree_batch_seq(tree, cols, NCOLS_HASH, NROWS_HASH, (NCOLS_HASH + 3) / 4, state.range(0));
    }
    Goldilocks::Element root[4];
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    // check results
    assert(Goldilocks::toU64(root[0]) == 0X9ce696d26651e066);
    assert(Goldilocks::toU64(root[1]) == 0Xc7f662974b960728);
    assert(Goldilocks::toU64(root[2]) == 0Xad8a489fec5811a1);
    assert(Goldilocks::toU64(root[3]) == 0Xd34d83367c86e333);

    // Rate = time to process 1 linear hash per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (((double)NROWS_HASH * (double)ceil((double)NCOLS_HASH / (double)RATE)) + log2(NROWS_HASH)) / state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter((uint64_t)NROWS_HASH * (uint64_t)NCOLS_HASH * sizeof(Goldilocks::Element), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);
    delete[] cols;
    delete[] tree;
}
static void MERKLETREE_BATCH_BENCH_AVX(benchmark::State &state)
{
    Goldilocks::Element *cols = new Goldilocks::Element[(uint64_t)NCOLS_HASH * (uint64_t)NROWS_HASH];

    // Test vector: Fibonacci series on the columns and increase the initial values to the right,
    // 1 2 3 4  5  6  ... NUM_COLS
    // 1 2 3 4  5  6  ... NUM_COLS
    // 2 4 6 8  10 12 ... NUM_COLS + NUM_COLS
    // 3 6 9 12 15 18 ... NUM_COLS + NUM_COLS + NUM_COLS
    for (uint64_t i = 0; i < NCOLS_HASH; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + NCOLS_HASH] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < NROWS_HASH; j++)
    {
        for (uint64_t i = 0; i < NCOLS_HASH; i++)
        {
            cols[j * NCOLS_HASH + i] = cols[(j - 2) * NCOLS_HASH + i] + cols[(j - 1) * NCOLS_HASH + i];
        }
    }

    uint64_t numElementsTree = MerklehashGoldilocks::getTreeNumElements(NROWS_HASH);
    Goldilocks::Element *tree = new Goldilocks::Element[numElementsTree];

    // Benchmark
    for (auto _ : state)
    {
        PoseidonGoldilocks::merkletree_batch_avx(tree, cols, NCOLS_HASH, NROWS_HASH, (NCOLS_HASH + 3) / 4, state.range(0));
    }
    Goldilocks::Element root[4];
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    // check results
    assert(Goldilocks::toU64(root[0]) == 0X9ce696d26651e066);
    assert(Goldilocks::toU64(root[1]) == 0Xc7f662974b960728);
    assert(Goldilocks::toU64(root[2]) == 0Xad8a489fec5811a1);
    assert(Goldilocks::toU64(root[3]) == 0Xd34d83367c86e333);

    // Rate = time to process 1 linear hash per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (((double)NROWS_HASH * (double)ceil((double)NCOLS_HASH / (double)RATE)) + log2(NROWS_HASH)) / state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter((uint64_t)NROWS_HASH * (uint64_t)NCOLS_HASH * sizeof(Goldilocks::Element), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);
    delete[] cols;
    delete[] tree;
}
#ifdef __AVX512__
static void MERKLETREE_BATCH_BENCH_AVX512(benchmark::State &state)
{
    Goldilocks::Element *cols = new Goldilocks::Element[(uint64_t)NCOLS_HASH * (uint64_t)NROWS_HASH];

    // Test vector: Fibonacci series on the columns and increase the initial values to the right,
    // 1 2 3 4  5  6  ... NUM_COLS
    // 1 2 3 4  5  6  ... NUM_COLS
    // 2 4 6 8  10 12 ... NUM_COLS + NUM_COLS
    // 3 6 9 12 15 18 ... NUM_COLS + NUM_COLS + NUM_COLS
    for (uint64_t i = 0; i < NCOLS_HASH; i++)
    {
        cols[i] = Goldilocks::fromU64(i) + Goldilocks::one();
        cols[i + NCOLS_HASH] = Goldilocks::fromU64(i) + Goldilocks::one();
    }
    for (uint64_t j = 2; j < NROWS_HASH; j++)
    {
        for (uint64_t i = 0; i < NCOLS_HASH; i++)
        {
            cols[j * NCOLS_HASH + i] = cols[(j - 2) * NCOLS_HASH + i] + cols[(j - 1) * NCOLS_HASH + i];
        }
    }

    uint64_t numElementsTree = MerklehashGoldilocks::getTreeNumElements(NROWS_HASH);
    Goldilocks::Element *tree = new Goldilocks::Element[numElementsTree];

    // Benchmark
    for (auto _ : state)
    {
        PoseidonGoldilocks::merkletree_batch_avx512(tree, cols, NCOLS_HASH, NROWS_HASH, (NCOLS_HASH + 3) / 4, state.range(0));
    }
    Goldilocks::Element root[4];
    MerklehashGoldilocks::root(&(root[0]), tree, numElementsTree);

    // check results
    assert(Goldilocks::toU64(root[0]) == 0X9ce696d26651e066);
    assert(Goldilocks::toU64(root[1]) == 0Xc7f662974b960728);
    assert(Goldilocks::toU64(root[2]) == 0Xad8a489fec5811a1);
    assert(Goldilocks::toU64(root[3]) == 0Xd34d83367c86e333);

    // Rate = time to process 1 linear hash per core
    // BytesProcessed = total bytes processed per second on every iteration
    int threads_core = 2 * state.range(0) / omp_get_max_threads(); // we assume hyperthreading
    state.counters["Rate"] = benchmark::Counter(threads_core * (((double)NROWS_HASH * (double)ceil((double)NCOLS_HASH / (double)RATE)) + log2(NROWS_HASH)) / state.range(0), benchmark::Counter::kIsIterationInvariantRate | benchmark::Counter::kInvert);
    state.counters["BytesProcessed"] = benchmark::Counter((uint64_t)NROWS_HASH * (uint64_t)NCOLS_HASH * sizeof(Goldilocks::Element), benchmark::Counter::kIsIterationInvariantRate, benchmark::Counter::OneK::kIs1024);
    delete[] cols;
    delete[] tree;
}
#endif

static void NTT_BENCH(benchmark::State &state)
{
    NTT_Goldilocks gntt(FFT_SIZE, state.range(0));

    Goldilocks::Element *a = (Goldilocks::Element *)malloc((uint64_t)FFT_SIZE * (uint64_t)NUM_COLUMNS * sizeof(Goldilocks::Element));

#pragma omp parallel for
    for (uint64_t k = 0; k < NUM_COLUMNS; k++)
    {
        uint64_t offset = k * FFT_SIZE;
        a[offset] = Goldilocks::one();
        a[offset + 1] = Goldilocks::one();
        for (uint64_t i = 2; i < FFT_SIZE; i++)
        {
            a[offset + i] = a[offset + i - 1] + a[offset + i - 2];
        }
    }
    for (auto _ : state)
    {
#pragma omp parallel for num_threads(state.range(0))
        for (u_int64_t i = 0; i < NUM_COLUMNS; i++)
        {
            u_int64_t offset = i * FFT_SIZE;
            gntt.NTT(a + offset, a + offset, FFT_SIZE);
        }
    }
    free(a);
}
static void NTT_BLOCK_BENCH(benchmark::State &state)
{
    Goldilocks::Element *a = (Goldilocks::Element *)malloc((uint64_t)FFT_SIZE * (uint64_t)NUM_COLUMNS * sizeof(Goldilocks::Element));
    NTT_Goldilocks gntt(FFT_SIZE, state.range(0));

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
    for (auto _ : state)
    {
        gntt.NTT(a, a, FFT_SIZE, NUM_COLUMNS, NULL, NPHASES_NTT, NBLOCKS);
    }
    free(a);
}

static void LDE_BENCH(benchmark::State &state)
{
    Goldilocks::Element *a = (Goldilocks::Element *)malloc((uint64_t)FFT_SIZE * (uint64_t)NUM_COLUMNS * sizeof(Goldilocks::Element));
    NTT_Goldilocks gntt(FFT_SIZE, state.range(0));
    NTT_Goldilocks gntt_extension((FFT_SIZE << BLOWUP_FACTOR));

    a[0] = Goldilocks::one();
    a[1] = Goldilocks::one();
    for (uint64_t i = 2; i < (uint64_t)FFT_SIZE * (uint64_t)NUM_COLUMNS; i++)
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

    Goldilocks::Element *zero_array = (Goldilocks::Element *)malloc((uint64_t)((FFT_SIZE << BLOWUP_FACTOR) - FFT_SIZE) * sizeof(Goldilocks::Element));
#pragma omp parallel for num_threads(state.range(0))
    for (uint i = 0; i < ((FFT_SIZE << BLOWUP_FACTOR) - FFT_SIZE); i++)
    {
        zero_array[i] = Goldilocks::zero();
    }

    for (auto _ : state)
    {
        Goldilocks::Element *res = (Goldilocks::Element *)malloc((uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * (uint64_t)NUM_COLUMNS * sizeof(Goldilocks::Element));

#pragma omp parallel for num_threads(state.range(0))
        for (uint64_t k = 0; k < NUM_COLUMNS; k++)
        {
            for (int i = 0; i < FFT_SIZE; i++)
            {
                a[k * FFT_SIZE + i] = a[k * FFT_SIZE + i] * r[i];
            }
            std::memcpy(res, &a[k * FFT_SIZE], FFT_SIZE);
            std::memcpy(&res[FFT_SIZE], zero_array, (FFT_SIZE << BLOWUP_FACTOR) - FFT_SIZE);

            gntt_extension.NTT(res, res, (FFT_SIZE << BLOWUP_FACTOR));
        }
        free(res);
    }
    free(zero_array);
    free(a);
    free(r);
}
static void LDE_BLOCK_BENCH(benchmark::State &state)
{
    Goldilocks::Element *a = (Goldilocks::Element *)malloc((uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));
    NTT_Goldilocks gntt(FFT_SIZE, state.range(0));
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
    for (auto _ : state)
    {
        Goldilocks::Element shift = Goldilocks::fromU64(49); // TODO: ask for this number, where to put it how to calculate it

        gntt.INTT(a, a, FFT_SIZE, NUM_COLUMNS, NULL, NPHASES_NTT);

        // TODO: This can be pre-generated
        Goldilocks::Element *r = (Goldilocks::Element *)malloc(FFT_SIZE * sizeof(Goldilocks::Element));
        r[0] = Goldilocks::one();
        for (int i = 1; i < FFT_SIZE; i++)
        {
            r[i] = r[i - 1] * shift;
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
        for (uint64_t i = (uint64_t)FFT_SIZE * (uint64_t)NUM_COLUMNS; i < (uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * (uint64_t)NUM_COLUMNS; i++)
        {
            a[i] = Goldilocks::zero();
        }

        gntt_extension.NTT(a, a, (FFT_SIZE << BLOWUP_FACTOR), NUM_COLUMNS, NULL, NPHASES_LDE, NBLOCKS);
        free(r);
    }
    free(a);
}

static void EXTENDEDPOL_BENCH(benchmark::State &state)
{
    Goldilocks::Element *a = (Goldilocks::Element *)malloc((uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));
    Goldilocks::Element *b = (Goldilocks::Element *)malloc((uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));
    Goldilocks::Element *c = (Goldilocks::Element *)malloc((uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));

    NTT_Goldilocks ntt(FFT_SIZE, state.range(0));

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
    for (auto _ : state)
    {
        ntt.extendPol(b, a, FFT_SIZE << BLOWUP_FACTOR, FFT_SIZE, NUM_COLUMNS, c);
    }
    free(a);
    free(b);
    free(c);
}

BENCHMARK(ADD_OP_AVX_BENCH)
    ->Unit(benchmark::kMicrosecond)
    ->UseRealTime();

BENCHMARK(SUB_OP_AVX_BENCH)
    ->Unit(benchmark::kMicrosecond)
    ->UseRealTime();

BENCHMARK(MUL_OP_AVX_BENCH)
    ->Unit(benchmark::kMicrosecond)
    ->UseRealTime();
   

BENCHMARK(ADD_OP_BENCH)
    ->Unit(benchmark::kMicrosecond)
    ->UseRealTime();
BENCHMARK(SUB_OP_BENCH)
    ->Unit(benchmark::kMicrosecond)
    ->UseRealTime();
BENCHMARK(MUL_OP_BENCH)
    ->Unit(benchmark::kMicrosecond)
    ->UseRealTime();

BENCHMARK(INV_OP_BENCH)
    ->Unit(benchmark::kMicrosecond)
    ->UseRealTime();

BENCHMARK(POSEIDON_BENCH_FULL)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

BENCHMARK(POSEIDON_BENCH_FULL_AVX)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

#ifdef __AVX512__
BENCHMARK(POSEIDON_BENCH_FULL_AVX512)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();
#endif

BENCHMARK(POSEIDON_BENCH)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

BENCHMARK(POSEIDON_BENCH_AVX)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

#ifdef __AVX512__
BENCHMARK(POSEIDON_BENCH_AVX512)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();
#endif

BENCHMARK(LINEAR_HASH_BENCH)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

BENCHMARK(LINEAR_HASH_BENCH_AVX)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

#ifdef __AVX512__
BENCHMARK(LINEAR_HASH_BENCH_AVX512)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();
#endif

BENCHMARK(MERKLETREE_BENCH)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

BENCHMARK(MERKLETREE_BENCH_AVX)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

#ifdef __AVX512__
BENCHMARK(MERKLETREE_BENCH_AVX512)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();
#endif

BENCHMARK(MERKLETREE_BATCH_BENCH)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

BENCHMARK(MERKLETREE_BATCH_BENCH_AVX)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

#ifdef __AVX512__
BENCHMARK(MERKLETREE_BATCH_BENCH_AVX512)
    ->Unit(benchmark::kMicrosecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();
#endif

BENCHMARK(NTT_BENCH)
    ->Unit(benchmark::kSecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

BENCHMARK(NTT_BLOCK_BENCH)
    ->Unit(benchmark::kSecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

BENCHMARK(LDE_BENCH)
    ->Unit(benchmark::kSecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

BENCHMARK(LDE_BLOCK_BENCH)
    ->Unit(benchmark::kSecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

BENCHMARK(EXTENDEDPOL_BENCH)
    ->Unit(benchmark::kSecond)
    ->DenseRange(omp_get_max_threads() / 2, omp_get_max_threads(), omp_get_max_threads() / 2)
    ->UseRealTime();

BENCHMARK_MAIN();

// Build commands AVX:

// g++:
//   g++ benchs/bench.cpp src/* -lbenchmark -lomp -lpthread -lgmp  -std=c++17 -Wall -pthread -fopenmp -mavx2 -L$(find /usr/lib/llvm-* -name "libomp.so" | sed 's/libomp.so//') -O3 -o bench
//  Intel:
//  icpx -std=c++17 -Wall -march=native -O3 -qopenmp -qopenmp-simd -mavx2 -ipo -qopt-zmm-usage=high benchs/bench.cpp src/*.cpp -lbenchmark -lgmp -o bench

// Build commands AVX512:

// g++:
// g++ benchs/bench.cpp src/* -lbenchmark -lomp -lpthread -lgmp  -std=c++17 -Wall -pthread -fopenmp -mavx2 -mavx512f -L$(find /usr/lib/llvm-* -name "libomp.so" | sed 's/libomp.so//') -O3 -o bench -D__AVX512__
//  Intel:
//  icpx -std=c++17 -Wall -march=native -O3 -qopenmp -qopenmp-simd -mavx512f -mavx2 -axCORE-AVX512,CORE-AVX2 -ipo -qopt-zmm-usage=high benchs/bench.cpp src/*.cpp -lbenchmark -lgmp -o bench -D__AVX512__

//  RUN:
// ./bench --benchmark_filter=POSEIDON
