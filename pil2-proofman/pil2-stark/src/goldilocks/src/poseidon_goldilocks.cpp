#include "poseidon_goldilocks.hpp"
#include <math.h> /* floor */
#include "merklehash_goldilocks.hpp"

void PoseidonGoldilocks::hash_full_result_seq(Goldilocks::Element *state, const Goldilocks::Element *input)
{
    const int length = SPONGE_WIDTH * sizeof(Goldilocks::Element);
    std::memcpy(state, input, length);

    add_(state, &(PoseidonGoldilocksConstants::C[0]));
    for (int r = 0; r < HALF_N_FULL_ROUNDS - 1; r++)
    {
        pow7add_(state, &(PoseidonGoldilocksConstants::C[(r + 1) * SPONGE_WIDTH]));
        mvp_(state, PoseidonGoldilocksConstants::M);
    }
    pow7add_(state, &(PoseidonGoldilocksConstants::C[(HALF_N_FULL_ROUNDS * SPONGE_WIDTH)]));
    mvp_(state, PoseidonGoldilocksConstants::P);

    for (int r = 0; r < N_PARTIAL_ROUNDS; r++)
    {
        pow7(state[0]);
        state[0] = state[0] + PoseidonGoldilocksConstants::C[(HALF_N_FULL_ROUNDS + 1) * SPONGE_WIDTH + r];
        Goldilocks::Element s0 = dot_(state, &(PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r]));
        Goldilocks::Element W_[SPONGE_WIDTH];
        prod_(W_, state[0], &(PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r + SPONGE_WIDTH - 1]));
        add_(state, W_);
        state[0] = s0;
    }

    for (int r = 0; r < HALF_N_FULL_ROUNDS - 1; r++)
    {
        pow7add_(state, &(PoseidonGoldilocksConstants::C[(HALF_N_FULL_ROUNDS + 1) * SPONGE_WIDTH + N_PARTIAL_ROUNDS + r * SPONGE_WIDTH]));
        mvp_(state, PoseidonGoldilocksConstants::M);
    }
    pow7_(&(state[0]));
    mvp_(state, PoseidonGoldilocksConstants::M);
}
void PoseidonGoldilocks::linear_hash_seq(Goldilocks::Element *output, Goldilocks::Element *input, uint64_t size)
{
    uint64_t remaining = size;
    Goldilocks::Element state[SPONGE_WIDTH];

    if (size <= CAPACITY)
    {
        std::memcpy(output, input, size * sizeof(Goldilocks::Element));
        std::memset(&output[size], 0, (CAPACITY - size) * sizeof(Goldilocks::Element));
        return; // no need to hash
    }
    while (remaining)
    {
        if (remaining == size)
        {
            memset(state + RATE, 0, CAPACITY * sizeof(Goldilocks::Element));
        }
        else
        {
            std::memcpy(state + RATE, state, CAPACITY * sizeof(Goldilocks::Element));
        }

        uint64_t n = (remaining < RATE) ? remaining : RATE;
        memset(&state[n], 0, (RATE - n) * sizeof(Goldilocks::Element));
        std::memcpy(state, input + (size - remaining), n * sizeof(Goldilocks::Element));
        hash_full_result_seq(state, state);
        remaining -= n;
    }
    if (size > 0)
    {
        std::memcpy(output, state, CAPACITY * sizeof(Goldilocks::Element));
    }
    else
    {
        memset(output, 0, CAPACITY * sizeof(Goldilocks::Element));
    }
}
void PoseidonGoldilocks::merkletree_seq(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, int nThreads, uint64_t dim)
{
    if (num_rows == 0)
    {
        return;
    }

    Goldilocks::Element *cursor = tree;
    // memset(cursor, 0, num_rows * CAPACITY * sizeof(Goldilocks::Element));
    if (nThreads == 0)
        nThreads = omp_get_max_threads();

#pragma omp parallel for num_threads(nThreads)
    for (uint64_t i = 0; i < num_rows; i++)
    {
        linear_hash_seq(&cursor[i * CAPACITY], &input[i * num_cols * dim], num_cols * dim);
    }

    // Build the merkle tree
    uint64_t pending = num_rows;
    uint64_t nextN = floor((pending - 1) / 2) + 1;
    uint64_t nextIndex = 0;

    while (pending > 1)
    {
#pragma omp parallel for num_threads(nThreads)
        for (uint64_t i = 0; i < nextN; i++)
        {
            Goldilocks::Element pol_input[SPONGE_WIDTH];
            memset(pol_input, 0, SPONGE_WIDTH * sizeof(Goldilocks::Element));
            std::memcpy(pol_input, &cursor[nextIndex + i * RATE], RATE * sizeof(Goldilocks::Element));
            hash_seq((Goldilocks::Element(&)[CAPACITY])cursor[nextIndex + (pending + i) * CAPACITY], pol_input);
        }
        nextIndex += pending * CAPACITY;
        pending = pending / 2;
        nextN = floor((pending - 1) / 2) + 1;
    }
}
void PoseidonGoldilocks::merkletree_batch_seq(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, uint64_t batch_size, int nThreads, uint64_t dim)
{
    if (num_rows == 0)
    {
        return;
    }

    Goldilocks::Element *cursor = tree;
    uint64_t nbatches = 1;
    if (num_cols > 0)
    {
        nbatches = (num_cols + batch_size - 1) / batch_size;
    }
    uint64_t nlastb = num_cols - (nbatches - 1) * batch_size;

    if (nThreads == 0)
        nThreads = omp_get_max_threads();

#pragma omp parallel for num_threads(nThreads)
    for (uint64_t i = 0; i < num_rows; i++)
    {
        Goldilocks::Element buff0[nbatches * CAPACITY];
        for (uint64_t j = 0; j < nbatches; j++)
        {
            uint64_t nn = batch_size;
            if (j == nbatches - 1)
                nn = nlastb;
            linear_hash_seq(&buff0[j * CAPACITY], &input[i * num_cols * dim + j * batch_size * dim], nn * dim);
        }
        linear_hash_seq(&cursor[i * CAPACITY], buff0, nbatches * CAPACITY);
    }

    // Build the merkle tree
    uint64_t pending = num_rows;
    uint64_t nextN = floor((pending - 1) / 2) + 1;
    uint64_t nextIndex = 0;

    while (pending > 1)
    {
#pragma omp parallel for num_threads(nThreads)
        for (uint64_t i = 0; i < nextN; i++)
        {
            Goldilocks::Element pol_input[SPONGE_WIDTH];
            memset(pol_input, 0, SPONGE_WIDTH * sizeof(Goldilocks::Element));
            std::memcpy(pol_input, &cursor[nextIndex + i * RATE], RATE * sizeof(Goldilocks::Element));
            hash_seq((Goldilocks::Element(&)[CAPACITY])cursor[nextIndex + (pending + i) * CAPACITY], pol_input);
        }
        nextIndex += pending * CAPACITY;
        pending = pending / 2;
        nextN = floor((pending - 1) / 2) + 1;
    }
}

