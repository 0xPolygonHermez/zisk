#include "ntt_goldilocks.hpp"
#include "cuda_utils.cuh"
#include "cuda_utils.hpp"
#include "gl64_t.cuh"
#include "poseidon_goldilocks.hpp"
#include "ntt_goldilocks.cuh"

// CUDA Threads per Block
#define TPB_V1 64

#define MAX_GPUS 16
const int nStreams = 2;
gl64_t *gpu_roots[MAX_GPUS];
gl64_t *gpu_a[MAX_GPUS];
gl64_t *gpu_a2[MAX_GPUS];
gl64_t *gpu_forward_twiddle_factors[MAX_GPUS];
gl64_t *gpu_inverse_twiddle_factors[MAX_GPUS];
gl64_t *gpu_r_[MAX_GPUS];
cudaStream_t gpu_stream[MAX_GPUS];
gl64_t *gpu_poseidon_state[MAX_GPUS];

//#ifdef GPU_TIMING
#include "timer_gl.hpp"
//#endif

__global__ void transpose(uint64_t *dst, uint64_t *src, uint32_t nblocks, uint32_t nrows, uint32_t ncols, uint32_t ncols_last_block)
{
    int i = blockIdx.x * blockDim.x + threadIdx.x; // tid
    if (i >= nrows)
        return;

    uint64_t *ldst = dst + i * ((nblocks - 1) * ncols + ncols_last_block);

    for (uint32_t k = 0; k < nblocks - 1; k++)
    {
        for (uint32_t j = 0; j < ncols; j++)
        {
            *ldst = src[k * nrows * ncols + i * ncols + j];
            ldst++;
        }
    }
    // last block
    for (uint32_t j = 0; j < ncols_last_block; j++)
    {
        *ldst = src[(nblocks - 1) * nrows * ncols + i * ncols_last_block + j];
        ldst++;
    }
}

__global__ void transpose_opt(uint64_t *dst, uint64_t *src, uint32_t nblocks, uint32_t nrows, uint32_t ncols, uint32_t ncols_last_block, uint32_t nrb)
{
    __shared__ uint64_t row[1056];

    int ncols_total = (nblocks - 1) * ncols + ncols_last_block;
    // tid is the destination column
    int tid = threadIdx.x;
    if (tid >= ncols_total)
        return;

    // bid is the destination/source row
    int bid = blockIdx.x * nrb;
    if (bid >= nrows)
        return;

    int k = tid / ncols;
    int nc = ncols;
    if (k == nblocks-1)
    {
        nc = ncols_last_block;
    }
    int j = tid % ncols;

    for (int r = bid; r < bid + nrb; r++)
    {
        uint64_t *pdst = dst + r * ncols_total + tid;
        uint64_t *psrc = src + (k * ncols * nrows) + r * nc + j;
        row[tid] = *psrc;
        __syncthreads();
        *pdst = row[tid];
        __syncthreads();
    }
}

void NTT_Goldilocks::LDE_MerkleTree_GPU(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t size, u_int64_t ext_size, u_int64_t ncols, Goldilocks::Element *buffer, u_int64_t nphase, bool buildMerkleTree)
{
    if (ncols == 0 || size == 0)
    {
        return;
    }

    printf("*** In LDE_MerkleTree_GPU ...\n");

    int gpu_id = 0;

    uint64_t aux_size = ext_size * ncols;
    CHECKCUDAERR(cudaSetDevice(gpu_id));
    CHECKCUDAERR(cudaStreamCreate(&gpu_stream[gpu_id]));
    CHECKCUDAERR(cudaMalloc(&gpu_a[gpu_id], aux_size * sizeof(uint64_t)));
    CHECKCUDAERR(cudaMalloc(&gpu_forward_twiddle_factors[gpu_id], ext_size * sizeof(uint64_t)));
    CHECKCUDAERR(cudaMalloc(&gpu_inverse_twiddle_factors[gpu_id], ext_size * sizeof(uint64_t)));
    CHECKCUDAERR(cudaMalloc(&gpu_r_[gpu_id], ext_size * sizeof(uint64_t)));

    int lg2 = log2(size);
    int lg2ext = log2(ext_size);
    init_twiddle_factors(gpu_forward_twiddle_factors[gpu_id], gpu_inverse_twiddle_factors[gpu_id], lg2);
    init_twiddle_factors(gpu_forward_twiddle_factors[gpu_id], gpu_inverse_twiddle_factors[gpu_id], lg2ext);
    init_r(gpu_r_[gpu_id], lg2);

    CHECKCUDAERR(cudaMemcpyAsync(gpu_a[gpu_id], src, size * ncols * sizeof(gl64_t), cudaMemcpyHostToDevice, gpu_stream[gpu_id]));
    CHECKCUDAERR(cudaMemsetAsync(gpu_a[gpu_id] + size * ncols, 0, size * ncols * sizeof(gl64_t), gpu_stream[gpu_id]));
    ntt_cuda(gpu_stream[gpu_id], gpu_a[gpu_id], gpu_r_[gpu_id], gpu_forward_twiddle_factors[gpu_id], gpu_inverse_twiddle_factors[gpu_id], lg2, ncols, true, true);
    ntt_cuda(gpu_stream[gpu_id], gpu_a[gpu_id], gpu_r_[gpu_id], gpu_forward_twiddle_factors[gpu_id], gpu_inverse_twiddle_factors[gpu_id], lg2ext, ncols, false, false);
    CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[gpu_id]));

    if (buildMerkleTree)
    {
        if (buffer != NULL)
        {
            CHECKCUDAERR(cudaMemcpyAsync(buffer, gpu_a[gpu_id], ext_size * ncols * sizeof(gl64_t), cudaMemcpyDeviceToHost, gpu_stream[gpu_id]));
        }
        PoseidonGoldilocks::merkletree_cuda_gpudata(dst, (uint64_t *)gpu_a[gpu_id], ncols, ext_size);
        CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[gpu_id]));
    }
    else
    {
        CHECKCUDAERR(cudaMemcpy(dst, gpu_a[gpu_id], ext_size * ncols * sizeof(gl64_t), cudaMemcpyDeviceToHost));
    }

    CHECKCUDAERR(cudaStreamDestroy(gpu_stream[gpu_id]));
    CHECKCUDAERR(cudaFree(gpu_a[gpu_id]));
    CHECKCUDAERR(cudaFree(gpu_r_[gpu_id]));
    CHECKCUDAERR(cudaFree(gpu_forward_twiddle_factors[gpu_id]));
    CHECKCUDAERR(cudaFree(gpu_inverse_twiddle_factors[gpu_id]));
}

