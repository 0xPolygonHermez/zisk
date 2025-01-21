#ifndef __CUDA_UTILS_CUH__
#define __CUDA_UTILS_CUH__

#include <cuda.h>
#include <stdio.h>
#include <assert.h>

__host__ inline void checkCudaError(cudaError_t code, const char *file, int line)
{
    if (code != cudaSuccess)
    {
        printf("CUDA error: %s %s %d\n", cudaGetErrorString(code), file, line);
        assert(0);
    }
}

#define CHECKCUDAERR(ans)                          \
    {                                              \
        checkCudaError((ans), __FILE__, __LINE__); \
    }

__device__ __forceinline__ void mymemcpy(uint64_t* dst, uint64_t* src, size_t n)
{
    for (uint32_t i = 0; i < n; i++)
    {
        dst[i] = src[i];
    }
}

__device__ __forceinline__ void mymemset(uint64_t* dst, uint64_t v, size_t n)
{
    for (uint32_t i = 0; i < n; i++)
    {
        dst[i] = v;
    }
}

#endif  // __CUDA_UTILS_CUH__