void PoseidonGoldilocks::hash_full_result(Goldilocks::Element *state, const Goldilocks::Element *input)
{
    const int length = SPONGE_WIDTH * sizeof(Goldilocks::Element);
    std::memcpy(state, input, length);
    __m256i st0, st1, st2;
    Goldilocks::load_avx(st0, &(state[0]));
    Goldilocks::load_avx(st1, &(state[4]));
    Goldilocks::load_avx(st2, &(state[8]));
    add_avx_small(st0, st1, st2, &(PoseidonGoldilocksConstants::C[0]));

    for (int r = 0; r < HALF_N_FULL_ROUNDS - 1; r++)
    {
        pow7_avx(st0, st1, st2);
        add_avx_small(st0, st1, st2, &(PoseidonGoldilocksConstants::C[(r + 1) * SPONGE_WIDTH]));
        Goldilocks::mmult_avx_8(st0, st1, st2, &(PoseidonGoldilocksConstants::M_[0]));
    }
    pow7_avx(st0, st1, st2);
    add_avx(st0, st1, st2, &(PoseidonGoldilocksConstants::C[(HALF_N_FULL_ROUNDS * SPONGE_WIDTH)]));
    Goldilocks::mmult_avx(st0, st1, st2, &(PoseidonGoldilocksConstants::P_[0]));

    Goldilocks::store_avx(&(state[0]), st0);
    Goldilocks::Element state0_ = state[0];
    Goldilocks::Element state0;

    __m256i mask = _mm256_set_epi64x(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0);
    for (int r = 0; r < N_PARTIAL_ROUNDS; r++)
    {
        state0 = state0_;
        pow7(state0);
        state0 = state0 + PoseidonGoldilocksConstants::C[(HALF_N_FULL_ROUNDS + 1) * SPONGE_WIDTH + r];
        state0_ = state0 * PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r];
        st0 = _mm256_and_si256(st0, mask);
        state0_ = state0_ + Goldilocks::dot_avx(st0, st1, st2, &(PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r]));
        __m256i scalar1 = _mm256_set1_epi64x(state0.fe);
        __m256i w0, w1, w2, s0, s1, s2;
        Goldilocks::load_avx(s0, &(PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r + SPONGE_WIDTH - 1]));
        Goldilocks::load_avx(s1, &(PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r + SPONGE_WIDTH - 1 + 4]));
        Goldilocks::load_avx(s2, &(PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r + SPONGE_WIDTH - 1 + 8]));
        Goldilocks::mult_avx(w0, scalar1, s0);
        Goldilocks::mult_avx(w1, scalar1, s1);
        Goldilocks::mult_avx(w2, scalar1, s2);
        Goldilocks::add_avx(st0, st0, w0);
        Goldilocks::add_avx(st1, st1, w1);
        Goldilocks::add_avx(st2, st2, w2);
        state0 = state0 + PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r + SPONGE_WIDTH - 1];
    }
    Goldilocks::store_avx(&(state[0]), st0);
    state[0] = state0_;
    Goldilocks::load_avx(st0, &(state[0]));

    for (int r = 0; r < HALF_N_FULL_ROUNDS - 1; r++)
    {
        pow7_avx(st0, st1, st2);
        add_avx_small(st0, st1, st2, &(PoseidonGoldilocksConstants::C[(HALF_N_FULL_ROUNDS + 1) * SPONGE_WIDTH + N_PARTIAL_ROUNDS + r * SPONGE_WIDTH]));
        Goldilocks::mmult_avx_8(st0, st1, st2, &(PoseidonGoldilocksConstants::M_[0]));
    }
    pow7_avx(st0, st1, st2);
    Goldilocks::mmult_avx_8(st0, st1, st2, &(PoseidonGoldilocksConstants::M_[0]));

    Goldilocks::store_avx(&(state[0]), st0);
    Goldilocks::store_avx(&(state[4]), st1);
    Goldilocks::store_avx(&(state[8]), st2);
}
void PoseidonGoldilocks::linear_hash(Goldilocks::Element *output, Goldilocks::Element *input, uint64_t size)
{
    uint64_t remaining = size;
    Goldilocks::Element state[SPONGE_WIDTH];

    if (size <= CAPACITY)
    {
        std::memcpy(output, input, size * sizeof(Goldilocks::Element));
        std::memset(&output[size], 0, (CAPACITY - size) * sizeof(Goldilocks::Element));
        return; // no need to hash
    }
    while (remaining)
    {
        if (remaining == size)
        {
            memset(state + RATE, 0, CAPACITY * sizeof(Goldilocks::Element));
        }
        else
        {
            std::memcpy(state + RATE, state, CAPACITY * sizeof(Goldilocks::Element));
        }

        uint64_t n = (remaining < RATE) ? remaining : RATE;
        memset(&state[n], 0, (RATE - n) * sizeof(Goldilocks::Element));
        std::memcpy(state, input + (size - remaining), n * sizeof(Goldilocks::Element));
        hash_full_result(state, state);
        remaining -= n;
    }
    if (size > 0)
    {
        std::memcpy(output, state, CAPACITY * sizeof(Goldilocks::Element));
    }
    else
    {
        memset(output, 0, CAPACITY * sizeof(Goldilocks::Element));
    }
}
void PoseidonGoldilocks::merkletree_avx(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, int nThreads, uint64_t dim)
{
    if (num_rows == 0)
    {
        return;
    }
    Goldilocks::Element *cursor = tree;
    // memset(cursor, 0, num_rows * CAPACITY * sizeof(Goldilocks::Element));
    if (nThreads == 0)
        nThreads = omp_get_max_threads();

#pragma omp parallel for num_threads(nThreads)
    for (uint64_t i = 0; i < num_rows; i++)
    {
        linear_hash(&cursor[i * CAPACITY], &input[i * num_cols * dim], num_cols * dim);
    }

    // Build the merkle tree
    uint64_t pending = num_rows;
    uint64_t nextN = floor((pending - 1) / 2) + 1;
    uint64_t nextIndex = 0;
    while (pending > 1)
    {
#pragma omp parallel for num_threads(nThreads)
        for (uint64_t i = 0; i < nextN; i++)
        {
            Goldilocks::Element pol_input[SPONGE_WIDTH];
            memset(pol_input, 0, SPONGE_WIDTH * sizeof(Goldilocks::Element));
            std::memcpy(pol_input, &cursor[nextIndex + i * RATE], RATE * sizeof(Goldilocks::Element));
            hash((Goldilocks::Element(&)[CAPACITY])cursor[nextIndex + (pending + i) * CAPACITY], pol_input);
        }
        nextIndex += pending * CAPACITY;
        pending = pending / 2;
        nextN = floor((pending - 1) / 2) + 1;
    }
}
void PoseidonGoldilocks::merkletree_batch_avx(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, uint64_t batch_size, int nThreads, uint64_t dim)
{
    if (num_rows == 0)
    {
        return;
    }
    Goldilocks::Element *cursor = tree;
    uint64_t nbatches = 1;
    if (num_cols > 0)
    {
        nbatches = (num_cols + batch_size - 1) / batch_size;
    }
    uint64_t nlastb = num_cols - (nbatches - 1) * batch_size;

    if (nThreads == 0)
        nThreads = omp_get_max_threads();

#pragma omp parallel for num_threads(nThreads)
    for (uint64_t i = 0; i < num_rows; i++)
    {
        Goldilocks::Element buff0[nbatches * CAPACITY];
        for (uint64_t j = 0; j < nbatches; j++)
        {
            uint64_t nn = batch_size;
            if (j == nbatches - 1)
                nn = nlastb;
            linear_hash(&buff0[j * CAPACITY], &input[i * num_cols * dim + j * batch_size * dim], nn * dim);
        }
        linear_hash(&cursor[i * CAPACITY], buff0, nbatches * CAPACITY);
    }

    // Build the merkle tree
    uint64_t pending = num_rows;
    uint64_t nextN = floor((pending - 1) / 2) + 1;
    uint64_t nextIndex = 0;

    while (pending > 1)
    {
#pragma omp parallel for num_threads(nThreads)
        for (uint64_t i = 0; i < nextN; i++)
        {
            Goldilocks::Element pol_input[SPONGE_WIDTH];
            memset(pol_input, 0, SPONGE_WIDTH * sizeof(Goldilocks::Element));
            std::memcpy(pol_input, &cursor[nextIndex + i * RATE], RATE * sizeof(Goldilocks::Element));
            hash((Goldilocks::Element(&)[CAPACITY])cursor[nextIndex + (pending + i) * CAPACITY], pol_input);
        }
        nextIndex += pending * CAPACITY;
        pending = pending / 2;
        nextN = floor((pending - 1) / 2) + 1;
    }
}

