#include <gtest/gtest.h>
#include "../src/goldilocks_base_field.hpp"
#include "../src/poseidon_goldilocks.hpp"
#include "../src/ntt_goldilocks.hpp"
#include "../utils/timer_gl.hpp"
#include "../utils/cuda_utils.hpp"
#include "../src/goldilocks_cubic_extension.hpp"
#include "../src/goldilocks_cubic_extension_pack.hpp"

#define FFT_SIZE (1 << 24)
#define BLOWUP_FACTOR 1
#define NUM_COLUMNS 32

#ifdef __USE_CUDA__
#include "../src/gl64_t.cuh"
#include "../utils/cuda_utils.cuh"
#include "../src/goldilocks_cubic_extension.cuh"
TEST(GOLDILOCKS_TEST, full_gpu)
{
    Goldilocks::Element *a;
    Goldilocks::Element *b;
    Goldilocks::Element *c;
    cudaMallocManaged(&a, (uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));
    cudaMallocManaged(&b, (uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));
    cudaMallocManaged(&c, (uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));

    Goldilocks::Element *bb;
    Goldilocks::Element *cc;
    cudaMallocManaged(&bb, (uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));
    cudaMallocManaged(&cc, (uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));

    NTT_Goldilocks ntt(FFT_SIZE);
    warmup_all_gpus();
    alloc_pinned_mem((uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS);

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

    TimerStart(LDE_MerkleTree_MultiGPU_viaCPU);
    ntt.LDE_MerkleTree_MultiGPU_viaCPU(b, a, FFT_SIZE, FFT_SIZE<<BLOWUP_FACTOR, NUM_COLUMNS, c);
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_viaCPU);

    TimerStart(LDE_MerkleTree_MultiGPU_Steps);
    ntt.LDE_MerkleTree_MultiGPU_Steps(bb, a, FFT_SIZE, FFT_SIZE<<BLOWUP_FACTOR, NUM_COLUMNS, cc, 5);
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_Steps);

    printf("check1:\n");
    for (uint64_t i = 0; i<(uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS; i++) {
        if (Goldilocks::toU64(c[i]) != Goldilocks::toU64(cc[i])) {
            printf("index:%lu, left:%lu, right:%lu\n", i, Goldilocks::toU64(c[i]), Goldilocks::toU64(cc[i]));
            return;
        }
        //ASSERT_EQ(Goldilocks::toU64(c[i]), Goldilocks::toU64(cc[i]));
    }

    printf("check2:\n");
    uint64_t total = (uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * 8;
    uint64_t count = 0;
    for (uint64_t i = 0; i<total; i++) {
        if (Goldilocks::toU64(b[i]) != Goldilocks::toU64(bb[i])) {
            printf("index:%lu, left:%lu, right:%lu\n", i, Goldilocks::toU64(b[i]), Goldilocks::toU64(bb[i]));
            return;
        }
        //ASSERT_EQ(Goldilocks::toU64(b[i]), Goldilocks::toU64(bb[i]));
    }

    printf("total:%lu, not equal:%lu:\n", total, count);

    uint64_t free_mem, total_mem;
    cudaMemGetInfo(&free_mem, &total_mem);
    printf("free_mem: %lu, total_mem: %lu\n", free_mem, total_mem);

    cudaFree(a);
    cudaFree(b);
    cudaFree(c);
    free_pinned_mem();
}

TEST(GOLDILOCKS_TEST, lde)
{
    Goldilocks::Element *a;
    Goldilocks::Element *b;
    Goldilocks::Element *buffer;
    cudaMallocHost(&a, (uint64_t)(FFT_SIZE) * NUM_COLUMNS * sizeof(Goldilocks::Element));
    cudaMallocHost(&b, (uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));

    cudaMallocHost(&buffer, (uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * 128 * sizeof(Goldilocks::Element));

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
    warmup_all_gpus();
    TimerStart(extendPol_MultiGPU);
    //ntt.extendPol_MultiGPU(b, a, FFT_SIZE<<BLOWUP_FACTOR, FFT_SIZE, NUM_COLUMNS, buffer, 16);
    ntt.extendPol_GPU(b, a, FFT_SIZE<<BLOWUP_FACTOR, FFT_SIZE, NUM_COLUMNS);
    //ntt.LDE_MultiGPU_Full(b, a, FFT_SIZE, FFT_SIZE<<BLOWUP_FACTOR, NUM_COLUMNS, buffer, 8);
    TimerStopAndLog(extendPol_MultiGPU);

    cudaFreeHost(a);
    cudaFreeHost(b);
    cudaFreeHost(buffer);
}

TEST(GOLDILOCKS_TEST, mt)
{
    Goldilocks::Element *tree_a;
    Goldilocks::Element *tree_b;
    Goldilocks::Element *input;
    uint64_t num_rows = FFT_SIZE << BLOWUP_FACTOR;
    uint64_t numElementsTree = (2* num_rows -1) * 4;
    cudaMallocHost(&tree_a, numElementsTree * sizeof(Goldilocks::Element));
    cudaMallocHost(&tree_b, numElementsTree * sizeof(Goldilocks::Element));

    cudaMallocHost(&input, num_rows * NUM_COLUMNS * sizeof(Goldilocks::Element));

    for (uint i = 0; i < 2; i++)
    {
        for (uint j = 0; j < NUM_COLUMNS; j++)
        {
            Goldilocks::add(input[i * NUM_COLUMNS + j], Goldilocks::one(), Goldilocks::fromU64(j));
        }
    }

    for (uint64_t i = 2; i < FFT_SIZE; i++)
    {
        for (uint j = 0; j < NUM_COLUMNS; j++)
        {
            input[i * NUM_COLUMNS + j] = input[NUM_COLUMNS * (i - 1) + j] + input[NUM_COLUMNS * (i - 2) + j];
        }
    }



    warmup_all_gpus();
    alloc_pinned_mem(numElementsTree);

    TimerStart(merkletree_cuda_async);
    PoseidonGoldilocks::merkletree_cuda_async(tree_a, input, NUM_COLUMNS, num_rows);
    TimerStopAndLog(merkletree_cuda_async);

    TimerStart(merkletree_cuda);
    PoseidonGoldilocks::merkletree_avx(tree_b, input, NUM_COLUMNS, num_rows);
    TimerStopAndLog(merkletree_cuda);

    for (uint64_t i=0;i<numElementsTree;i++) {
        uint64_t left = Goldilocks::toU64(tree_a[i]);
        uint64_t right = Goldilocks::toU64(tree_b[i]);
        if (left != right) {
            printf("i:%lu, left:%lu, right:%lu\n", i, left, right);
            assert(0);
        }
    }

    cudaFreeHost(tree_a);
    cudaFreeHost(tree_b);
    cudaFreeHost(input);
}

#endif

TEST(GOLDILOCKS_TEST, full_cpu)
{
    Goldilocks::Element *a = (Goldilocks::Element *)malloc((uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));
    Goldilocks::Element *b = (Goldilocks::Element *)malloc((uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));
    Goldilocks::Element *c = (Goldilocks::Element *)malloc((uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS * sizeof(Goldilocks::Element));

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

    TimerStart(LDE_MerkleTree_CPU);
    ntt.extendPol(c, a, FFT_SIZE<<BLOWUP_FACTOR, FFT_SIZE, NUM_COLUMNS, b);
    PoseidonGoldilocks::merkletree_avx(b, c, NUM_COLUMNS, FFT_SIZE<<BLOWUP_FACTOR);
    TimerStopAndLog(LDE_MerkleTree_CPU);

    free(a);
    free(b);
    free(c);
}

__global__ void add_one(uint64_t *a, uint64_t n) {
    uint64_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= n) {
        return;
    }
    a[idx] += 1;
}

#include <cstdlib>

void init(uint64_t *a, uint64_t n) {
    a[0] = rand() % 100;
    for (uint64_t i = 1; i < n; i++) {
        a[i] = (a[i-1]*a[i-1])%18446744069414584321lu;
    }
}

TEST(GOLDILOCKS_TEST, copy)
{
    CHECKCUDAERR(cudaSetDevice(0));
    cudaDeviceProp cuInfo;
    CHECKCUDAERR(cudaGetDeviceProperties(&cuInfo, 0));
    printf("Name: %s\n", cuInfo.name);
    printf("deviceOverlap:%d\n", cuInfo.deviceOverlap);
    printf("asyncEngineCount:%d\n", cuInfo.asyncEngineCount);

    uint64_t total = (uint64_t)(FFT_SIZE << BLOWUP_FACTOR) * NUM_COLUMNS;

    uint64_t *a;
    uint64_t *b;
    CHECKCUDAERR(cudaMalloc(&b, total * sizeof(uint64_t)));

    a = (uint64_t *)malloc(total * sizeof(uint64_t));
    init(a, total);
    TimerStart(MEMCPY_H_TO_D);
    CHECKCUDAERR(cudaMemcpy(b, a, total * sizeof(uint64_t), cudaMemcpyHostToDevice));
    TimerStopAndLog(MEMCPY_H_TO_D);
    add_one<<<total/16, 16>>>(b, total);
    TimerStart(MEMCPY_D_TO_H);
    CHECKCUDAERR(cudaMemcpy(a, b, total * sizeof(uint64_t), cudaMemcpyDeviceToHost));
    TimerStopAndLog(MEMCPY_D_TO_H);
    free(a);

    cudaMallocManaged(&a, total * sizeof(uint64_t));
    init(a, total);
    TimerStart(MEMCPY_H_TO_D2);
    CHECKCUDAERR(cudaMemcpy(b, a, total * sizeof(uint64_t), cudaMemcpyHostToDevice));
    TimerStopAndLog(MEMCPY_H_TO_D2);
    add_one<<<total/16, 16>>>(b, total);
    TimerStart(MEMCPY_D_TO_H2);
    CHECKCUDAERR(cudaMemcpy(a, b, total * sizeof(uint64_t), cudaMemcpyDeviceToHost));
    TimerStopAndLog(MEMCPY_D_TO_H2);
    cudaFree(a);

    CHECKCUDAERR(cudaMallocHost(&a, total * sizeof(uint64_t)));
    init(a, total);
    TimerStart(MEMCPY_D_TO_H3);
    CHECKCUDAERR(cudaMemcpy(a, b, total * sizeof(uint64_t), cudaMemcpyDeviceToHost));
    TimerStopAndLog(MEMCPY_D_TO_H3);
    add_one<<<total/16, 16>>>(b, total);
    TimerStart(MEMCPY_H_TO_D3);
    CHECKCUDAERR(cudaMemcpy(b, a, total * sizeof(uint64_t), cudaMemcpyHostToDevice));
    TimerStopAndLog(MEMCPY_H_TO_D3);

    init(a, total);
    for (uint64_t i = 0; i < 4; i++) {
        printf("a[%lu] = %lu\n", i, a[i]);
    }
    const int nstream = 2;
    cudaStream_t streams[16];
    for (uint64_t i = 0; i < nstream; i++) {
        CHECKCUDAERR(cudaStreamCreate(&streams[i]));
    }
    TimerStart(MEMCPY_ASYNC);
    uint64_t pieces = 1<<10;
    uint64_t segment = total/pieces;
    for (uint64_t i = 0; i < pieces; i++) {
        cudaStream_t stream = streams[i%nstream];
        CHECKCUDAERR(cudaMemcpyAsync(b+i*segment, a+i*segment, segment * sizeof(uint64_t), cudaMemcpyHostToDevice, stream));
        add_one<<<segment/16, 16, 0, stream>>>(b+i*segment, segment);
        CHECKCUDAERR(cudaMemcpyAsync(a+i*segment, b+i*segment, segment * sizeof(uint64_t), cudaMemcpyDeviceToHost, stream));
    }
    TimerStart(MEMCPY_H_TO_D_ASYNC);

//    TimerStopAndLog(MEMCPY_H_TO_D_ASYNC);
//    TimerStart(KERNEL_ASYNC);
//
//    TimerStopAndLog(KERNEL_ASYNC);
//    TimerStart(MEMCPY_D_TO_H_ASYNC);
//
//    TimerStopAndLog(MEMCPY_D_TO_H_ASYNC);
    TimerStart(WAIT_STREAM);
    for (uint64_t i = 0; i < nstream; i++) {
        CHECKCUDAERR(cudaStreamSynchronize(streams[i]));
        CHECKCUDAERR(cudaStreamDestroy(streams[i]));
    }
    TimerStopAndLog(WAIT_STREAM);
    TimerStopAndLog(MEMCPY_ASYNC);
    for (uint64_t i = 0; i < 4; i++) {
        printf("a[%lu] = %lu\n", i, a[i]);
    }
    cudaFreeHost(a);
    cudaFree(b);


    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));

    cudaStream_t streams2d[8][16];
    uint64_t *aa;
    CHECKCUDAERR(cudaMallocHost(&aa, total * nDevices * sizeof(uint64_t)));
    init(aa, total*nDevices);
    uint64_t *bb[8];
#pragma omp parallel for num_threads(nDevices)
    for (uint64_t d = 0; d < nDevices; d++) {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaMalloc(&bb[d], total * sizeof(uint64_t)));
        for (uint64_t i = 0; i <nstream; i++) {
            CHECKCUDAERR(cudaStreamCreate(&streams2d[d][i]));
        }
    }


    TimerStart(MEMCPY_MULTIGPU_ASYNC);