void NTT_Goldilocks::LDE_MerkleTree_MultiGPU(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t size, u_int64_t ext_size, u_int64_t ncols, Goldilocks::Element *buffer, u_int64_t nphase, bool buildMerkleTree)
{
    if (ncols == 0 || size == 0)
    {
        return;
    }

    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));

    uint64_t free_mem, total_mem;
    cudaMemGetInfo(&free_mem, &total_mem);

    uint64_t ncols_per_gpu = (ncols % nDevices) ? ncols / nDevices + 1 : ncols / nDevices;
    uint64_t ncols_last_gpu = ncols - ncols_per_gpu * (nDevices - 1);
    uint64_t aux_ext_size = ext_size * ncols_per_gpu;

    printf("*** In LDE_MerkleTree_MultiGPU() ...\n");
    printf("Number of CPU threads: %d\n", nThreads);
    printf("Number of GPUs: %d\n", nDevices);
    printf("Number columns: %lu\n", ncols);
    printf("Cols per GPU: %lu\n", ncols_per_gpu);
    printf("Cols last GPU: %lu\n", ncols_last_gpu);
    printf("Total cuda memory: %lu MB\n", total_mem >> 20);
    printf("Free cuda memory: %lu MB\n", free_mem >> 20);

    // TODO - we suppose the GPU memory is large enough, so we do not test it

    int lg2 = log2(size);
    int lg2ext = log2(ext_size);

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_PrepareGPUs);
#endif
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaStreamCreate(gpu_stream + d));
        CHECKCUDAERR(cudaStreamCreate(gpu_stream + nDevices + d));
        CHECKCUDAERR(cudaMalloc(&gpu_a[d], aux_ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_r_[d], ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_forward_twiddle_factors[d], ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_inverse_twiddle_factors[d], ext_size * sizeof(uint64_t)));
        init_twiddle_factors(gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2);
        init_twiddle_factors(gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2ext);
        init_r(gpu_r_[d], lg2);
    }

#ifdef GPU_TIMING
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
    }
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_PrepareGPUs);
#endif

    Goldilocks::Element *aux[MAX_GPUS];
#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_SplitColsOnCPU);
#endif
    for (uint32_t d = 0; d < nDevices; d++)
    {
        uint64_t aux_ncols = (d == nDevices - 1) ? ncols_last_gpu : ncols_per_gpu;
        if (buffer != NULL)
        {
            aux[d] = buffer + d * ext_size * ncols_per_gpu;
        }
        else
        {
            aux[d] = (Goldilocks::Element *)malloc(sizeof(Goldilocks::Element) * ext_size * aux_ncols);
        }
        assert(aux[d] != NULL);
#pragma omp parallel for schedule(static)
        for (u_int64_t ie = 0; ie < size; ++ie)
        {
            u_int64_t offset2 = ie * ncols + d * ncols_per_gpu;
            std::memcpy(&(aux[d][ie * aux_ncols]), &src[offset2], aux_ncols * sizeof(Goldilocks::Element));
        }
    }
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_SplitColsOnCPU);
#endif

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_EXT);
#endif
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        uint64_t aux_ncols = (d == nDevices - 1) ? ncols_last_gpu : ncols_per_gpu;
        CHECKCUDAERR(cudaMemcpyAsync(gpu_a[d], aux[d], size * aux_ncols * sizeof(gl64_t), cudaMemcpyHostToDevice, gpu_stream[d]));
        CHECKCUDAERR(cudaMemsetAsync(gpu_a[d] + size * aux_ncols, 0, size * aux_ncols * sizeof(gl64_t), gpu_stream[d]));
        ntt_cuda(gpu_stream[d], gpu_a[d], gpu_r_[d], gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2, aux_ncols, true, true);
        ntt_cuda(gpu_stream[d], gpu_a[d], gpu_r_[d], gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2ext, aux_ncols, false, false);
        CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
    }
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_EXT);
    TimerStart(LDE_MerkleTree_MultiGPU_PartialCleanup);
#endif

#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaFree(gpu_forward_twiddle_factors[d]));
        CHECKCUDAERR(cudaFree(gpu_inverse_twiddle_factors[d]));
        CHECKCUDAERR(cudaFree(gpu_r_[d]));
        CHECKCUDAERR(cudaMalloc(&gpu_a2[d], aux_ext_size * sizeof(uint64_t)));
    }
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_PartialCleanup);
#endif

    if (buildMerkleTree)
    {
        uint64_t nrows_per_gpu = ext_size / nDevices;
        uint64_t nrows_last_gpu = nrows_per_gpu;
        if (ext_size % nDevices != 0)
        {
            nrows_last_gpu = ext_size - (nDevices - 1) * nrows_per_gpu;
        }
        printf("Rows per GPU: %lu\n", nrows_per_gpu);
        printf("Rows last GPU: %lu\n", nrows_last_gpu);
        uint64_t block_elem = nrows_per_gpu * ncols_per_gpu;
        uint64_t block_size = block_elem * sizeof(uint64_t);

#ifdef GPU_TIMING
        TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree);
        TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree_Peer2PeerCopy);
#endif
        // di is destination, dj is source
        for (uint64_t di = 0; di < nDevices; di++)
        {
            for (uint64_t dj = 0; dj < nDevices - 1; dj++)
            {
                CHECKCUDAERR(cudaMemcpyPeerAsync(gpu_a2[di] + dj * block_elem, di, gpu_a[dj] + di * block_elem, dj, block_size, gpu_stream[di]));
            }
        }
        // last block may have different size
        uint64_t block_elem_last = nrows_per_gpu * ncols_last_gpu;
        uint64_t block_size_last = block_elem_last * sizeof(uint64_t);
        for (uint64_t di = 0; di < nDevices; di++)
        {
            uint64_t dj = nDevices - 1;
            CHECKCUDAERR(cudaMemcpyPeerAsync(gpu_a2[di] + dj * block_elem, di, gpu_a[dj] + di * block_elem_last, dj, block_size_last, gpu_stream[di]));
        }
