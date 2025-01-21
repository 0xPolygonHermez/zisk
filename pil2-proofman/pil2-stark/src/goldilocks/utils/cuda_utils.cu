#include "cuda_utils.hpp"
#include "cuda_utils.cuh"

#define MAX_GPUS 16

uint64_t *global_buffer;

void alloc_pinned_mem(uint64_t n)
{
    CHECKCUDAERR(cudaHostAlloc(&global_buffer, n * sizeof(uint64_t), cudaHostAllocPortable));
}

void alloc_pinned_mem_per_device(uint64_t n)
{
    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));
    CHECKCUDAERR(cudaHostAlloc(&global_buffer, n * nDevices * sizeof(uint64_t), cudaHostAllocPortable));
}

uint64_t* get_pinned_mem() {
    return global_buffer;
}

void free_pinned_mem()
{
    cudaFreeHost(global_buffer);
}

void warmup_all_gpus()
{
    uint64_t *gpu_a[MAX_GPUS];
    uint64_t size = (1 << 20);

    int nDevices = 0;
    CHECKCUDAERR(cudaGetDeviceCount(&nDevices));

#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaMalloc(&gpu_a[d], size * sizeof(uint64_t)));
    }
#pragma omp parallel for num_threads(nDevices)
    for (uint32_t d = 0; d < nDevices; d++)
    {
        CHECKCUDAERR(cudaSetDevice(d));
        CHECKCUDAERR(cudaFree(gpu_a[d]));
    }
}