#ifdef __AVX512__
void PoseidonGoldilocks::hash_full_result_avx512(Goldilocks::Element *state, const Goldilocks::Element *input)
{

    const int length = 2 * SPONGE_WIDTH * sizeof(Goldilocks::Element);
    std::memcpy(state, input, length);
    __m512i st0, st1, st2;
    Goldilocks::load_avx512(st0, &(state[0]));
    Goldilocks::load_avx512(st1, &(state[8]));
    Goldilocks::load_avx512(st2, &(state[16]));
    add_avx512_small(st0, st1, st2, &(PoseidonGoldilocksConstants::C[0]));

    for (int r = 0; r < HALF_N_FULL_ROUNDS - 1; r++)
    {
        pow7_avx512(st0, st1, st2);
        add_avx512_small(st0, st1, st2, &(PoseidonGoldilocksConstants::C[(r + 1) * SPONGE_WIDTH])); // rick
        Goldilocks::mmult_avx512_8(st0, st1, st2, &(PoseidonGoldilocksConstants::M_[0]));
    }
    pow7_avx512(st0, st1, st2);
    add_avx512(st0, st1, st2, &(PoseidonGoldilocksConstants::C[(HALF_N_FULL_ROUNDS * SPONGE_WIDTH)]));
    Goldilocks::mmult_avx512(st0, st1, st2, &(PoseidonGoldilocksConstants::P_[0]));

    Goldilocks::store_avx512(&(state[0]), st0);
    Goldilocks::Element s04_[2] = {state[0], state[4]};
    Goldilocks::Element s04[2];

    __m512i mask = _mm512_set_epi64(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0); // rick, not better to define where u use it?
    for (int r = 0; r < N_PARTIAL_ROUNDS; r++)
    {
        s04[0] = s04_[0];
        s04[1] = s04_[1];
        pow7(s04[0]);
        pow7(s04[1]);
        s04[0] = s04[0] + PoseidonGoldilocksConstants::C[(HALF_N_FULL_ROUNDS + 1) * SPONGE_WIDTH + r];
        s04[1] = s04[1] + PoseidonGoldilocksConstants::C[(HALF_N_FULL_ROUNDS + 1) * SPONGE_WIDTH + r];
        s04_[0] = s04[0] * PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r];
        s04_[1] = s04[1] * PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r];
        st0 = _mm512_and_si512(st0, mask); // rick, do we need a new one?
        Goldilocks::Element aux[2];
        Goldilocks::dot_avx512(aux, st0, st1, st2, &(PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r]));
        s04_[0] = s04_[0] + aux[0];
        s04_[1] = s04_[1] + aux[1];
        __m512i scalar1 = _mm512_set_epi64(s04[1].fe, s04[1].fe, s04[1].fe, s04[1].fe, s04[0].fe, s04[0].fe, s04[0].fe, s04[0].fe);
        __m512i w0, w1, w2;

        const Goldilocks::Element *auxS = &(PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r + SPONGE_WIDTH - 1]);
        __m512i s0 = _mm512_set4_epi64(auxS[3].fe, auxS[2].fe, auxS[1].fe, auxS[0].fe);
        __m512i s1 = _mm512_set4_epi64(auxS[7].fe, auxS[6].fe, auxS[5].fe, auxS[4].fe);
        __m512i s2 = _mm512_set4_epi64(auxS[11].fe, auxS[10].fe, auxS[9].fe, auxS[8].fe);

        Goldilocks::mult_avx512(w0, scalar1, s0);
        Goldilocks::mult_avx512(w1, scalar1, s1);
        Goldilocks::mult_avx512(w2, scalar1, s2);
        Goldilocks::add_avx512(st0, st0, w0);
        Goldilocks::add_avx512(st1, st1, w1);
        Goldilocks::add_avx512(st2, st2, w2);
        s04[0] = s04[0] + PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r + SPONGE_WIDTH - 1];
        s04[1] = s04[1] + PoseidonGoldilocksConstants::S[(SPONGE_WIDTH * 2 - 1) * r + SPONGE_WIDTH - 1];
    }

    Goldilocks::store_avx512(&(state[0]), st0);
    state[0] = s04_[0];
    state[4] = s04_[1];
    Goldilocks::load_avx512(st0, &(state[0]));

    for (int r = 0; r < HALF_N_FULL_ROUNDS - 1; r++)
    {
        pow7_avx512(st0, st1, st2);
        add_avx512_small(st0, st1, st2, &(PoseidonGoldilocksConstants::C[(HALF_N_FULL_ROUNDS + 1) * SPONGE_WIDTH + N_PARTIAL_ROUNDS + r * SPONGE_WIDTH]));
        Goldilocks::mmult_avx512_8(st0, st1, st2, &(PoseidonGoldilocksConstants::M_[0]));
    }
    pow7_avx512(st0, st1, st2);
    Goldilocks::mmult_avx512_8(st0, st1, st2, &(PoseidonGoldilocksConstants::M_[0]));

    Goldilocks::store_avx512(&(state[0]), st0);
    Goldilocks::store_avx512(&(state[8]), st1);
    Goldilocks::store_avx512(&(state[16]), st2);
}
void PoseidonGoldilocks::linear_hash_avx512(Goldilocks::Element *output, Goldilocks::Element *input, uint64_t size)
{
    uint64_t remaining = size;
    Goldilocks::Element state[2 * SPONGE_WIDTH];

    if (size <= CAPACITY)
    {
        std::memcpy(output, input, size * sizeof(Goldilocks::Element));
        std::memset(output + size, 0, (CAPACITY - size) * sizeof(Goldilocks::Element));
        std::memcpy(output + CAPACITY, input + size, size * sizeof(Goldilocks::Element));
        std::memset(output + CAPACITY + size, 0, (CAPACITY - size) * sizeof(Goldilocks::Element));
        return; // no need to hash
    }
    while (remaining)
    {
        if (remaining == size)
        {
            memset(state + 2 * RATE, 0, 2 * CAPACITY * sizeof(Goldilocks::Element));
        }
        else
        {
            std::memcpy(state + 2 * RATE, state, 2 * CAPACITY * sizeof(Goldilocks::Element));
        }

        uint64_t n = (remaining < RATE) ? remaining : RATE;
        memset(state, 0, 2 * RATE * sizeof(Goldilocks::Element));

        if (n <= 4)
        {
            std::memcpy(state, input + (size - remaining), n * sizeof(Goldilocks::Element));
            std::memcpy(state + 4, input + size + (size - remaining), n * sizeof(Goldilocks::Element));
        }
        else
        {
            std::memcpy(state, input + (size - remaining), 4 * sizeof(Goldilocks::Element));
            std::memcpy(state + 4, input + size + (size - remaining), 4 * sizeof(Goldilocks::Element));
            std::memcpy(state + 8, input + (size - remaining) + 4, (n - 4) * sizeof(Goldilocks::Element));
            std::memcpy(state + 12, input + size + (size - remaining) + 4, (n - 4) * sizeof(Goldilocks::Element));
        }

        hash_full_result_avx512(state, state);
        remaining -= n;
    }
    if (size > 0)
    {
        std::memcpy(output, state, 2 * CAPACITY * sizeof(Goldilocks::Element));
    }
    else
    {
        memset(output, 0, 2 * CAPACITY * sizeof(Goldilocks::Element));
    }
}
void PoseidonGoldilocks::merkletree_avx512(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, int nThreads, uint64_t dim)
{
    if (num_rows == 0)
    {
        return;
    }
    Goldilocks::Element *cursor = tree;
    // memset(cursor, 0, num_rows * CAPACITY * sizeof(Goldilocks::Element));
    if (nThreads == 0)
        nThreads = omp_get_max_threads();

#pragma omp parallel for num_threads(nThreads)
    for (uint64_t i = 0; i < num_rows; i += 2)
    {
        linear_hash_avx512(&cursor[i * CAPACITY], &input[i * num_cols * dim], num_cols * dim);
    }

    // Build the merkle tree
    uint64_t pending = num_rows;
    uint64_t nextN = floor((pending - 1) / 2) + 1;
    uint64_t nextIndex = 0;

    while (pending > 1)
    {
#pragma omp parallel for num_threads(nThreads)
        for (uint64_t i = 0; i < nextN; i++)
        {
            Goldilocks::Element pol_input[SPONGE_WIDTH];
            memset(pol_input, 0, SPONGE_WIDTH * sizeof(Goldilocks::Element));
            std::memcpy(pol_input, &cursor[nextIndex + i * RATE], RATE * sizeof(Goldilocks::Element));
            hash((Goldilocks::Element(&)[CAPACITY])cursor[nextIndex + (pending + i) * CAPACITY], pol_input);
        }
        nextIndex += pending * CAPACITY;
        pending = pending / 2;
        nextN = floor((pending - 1) / 2) + 1;
    }
}
void PoseidonGoldilocks::merkletree_batch_avx512(Goldilocks::Element *tree, Goldilocks::Element *input, uint64_t num_cols, uint64_t num_rows, uint64_t batch_size, int nThreads, uint64_t dim)
{
    if (num_rows == 0)
    {
        return;
    }
    Goldilocks::Element *cursor = tree;
    uint64_t nbatches = 1;
    if (num_cols > 0)
    {
        nbatches = (num_cols + batch_size - 1) / batch_size;
    }
    uint64_t nlastb = num_cols - (nbatches - 1) * batch_size;

    if (nThreads == 0)
        nThreads = omp_get_max_threads();

#pragma omp parallel for num_threads(nThreads)
    for (uint64_t i = 0; i < num_rows; i += 2)
    {
        Goldilocks::Element buff0[2 * nbatches * CAPACITY];
        for (uint64_t j = 0; j < nbatches; ++j)
        {
            uint64_t nn = batch_size;
            if (j == nbatches - 1)
                nn = nlastb;
            Goldilocks::Element buff1[2 * nn * dim];
            Goldilocks::Element buff2[2 * CAPACITY];
            std::memcpy(&buff1[0], &input[i * num_cols * dim + j * batch_size * dim], dim * nn * sizeof(Goldilocks::Element));
            std::memcpy(&buff1[nn * dim], &input[(i + 1) * num_cols * dim + j * batch_size * dim], dim * nn * sizeof(Goldilocks::Element));
            linear_hash_avx512(buff2, buff1, nn * dim);
            memcpy(&buff0[j * CAPACITY], buff2, CAPACITY * sizeof(Goldilocks::Element));
            memcpy(&buff0[(j + nbatches) * CAPACITY], &buff2[CAPACITY], CAPACITY * sizeof(Goldilocks::Element));
        }
        linear_hash_avx512(&cursor[i * CAPACITY], buff0, nbatches * CAPACITY);
    }

    // Build the merkle tree
    uint64_t pending = num_rows;
    uint64_t nextN = floor((pending - 1) / 2) + 1;
    uint64_t nextIndex = 0;

    while (pending > 1)
    {
#pragma omp parallel for num_threads(nThreads)
        for (uint64_t i = 0; i < nextN; i++)
        {
            Goldilocks::Element pol_input[SPONGE_WIDTH];
            memset(pol_input, 0, SPONGE_WIDTH * sizeof(Goldilocks::Element));
            std::memcpy(pol_input, &cursor[nextIndex + i * RATE], RATE * sizeof(Goldilocks::Element));
            hash((Goldilocks::Element(&)[CAPACITY])cursor[nextIndex + (pending + i) * CAPACITY], pol_input);
        }
        nextIndex += pending * CAPACITY;
        pending = pending / 2;
        nextN = floor((pending - 1) / 2) + 1;
    }
}
#endif