#pragma omp parallel for num_threads(nDevices)
        for (uint32_t d = 0; d < nDevices; d++)
        {
            CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
        }
#ifdef GPU_TIMING
        TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree_Peer2PeerCopy);
        TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree_Transpose);
#endif
        // re-arrange (transpose)
#pragma omp parallel for num_threads(nDevices)
        for (uint32_t d = 0; d < nDevices; d++)
        {
            CHECKCUDAERR(cudaSetDevice(d));
            // transpose<<<ceil(nrows_per_gpu / (1.0 * TPB_V1)), TPB_V1, 0, gpu_stream[d]>>>((uint64_t *)gpu_a[d], (uint64_t *)gpu_a2[d], nDevices, nrows_per_gpu, ncols_per_gpu, ncols_last_gpu);
            transpose_opt<<<128, ncols, 0, gpu_stream[d]>>>((uint64_t *)gpu_a[d], (uint64_t *)gpu_a2[d], nDevices, nrows_per_gpu, ncols_per_gpu, ncols_last_gpu, nrows_per_gpu / 128);
            CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
        }

        if (buffer != NULL)
        {
#pragma omp parallel for num_threads(nDevices)
            for (uint64_t d = 0; d < nDevices; d++)
            {
                CHECKCUDAERR(cudaSetDevice(d));
                CHECKCUDAERR(cudaMemcpyAsync(buffer + d * (nrows_per_gpu * ncols), gpu_a[d], nrows_per_gpu * ncols * sizeof(uint64_t), cudaMemcpyDeviceToHost, gpu_stream[nDevices + d]));
            }
        }

#pragma omp parallel for num_threads(nDevices)
        for (uint32_t d = 0; d < nDevices; d++)
        {
            CHECKCUDAERR(cudaSetDevice(d));
            CHECKCUDAERR(cudaFree(gpu_a2[d]));
            CHECKCUDAERR(cudaMalloc(&gpu_a2[d], 4 * nrows_per_gpu * sizeof(uint64_t)));
        }

#ifdef GPU_TIMING
        TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree_Transpose);
        TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree_Kernel);
#endif
        // Merkle tree building
        PoseidonGoldilocks::merkletree_cuda_multi_gpu_full(dst, (uint64_t **)gpu_a, (uint64_t **)gpu_a2, gpu_stream, ncols, ext_size, nrows_per_gpu, nDevices);
#ifdef GPU_TIMING
        TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree_Kernel);
        TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree);
#endif
    }
    else
    {
#ifdef GPU_TIMING
        TimerStart(LDE_MerkleTree_MultiGPU_CopyBackAfterNTT);
#endif
        for (uint32_t d = 0; d < nDevices; d++)
        {
            uint64_t aux_ncols = (d == nDevices - 1) ? ncols_last_gpu : ncols_per_gpu;
#pragma omp parallel for schedule(static)
            for (u_int64_t ie = 0; ie < ext_size; ++ie)
            {
                u_int64_t offset2 = ie * ncols + d * ncols_per_gpu;
                CHECKCUDAERR(cudaMemcpyAsync(&dst[offset2], &gpu_a[d][ie * aux_ncols], aux_ncols * sizeof(uint64_t), cudaMemcpyDeviceToHost, gpu_stream[d]));
            }
        }
#pragma omp parallel for num_threads(nDevices)
        for (uint32_t d = 0; d < nDevices; d++)
        {
            CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
        }
#ifdef GPU_TIMING
        TimerStopAndLog(LDE_MerkleTree_MultiGPU_CopyBackAfterNTT);
#endif
    }

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_Cleanup);
#endif
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaStreamDestroy(gpu_stream[d]));
        CHECKCUDAERR(cudaStreamDestroy(gpu_stream[nDevices + d]));
        CHECKCUDAERR(cudaFree(gpu_a[d]));
        CHECKCUDAERR(cudaFree(gpu_a2[d]));
        if (buffer == NULL)
        {
            free(aux[d]);
        }
    }
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_Cleanup);
#endif
}

void NTT_Goldilocks::LDE_MerkleTree_MultiGPU_viaCPU(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t size, u_int64_t ext_size, u_int64_t ncols, Goldilocks::Element *buffer, u_int64_t nphase, bool buildMerkleTree)
{
    if (ncols == 0 || size == 0)
    {
        return;
    }

    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));
    uint64_t ncols_per_gpu = (ncols % nDevices) ? ncols / nDevices + 1 : ncols / nDevices;
    uint64_t ncols_last_gpu = ncols - ncols_per_gpu * (nDevices - 1);
    uint64_t aux_ext_size = ext_size * ncols_per_gpu;

    printf("*** In LDE_MerkleTree_MultiGPU_viaCPU() ...\n");
    printf("Number of CPU threads: %d\n", nThreads);
    printf("Number of GPUs: %d\n", nDevices);
    printf("Number columns: %lu\n", ncols);
    printf("Cols per GPU: %lu\n", ncols_per_gpu);
    printf("Cols last GPU: %lu\n", ncols_last_gpu);
    // TODO - we suppose the GPU memory is large enough, so we do not test it

    int lg2 = log2(size);
    int lg2ext = log2(ext_size);

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_PrepareGPUs);
#endif
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaStreamCreate(gpu_stream + d));
        CHECKCUDAERR(cudaStreamCreate(gpu_stream + nDevices + d));
        CHECKCUDAERR(cudaMalloc(&gpu_a[d], aux_ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_r_[d], ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_forward_twiddle_factors[d], ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_inverse_twiddle_factors[d], ext_size * sizeof(uint64_t)));
        init_twiddle_factors(gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2);
        init_twiddle_factors(gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2ext);
        init_r(gpu_r_[d], lg2);
    }

#ifdef GPU_TIMING
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
    }
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_PrepareGPUs);
#endif

    Goldilocks::Element *aux[MAX_GPUS];