#pragma omp parallel for num_threads(nDevices)
    for (uint64_t d = 0; d < nDevices; d++) {
        CHECKCUDAERR(cudaSetDevice(d));
        uint64_t *a = aa + d*total;
        uint64_t *b = bb[d];
#pragma omp parallel for num_threads(pieces)
        for (uint64_t i = 0; i < pieces; i++) {
            uint32_t stream_idx = pieces%nstream;
            cudaStream_t stream = streams2d[d][stream_idx];
            CHECKCUDAERR(cudaMemcpyAsync(b+i*segment, a+i*segment, segment * sizeof(uint64_t), cudaMemcpyHostToDevice, stream));
            add_one<<<segment/16, 16, 0, stream>>>(b+i*segment, segment);
            CHECKCUDAERR(cudaMemcpyAsync(a+i*segment, b+i*segment, segment * sizeof(uint64_t), cudaMemcpyDeviceToHost, stream));
        }
    }

    TimerStart(MULTIGPU_WAIT);
    for (uint64_t i = 0; i < nDevices; i++) {
        for (uint64_t j = 0; j <nstream; j++) {
            CHECKCUDAERR(cudaStreamSynchronize(streams2d[i][j]));
        }
    }
    TimerStopAndLog(MULTIGPU_WAIT);
    TimerStopAndLog(MEMCPY_MULTIGPU_ASYNC);
    for (uint64_t i = 0; i < nDevices; i++) {
        for (uint64_t j = 0; j <nstream; j++) {
            CHECKCUDAERR(cudaStreamDestroy(streams2d[i][j]));
        }
    }
}

int main(int argc, char **argv)
{
    ::testing::InitGoogleTest(&argc, argv);
    return RUN_ALL_TESTS();
}