#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_SplitColsOnCPU);
#endif
    for (uint32_t d = 0; d < nDevices; d++)
    {
        uint64_t aux_ncols = (d == nDevices - 1) ? ncols_last_gpu : ncols_per_gpu;
        if (buffer != NULL)
        {
            aux[d] = buffer + d * ext_size * ncols_per_gpu;
        }
        else
        {
            aux[d] = (Goldilocks::Element *)malloc(sizeof(Goldilocks::Element) * ext_size * aux_ncols);
        }
        assert(aux[d] != NULL);
#pragma omp parallel for schedule(static)
        for (u_int64_t ie = 0; ie < size; ++ie)
        {
            u_int64_t offset2 = ie * ncols + d * ncols_per_gpu;
            std::memcpy(&(aux[d][ie * aux_ncols]), &src[offset2], aux_ncols * sizeof(Goldilocks::Element));
        }
    }
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_SplitColsOnCPU);
#endif

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_EXT);
#endif
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        uint64_t aux_ncols = (d == nDevices - 1) ? ncols_last_gpu : ncols_per_gpu;
        CHECKCUDAERR(cudaMemcpyAsync(gpu_a[d], aux[d], size * aux_ncols * sizeof(gl64_t), cudaMemcpyHostToDevice, gpu_stream[d]));
        CHECKCUDAERR(cudaMemsetAsync(gpu_a[d] + size * aux_ncols, 0, size * aux_ncols * sizeof(gl64_t), gpu_stream[d]));
        ntt_cuda(gpu_stream[d], gpu_a[d], gpu_r_[d], gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2, aux_ncols, true, true);
        ntt_cuda(gpu_stream[d], gpu_a[d], gpu_r_[d], gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2ext, aux_ncols, false, false);
        CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
    }
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_EXT);
    TimerStart(LDE_MerkleTree_MultiGPU_PartialCleanup);
#endif

    uint64_t* buffer2 = get_pinned_mem();
    assert(NULL != buffer2);

#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaFree(gpu_forward_twiddle_factors[d]));
        CHECKCUDAERR(cudaFree(gpu_inverse_twiddle_factors[d]));
        CHECKCUDAERR(cudaFree(gpu_r_[d]));
    }
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_PartialCleanup);
#endif

    if (buildMerkleTree)
    {
        uint64_t nrows_per_gpu = ext_size / nDevices;
        uint64_t nrows_last_gpu = nrows_per_gpu;
        if (ext_size % nDevices != 0)
        {
            nrows_last_gpu = ext_size - (nDevices - 1) * nrows_per_gpu;
        }
        printf("Rows per GPU: %lu\n", nrows_per_gpu);
        printf("Rows last GPU: %lu\n", nrows_last_gpu);

#ifdef GPU_TIMING
        TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree);
        TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree_D2H);
#endif

        // Transpose is done on CPU. First we copy data to CPU.
        assert(buffer != NULL);
#pragma omp parallel for num_threads(nDevices)
        for (uint64_t d = 0; d < nDevices; d++)
        {
            uint64_t ncols_act = (d == nDevices - 1) ? ncols_last_gpu : ncols_per_gpu;
            CHECKCUDAERR(cudaMemcpyAsync(buffer2 + d * ext_size * ncols_per_gpu, gpu_a[d], ext_size * ncols_act * sizeof(uint64_t), cudaMemcpyDeviceToHost, gpu_stream[d]));
        }
#pragma omp parallel for num_threads(nDevices)
        for (uint64_t d = 0; d < nDevices; d++)
        {
            CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
        }

#ifdef GPU_TIMING
        TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree_D2H);
        TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree_TransposeOnCPU);
#endif

#pragma omp parallel for
        for (uint64_t row = 0; row < ext_size; row++)
        {
            uint64_t* dst = (uint64_t*)buffer + row * ncols;
            for (uint64_t d = 0; d < nDevices - 1; d++)
            {
                uint64_t* src = buffer2 + d * ext_size * ncols_per_gpu + row * ncols_per_gpu;
                memcpy(dst + d * ncols_per_gpu, src, ncols_per_gpu * sizeof(uint64_t));
            }
            // last block
            uint64_t d = nDevices - 1;
            uint64_t* src = buffer2 + d * ext_size * ncols_per_gpu + row * ncols_last_gpu;
            memcpy(dst + d * ncols_per_gpu, src, ncols_last_gpu * sizeof(uint64_t));
        }

#ifdef GPU_TIMING
        TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree_TransposeOnCPU);
        TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree_H2D);
#endif

#pragma omp parallel for num_threads(nDevices)
        for (uint64_t d = 0; d < nDevices; d++)
        {
            CHECKCUDAERR(cudaMemcpyAsync(gpu_a[d], buffer + d * nrows_per_gpu * ncols, nrows_per_gpu * ncols * sizeof(uint64_t), cudaMemcpyHostToDevice, gpu_stream[d]));
        }

#ifdef GPU_TIMING
        TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree_H2D);
        TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree_Kernel);
#endif

#pragma omp parallel for num_threads(nDevices)
        for (uint32_t d = 0; d < nDevices; d++)
        {
            CHECKCUDAERR(cudaSetDevice(d));
            CHECKCUDAERR(cudaMalloc(&gpu_a2[d], 4 * nrows_per_gpu * sizeof(uint64_t)));
        }

        // Merkle tree building
        PoseidonGoldilocks::merkletree_cuda_multi_gpu_full(dst, (uint64_t **)gpu_a, (uint64_t **)gpu_a2, gpu_stream, ncols, ext_size, nrows_per_gpu, nDevices);
#ifdef GPU_TIMING
        TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree_Kernel);
        TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree);
#endif
    }
    else
    {
#ifdef GPU_TIMING
        TimerStart(LDE_MerkleTree_MultiGPU_CopyBackAfterNTT);
#endif
        for (uint32_t d = 0; d < nDevices; d++)
        {
            uint64_t aux_ncols = (d == nDevices - 1) ? ncols_last_gpu : ncols_per_gpu;
#pragma omp parallel for schedule(static)
            for (u_int64_t ie = 0; ie < ext_size; ++ie)
            {
                u_int64_t offset2 = ie * ncols + d * ncols_per_gpu;
                // std::memcpy(&dst[offset2], &(aux[d][ie * aux_ncols]), aux_ncols * sizeof(Goldilocks::Element));
                CHECKCUDAERR(cudaMemcpyAsync(&dst[offset2], &gpu_a[d][ie * aux_ncols], aux_ncols * sizeof(uint64_t), cudaMemcpyDeviceToHost, gpu_stream[d]));
            }
        }
#pragma omp parallel for num_threads(nDevices)
        for (uint32_t d = 0; d < nDevices; d++)
        {
            CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
        }
#ifdef GPU_TIMING
        TimerStopAndLog(LDE_MerkleTree_MultiGPU_CopyBackAfterNTT);
#endif
    }

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_Cleanup);
#endif
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaStreamDestroy(gpu_stream[d]));
        CHECKCUDAERR(cudaStreamDestroy(gpu_stream[nDevices + d]));
        CHECKCUDAERR(cudaFree(gpu_a[d]));
        CHECKCUDAERR(cudaFree(gpu_a2[d]));
        if (buffer == NULL)
        {
            free(aux[d]);
        }
    }
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_Cleanup);
#endif
}

void NTT_Goldilocks::Prepare_MultiGPU(u_int64_t size, u_int64_t ext_size, u_int64_t ncols, uint64_t steps) {

    int lg2 = log2(size);
    int lg2ext = log2(ext_size);

    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));
    int pieces = steps*nDevices;
    uint64_t ncols_per_gpu = (ncols + pieces - 1) / pieces;
    uint64_t aux_ext_size = ext_size * ncols_per_gpu;

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_PrepareGPUs);
#endif
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaStreamCreate(gpu_stream + d));
        CHECKCUDAERR(cudaStreamCreate(gpu_stream + nDevices + d));
        CHECKCUDAERR(cudaMalloc(&gpu_a[d], aux_ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_r_[d], ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_forward_twiddle_factors[d], ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_inverse_twiddle_factors[d], ext_size * sizeof(uint64_t)));
        init_twiddle_factors(gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2);
        init_twiddle_factors(gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2ext);
        init_r(gpu_r_[d], lg2);
    }

#ifdef GPU_TIMING
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
    }
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_PrepareGPUs);
#endif
}

void NTT_Goldilocks::LDE_MerkleTree_Switch_MultiGPU(u_int64_t nrows_per_gpu) {

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_Switch_MultiGPU);
#endif

    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));

#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaFree(gpu_forward_twiddle_factors[d]));
        CHECKCUDAERR(cudaFree(gpu_inverse_twiddle_factors[d]));
        CHECKCUDAERR(cudaFree(gpu_r_[d]));
        CHECKCUDAERR(cudaMalloc(&gpu_a2[d], 4 * nrows_per_gpu * sizeof(uint64_t)));
    }

#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_Switch_MultiGPU);
#endif
}

void NTT_Goldilocks::LDE_MultiGPU(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t size, u_int64_t ext_size, u_int64_t ncols, u_int64_t ncols_per_gpu, u_int64_t ncols_last_gpu, Goldilocks::Element *buffer) {
    if (ncols == 0 || size == 0)
    {
        return;
    }

    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));

//    printf("*** In LDE__MultiGPU() ...\n");
//    printf("Number of CPU threads: %d\n", nThreads);
//    printf("Number of GPUs: %d\n", nDevices);
//    printf("Number columns: %lu\n", ncols);
//    printf("Cols per GPU: %lu\n", ncols_per_gpu);
//    printf("Cols last GPU: %lu\n", ncols_last_gpu);
    // TODO - we suppose the GPU memory is large enough, so we do not test it

    uint64_t *buffer_ = buffer == NULL?get_pinned_mem():(uint64_t *)buffer;

    Goldilocks::Element *aux[MAX_GPUS];
#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_SplitColsOnCPU);
#endif
    for (uint32_t d = 0; d < nDevices; d++)
    {
        uint64_t aux_ncols = (d == nDevices - 1) ? ncols_last_gpu : ncols_per_gpu;
        aux[d] = (Goldilocks::Element *)(buffer_ + d * size * ncols_per_gpu);
        assert(aux[d] != NULL);
#pragma omp parallel for schedule(static)
        for (u_int64_t ie = 0; ie < size; ++ie)
        {
            u_int64_t offset2 = ie * ncols + d * ncols_per_gpu;
            std::memcpy(&(aux[d][ie * aux_ncols]), &src[offset2], aux_ncols * sizeof(Goldilocks::Element));
        }
    }
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_SplitColsOnCPU);
#endif

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_EXT);
#endif
    int lg2 = log2(size);
    int lg2ext = log2(ext_size);
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        uint64_t aux_ncols = (d == nDevices - 1) ? ncols_last_gpu : ncols_per_gpu;
        CHECKCUDAERR(cudaMemcpyAsync(gpu_a[d], aux[d], size * aux_ncols * sizeof(gl64_t), cudaMemcpyHostToDevice, gpu_stream[d]));
        CHECKCUDAERR(cudaMemsetAsync(gpu_a[d] + size * aux_ncols, 0, size * aux_ncols * sizeof(gl64_t), gpu_stream[d]));
        ntt_cuda(gpu_stream[d], gpu_a[d], gpu_r_[d], gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2, aux_ncols, true, true);
        ntt_cuda(gpu_stream[d], gpu_a[d], gpu_r_[d], gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2ext, aux_ncols, false, false);
        CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
    }
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_EXT);
#endif


#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree_D2H);
#endif

    assert(NULL != buffer_);
    // Transpose is done on CPU. First we copy data to CPU.
#pragma omp parallel for num_threads(nDevices)
    for (uint64_t d = 0; d < nDevices; d++)
    {
        uint64_t ncols_act = (d == nDevices - 1) ? ncols_last_gpu : ncols_per_gpu;
        CHECKCUDAERR(cudaMemcpyAsync(buffer_ + d * ext_size * ncols_per_gpu, gpu_a[d], ext_size * ncols_act * sizeof(uint64_t), cudaMemcpyDeviceToHost, gpu_stream[d]));
    }
#pragma omp parallel for num_threads(nDevices)
    for (uint64_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
    }

#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree_D2H);
    TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree_TransposeOnCPU);
#endif

#pragma omp parallel for
    for (uint64_t row = 0; row < ext_size; row++)
    {
        uint64_t* dst_ = (uint64_t*)dst + row * ncols;
        for (uint64_t d = 0; d < nDevices; d++)
        {
            uint64_t gpu_size = d == nDevices - 1 ? ncols_last_gpu : ncols_per_gpu;
            uint64_t* src = buffer_ + d * ext_size * ncols_per_gpu + row * gpu_size;
            memcpy(dst_ + d * ncols_per_gpu, src, gpu_size * sizeof(uint64_t));
        }
    }

#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree_TransposeOnCPU);
#endif
}

void NTT_Goldilocks::MerkleTree_MultiGPU(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t ncols, u_int64_t nrows_per_gpu, Goldilocks::Element *buffer) {

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree);
    TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree_H2D);
#endif

    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));

#pragma omp parallel for num_threads(nDevices)
    for (uint64_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaMemcpyAsync(gpu_a[d], buffer + d * nrows_per_gpu * ncols, nrows_per_gpu * ncols * sizeof(uint64_t), cudaMemcpyHostToDevice, gpu_stream[d]));
    }

#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree_H2D);
    TimerStart(LDE_MerkleTree_MultiGPU_MerkleTree_Kernel);
#endif

    // Merkle tree building
    PoseidonGoldilocks::merkletree_cuda_multi_gpu_steps((uint64_t **)gpu_a, (uint64_t **)gpu_a2, gpu_stream, ncols, nrows_per_gpu, nDevices);
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree_Kernel);
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_MerkleTree);
#endif
}

void NTT_Goldilocks::LDE_MerkleTree_Cleanup_MultiGPU() {
#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_Cleanup_MultiGPU);
#endif

    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaStreamDestroy(gpu_stream[d]));
        CHECKCUDAERR(cudaStreamDestroy(gpu_stream[nDevices + d]));
        CHECKCUDAERR(cudaFree(gpu_a[d]));
        CHECKCUDAERR(cudaFree(gpu_a2[d]));
    }
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_Cleanup_MultiGPU);
#endif
}

void NTT_Goldilocks::LDE_MerkleTree_MultiGPU_Steps(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t size, u_int64_t ext_size, u_int64_t ncols, Goldilocks::Element *buffer, u_int64_t steps) {
    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));

    assert(steps>0);
    assert((nDevices&(nDevices-1)) == 0);

    int pieces = steps*nDevices;
    uint64_t ncols_per_gpu = (ncols + pieces - 1)/pieces;
    uint64_t ncols_last_gpu = ncols - ncols_per_gpu * (pieces - 1);
//    uint64_t nrows_per_gpu = (ext_size/8 + pieces - 1)/pieces * 8;
//    uint64_t nrows_last_gpu = ext_size - nrows_per_gpu * (pieces - 1);
    uint64_t group = nDevices > 8 ? nDevices : 8;
    uint64_t nrows_per_step = (ext_size/group + steps - 1) / steps * group;
    uint64_t nrows_last_step = ext_size - nrows_per_step * (steps - 1);

    printf("*** In LDE__MultiGPU() ...\n");
    printf("Number of CPU threads: %d\n", nThreads);
    printf("Number of GPUs: %d\n", nDevices);
    printf("Number columns: %lu\n", ncols);
    printf("Cols per GPU: %lu\n", ncols_per_gpu);
    printf("Cols last GPU: %lu\n", ncols_last_gpu);
    printf("Rows per step: %lu\n", nrows_per_step);
    printf("Rows last step: %lu\n", nrows_last_step);
    printf("Steps: %lu\n", steps);

    Prepare_MultiGPU(size, ext_size, ncols, steps);
    for (uint64_t i = 0; i < steps; i++) {
        uint64_t ncols_last = i == steps - 1? ncols_last_gpu : ncols_per_gpu;
        LDE_MultiGPU(buffer + ncols_per_gpu * nDevices * i, src + ncols_per_gpu * nDevices * i, size, ext_size, ncols, ncols_per_gpu, ncols_last, NULL);
    }

    LDE_MerkleTree_Switch_MultiGPU(nrows_per_step/nDevices);
    uint64_t *final_tree = get_pinned_mem();
    for (uint64_t i = 0; i < steps; i++) {
        uint64_t nrows_step = i == steps - 1 ? nrows_last_step : nrows_per_step;
        uint64_t nrows_gpu = nrows_step / nDevices;
        MerkleTree_MultiGPU(dst + nrows_per_step*HASH_SIZE * i, NULL, ncols, nrows_gpu, buffer + ncols * nrows_per_step * i);
#pragma omp parallel for num_threads(nDevices)
        for (uint64_t d = 0; d < nDevices; d++) {
            CHECKCUDAERR(cudaSetDevice(d));
            CHECKCUDAERR(cudaMemcpy(final_tree + (nrows_per_step * i + nrows_gpu * d)*HASH_SIZE, gpu_a2[d], nrows_gpu * HASH_SIZE * sizeof(gl64_t), cudaMemcpyDeviceToHost));
        }
    }
    PoseidonGoldilocks::merkletree_cuda_multi_gpu_final(dst, final_tree, gpu_stream, ext_size);
    LDE_MerkleTree_Cleanup_MultiGPU();
    return;
}

void NTT_Goldilocks::LDE_MerkleTree_Auto(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t size, u_int64_t ext_size, u_int64_t ncols, Goldilocks::Element *buffer, u_int64_t steps) {
    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));
    if (ncols <= nDevices * RATE) {
        return LDE_MerkleTree_GPU(dst, src, size, ext_size, ncols, buffer);
    }
    uint64_t free_mem, total_mem;
    cudaMemGetInfo(&free_mem, &total_mem);
    if (steps == 0) {
        steps = 1;
        // ntt parameters ===> 3*ext_size
        // data per gpu in each step: (ncols + nDevices * steps - 1) / (nDevices * steps) * ext_size
        while ((3 * ext_size + (ncols + nDevices * steps - 1) / (nDevices * steps) * ext_size) * sizeof(uint64_t) > total_mem) {
            steps++;
        }
    }
    return LDE_MerkleTree_MultiGPU_Steps(dst, src, size, ext_size, ncols, buffer, steps);
}

void NTT_Goldilocks::LDE_MerkleTree_MultiGPU_Init(u_int64_t size, u_int64_t ext_size, u_int64_t ncols)
{
    int lg2 = log2(size);
    int lg2ext = log2(ext_size);

    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));

    uint64_t ncols_per_gpu = (ncols % nDevices) ? ncols / nDevices + 1 : ncols / nDevices;
    uint64_t aux_ext_size = ext_size * ncols_per_gpu;

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_PrepareGPUs);
#endif
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaStreamCreate(gpu_stream + d));
        CHECKCUDAERR(cudaStreamCreate(gpu_stream + nDevices + d));
        CHECKCUDAERR(cudaMalloc(&gpu_a[d], aux_ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_a2[d], aux_ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_r_[d], ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_forward_twiddle_factors[d], ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_inverse_twiddle_factors[d], ext_size * sizeof(uint64_t)));
        init_twiddle_factors(gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2);
        init_twiddle_factors(gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2ext);
        init_r(gpu_r_[d], lg2);
    }

#ifdef GPU_TIMING
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
    }
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_PrepareGPUs);
#endif
}

void NTT_Goldilocks::LDE_MerkleTree_MultiGPU_Free()
{
    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_Cleanup);
#endif
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaStreamDestroy(gpu_stream[d]));
        CHECKCUDAERR(cudaStreamDestroy(gpu_stream[nDevices + d]));
        CHECKCUDAERR(cudaFree(gpu_a[d]));
        CHECKCUDAERR(cudaFree(gpu_a2[d]));
        CHECKCUDAERR(cudaFree(gpu_forward_twiddle_factors[d]));
        CHECKCUDAERR(cudaFree(gpu_inverse_twiddle_factors[d]));
        CHECKCUDAERR(cudaFree(gpu_r_[d]));
    }
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_Cleanup);
#endif
}

void NTT_Goldilocks::LDE_MultiGPU_Full(Goldilocks::Element *dst, Goldilocks::Element *src, u_int64_t size, u_int64_t ext_size, u_int64_t ncols, Goldilocks::Element *buffer, u_int64_t steps) {
    int lg2 = log2(size);
    int lg2ext = log2(ext_size);

    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));

    int pieces = steps*nDevices;
    uint64_t ncols_per_gpu = (ncols + pieces - 1)/pieces;
    uint64_t ncols_last_gpu = ncols - ncols_per_gpu * (pieces - 1);

    printf("*** In LDE_MultiGPU_Full() ...\n");
    printf("Number of CPU threads: %d\n", nThreads);
    printf("Number of GPUs: %d\n", nDevices);
    printf("Number columns: %lu\n", ncols);
    printf("Cols per GPU: %lu\n", ncols_per_gpu);
    printf("Cols last GPU: %lu\n", ncols_last_gpu);
    printf("Steps: %lu\n", steps);

    uint64_t aux_ext_size = ext_size * ncols_per_gpu;

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_PrepareGPUs);
#endif
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaStreamCreate(gpu_stream + d));
        CHECKCUDAERR(cudaStreamCreate(gpu_stream + nDevices + d));
        CHECKCUDAERR(cudaMalloc(&gpu_a[d], aux_ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_r_[d], ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_forward_twiddle_factors[d], ext_size * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&gpu_inverse_twiddle_factors[d], ext_size * sizeof(uint64_t)));
        init_twiddle_factors(gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2);
        init_twiddle_factors(gpu_forward_twiddle_factors[d], gpu_inverse_twiddle_factors[d], lg2ext);
        init_r(gpu_r_[d], lg2);
    }

#ifdef GPU_TIMING
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaStreamSynchronize(gpu_stream[d]));
    }
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_PrepareGPUs);
#endif


    for (uint64_t i = 0; i < steps; i++) {
        uint64_t ncols_last = i == steps - 1? ncols_last_gpu : ncols_per_gpu;
        LDE_MultiGPU(dst + ncols_per_gpu * nDevices * i, src + ncols_per_gpu * nDevices * i, size, ext_size, ncols, ncols_per_gpu, ncols_last, buffer);
    }

#ifdef GPU_TIMING
    TimerStart(LDE_MerkleTree_MultiGPU_Cleanup);
#endif
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaStreamDestroy(gpu_stream[d]));
        CHECKCUDAERR(cudaStreamDestroy(gpu_stream[nDevices + d]));
        CHECKCUDAERR(cudaFree(gpu_a[d]));
        CHECKCUDAERR(cudaFree(gpu_forward_twiddle_factors[d]));
        CHECKCUDAERR(cudaFree(gpu_inverse_twiddle_factors[d]));
        CHECKCUDAERR(cudaFree(gpu_r_[d]));
    }
#ifdef GPU_TIMING
    TimerStopAndLog(LDE_MerkleTree_MultiGPU_Cleanup);
#endif
}

void NTT_Goldilocks::extendPol_GPU(Goldilocks::Element *output, Goldilocks::Element *input, uint64_t N_Extended, uint64_t N, uint64_t ncols) {

    int lg2 = log2(N);
    int lg2ext = log2(N_Extended);

    printf("into extendPol_GPU...\n");
    printf("lg2:%d, lg2ext:%d, ncols:%lu\n", lg2, lg2ext, ncols);

    cudaStream_t stream;
    gl64_t *forward_tf_d;
    gl64_t *inverse_tf_d;
    gl64_t *r_d;
    gl64_t *data_d;

    TimerStart(ExtendPol_Full);

    CHECKCUDAERR(cudaSetDevice(0));
    CHECKCUDAERR(cudaStreamCreate(&stream))
    CHECKCUDAERR(cudaMalloc(&forward_tf_d, N_Extended * sizeof(uint64_t)));
    CHECKCUDAERR(cudaMalloc(&inverse_tf_d, N_Extended * sizeof(uint64_t)));
    CHECKCUDAERR(cudaMalloc(&r_d, N_Extended * sizeof(uint64_t)));
    CHECKCUDAERR(cudaMalloc(&data_d,  N_Extended * ncols * sizeof(uint64_t)));
    init_twiddle_factors(forward_tf_d, inverse_tf_d, lg2);
    init_twiddle_factors(forward_tf_d, inverse_tf_d, lg2ext);
    init_r(r_d, lg2);

    CHECKCUDAERR(cudaMemcpyAsync(data_d, input, N*ncols * sizeof(gl64_t), cudaMemcpyHostToDevice, stream));
    CHECKCUDAERR(cudaMemsetAsync(data_d + N*ncols, 0, N*ncols * sizeof(gl64_t), stream));
    ntt_cuda(stream, data_d, r_d, forward_tf_d, inverse_tf_d, lg2, ncols, true, true);
    ntt_cuda(stream, data_d, r_d, forward_tf_d, inverse_tf_d, lg2ext, ncols, false, false);
    CHECKCUDAERR(cudaMemcpyAsync(output, data_d, N_Extended*ncols * sizeof(gl64_t), cudaMemcpyDeviceToHost, stream));
    CHECKCUDAERR(cudaStreamSynchronize(stream));

    cudaStreamDestroy(stream);
    cudaFree(forward_tf_d);
    cudaFree(inverse_tf_d);
    cudaFree(r_d);
    cudaFree(data_d);
    TimerStopAndLog(ExtendPol_Full);
}

void NTT_Goldilocks::extendPol_MultiGPU(Goldilocks::Element *output, Goldilocks::Element *input, uint64_t N_Extended, uint64_t N, uint64_t ncols, Goldilocks::Element *buffer, uint64_t PACK) {
    printf("*** In extendPol_MultiGPU() ...\n");
    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));
    printf("nDevices: %d\n", nDevices);

    int lg2 = log2(N);
    int lg2ext = log2(N_Extended);

    printf("lg2: %d\n", lg2);
    printf("lg2ext: %d\n", lg2ext);
    printf("ncols:%lu\n", ncols);
    printf("PACK:%lu\n", PACK);

    if (buffer == NULL) {
        buffer = (Goldilocks::Element *)get_pinned_mem();
    }
    assert(buffer != NULL);

    gl64_t *forward_tf_d[MAX_GPUS];
    gl64_t *inverse_tf_d[MAX_GPUS];
    gl64_t *r_d[MAX_GPUS];
    gl64_t *data_d[nStreams*MAX_GPUS];
    cudaStream_t cuda_streams[nStreams*MAX_GPUS];
    cudaEvent_t events[nStreams*MAX_GPUS];

#pragma omp parallel for
    for (int d=0;d<nDevices;d++) {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaMalloc(&forward_tf_d[d], N_Extended * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&inverse_tf_d[d], N_Extended * sizeof(uint64_t)));
        CHECKCUDAERR(cudaMalloc(&r_d[d], N_Extended * sizeof(uint64_t)));
        init_twiddle_factors(forward_tf_d[d], inverse_tf_d[d], lg2);
        init_twiddle_factors(forward_tf_d[d], inverse_tf_d[d], lg2ext);
        init_r(r_d[d], lg2);
    }

    for (int d=0;d<nDevices;d++) {
        CHECKCUDAERR(cudaSetDevice(d));
        for (int s=0;s<nStreams;s++) {
            int idx = d+s*nDevices;
            CHECKCUDAERR(cudaMalloc(&data_d[idx],  N_Extended * PACK * sizeof(uint64_t)));
            CHECKCUDAERR(cudaStreamCreate(cuda_streams + idx));
            CHECKCUDAERR(cudaEventCreate(events + idx));
        }
    }

    Goldilocks::Element *buffer_[nStreams*nDevices];

    for (int d=0;d<nDevices;d++) {
        for (int s=0;s<nStreams;s++) {
            uint64_t idx = d+s*nDevices;
            buffer_[idx] = buffer + idx*N_Extended*PACK;
        }
    }

    uint32_t pack_count = (ncols + PACK - 1)/PACK;
    uint32_t last_pack = ncols - PACK * (pack_count - 1);

    uint32_t delay = pack_count<nDevices*nStreams?pack_count:nDevices*nStreams;

    printf("pack_count:%u, delay:%u\n", pack_count, delay);

    for (int i=0;i<pack_count+delay;i++) {
        if (i >= delay) {
            uint32_t idx = (i-delay)%(nDevices*nStreams);
            uint32_t c = (i-delay)*PACK;
            uint32_t nPack = i-delay == pack_count-1?last_pack:PACK;
            CHECKCUDAERR(cudaEventSynchronize(events[idx]));
#pragma omp parallel for
            for (uint64_t j=0;j<N_Extended;j++) {
                memcpy(output + j*ncols+c, buffer_[idx] + j*nPack, nPack * sizeof(uint64_t));
            }
        }

        if (i<pack_count) {
            uint32_t idx = i%(nDevices*nStreams);
            uint32_t d = i%nDevices;
            uint32_t c = i*PACK;
            uint32_t nPack = i == pack_count-1?last_pack:PACK;

            CHECKCUDAERR(cudaSetDevice(d));
#pragma omp parallel for
            for (uint64_t j=0;j<N;j++) {
                memcpy(buffer_[idx] + j*nPack, input + j*ncols + c, nPack * sizeof(uint64_t));
            }

            cudaStream_t stream = cuda_streams[idx];
            CHECKCUDAERR(cudaMemcpyAsync(data_d[idx], buffer_[idx], N*nPack * sizeof(gl64_t), cudaMemcpyHostToDevice, stream));
            CHECKCUDAERR(cudaMemsetAsync(data_d[idx] + N*nPack, 0, (N_Extended-N)*nPack * sizeof(gl64_t), stream));
            ntt_cuda(stream, data_d[idx], r_d[d], forward_tf_d[d], inverse_tf_d[d], lg2, nPack, true, true);
            ntt_cuda(stream, data_d[idx], r_d[d], forward_tf_d[d], inverse_tf_d[d], lg2ext, nPack, false, false);
            CHECKCUDAERR(cudaMemcpyAsync(buffer_[idx], data_d[idx], N_Extended*nPack * sizeof(gl64_t), cudaMemcpyDeviceToHost, stream));
            CHECKCUDAERR(cudaEventRecord(events[idx], stream));
        }
    }

    for (int s=0;s<nStreams*nDevices;s++) {
        cudaFree(data_d[s]);
        cudaEventDestroy(events[s]);
        cudaStreamDestroy(cuda_streams[s]);
    }
    for (int d=0;d<nDevices;d++) {
        cudaFree(forward_tf_d[d]);
        cudaFree(inverse_tf_d[d]);
        cudaFree(r_d[d]);
    }
}
