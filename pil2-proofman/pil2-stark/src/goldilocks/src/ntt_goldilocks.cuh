#include "gl64_t.cuh"
#include "cuda_utils.cuh"
#include <cuda_runtime.h>
#include <sys/time.h>

#ifdef GPU_TIMING
#include "timer_gl.hpp"
#endif

__device__ __constant__ uint64_t omegas[33] = {
    1,
    18446744069414584320ULL,
    281474976710656ULL,
    16777216ULL,
    4096ULL,
    64ULL,
    8ULL,
    2198989700608ULL,
    4404853092538523347ULL,
    6434636298004421797ULL,
    4255134452441852017ULL,
    9113133275150391358ULL,
    4355325209153869931ULL,
    4308460244895131701ULL,
    7126024226993609386ULL,
    1873558160482552414ULL,
    8167150655112846419ULL,
    5718075921287398682ULL,
    3411401055030829696ULL,
    8982441859486529725ULL,
    1971462654193939361ULL,
    6553637399136210105ULL,
    8124823329697072476ULL,
    5936499541590631774ULL,
    2709866199236980323ULL,
    8877499657461974390ULL,
    3757607247483852735ULL,
    4969973714567017225ULL,
    2147253751702802259ULL,
    2530564950562219707ULL,
    1905180297017055339ULL,
    3524815499551269279ULL,
    7277203076849721926ULL,
};

__device__ __constant__ uint64_t omegas_inv[33] = {
    0x1,
    0xffffffff00000000,
    0xfffeffff00000001,
    0xfffffeff00000101,
    0xffefffff00100001,
    0xfbffffff04000001,
    0xdfffffff20000001,
    0x3fffbfffc0,
    0x7f4949dce07bf05d,
    0x4bd6bb172e15d48c,
    0x38bc97652b54c741,
    0x553a9b711648c890,
    0x55da9bb68958caa,
    0xa0a62f8f0bb8e2b6,
    0x276fd7ae450aee4b,
    0x7b687b64f5de658f,
    0x7de5776cbda187e9,
    0xd2199b156a6f3b06,
    0xd01c8acd8ea0e8c0,
    0x4f38b2439950a4cf,
    0x5987c395dd5dfdcf,
    0x46cf3d56125452b1,
    0x909c4b1a44a69ccb,
    0xc188678a32a54199,
    0xf3650f9ddfcaffa8,
    0xe8ef0e3e40a92655,
    0x7c8abec072bb46a6,
    0xe0bfc17d5c5a7a04,
    0x4c6b8a5a0b79f23a,
    0x6b4d20533ce584fe,
    0xe5cceae468a70ec2,
    0x8958579f296dac7a,
    0x16d265893b5b7e85,
};

__device__ __constant__ uint64_t domain_size_inverse[33] = {
    0x0000000000000001, // 1^{-1}
    0x7fffffff80000001, // 2^{-1}
    0xbfffffff40000001, // (1 << 2)^{-1}
    0xdfffffff20000001, // (1 << 3)^{-1}
    0xefffffff10000001,
    0xf7ffffff08000001,
    0xfbffffff04000001,
    0xfdffffff02000001,
    0xfeffffff01000001,
    0xff7fffff00800001,
    0xffbfffff00400001,
    0xffdfffff00200001,
    0xffefffff00100001,
    0xfff7ffff00080001,
    0xfffbffff00040001,
    0xfffdffff00020001,
    0xfffeffff00010001,
    0xffff7fff00008001,
    0xffffbfff00004001,
    0xffffdfff00002001,
    0xffffefff00001001,
    0xfffff7ff00000801,
    0xfffffbff00000401,
    0xfffffdff00000201,
    0xfffffeff00000101,
    0xffffff7f00000081,
    0xffffffbf00000041,
    0xffffffdf00000021,
    0xffffffef00000011,
    0xfffffff700000009,
    0xfffffffb00000005,
    0xfffffffd00000003,
    0xfffffffe00000002, // (1 << 32)^{-1}
};

// CUDA Threads Per Block
#define TPB_NTT 16
#define SHIFT 7

__global__ void br_ntt_group(gl64_t *data, gl64_t *twiddles, uint32_t i, uint32_t domain_size, uint32_t ncols)
{
      uint32_t j = blockIdx.x;
      uint32_t col = threadIdx.x;
      uint32_t start = domain_size >> 1;
      twiddles = twiddles + start;
      if (j < domain_size / 2 && col < ncols)
      {
            uint32_t half_group_size = 1 << i;
            uint32_t group = j >> i;                     // j/(group_size/2);
            uint32_t offset = j & (half_group_size - 1); // j%(half_group_size);
            uint32_t index1 = (group << i + 1) + offset;
            uint32_t index2 = index1 + half_group_size;
            gl64_t factor = twiddles[offset * (domain_size >> i + 1)];
            gl64_t odd_sub = gl64_t((uint64_t)data[index2 * ncols + col]) * factor;
            data[index2 * ncols + col] = gl64_t((uint64_t)data[index1 * ncols + col]) - odd_sub;
            data[index1 * ncols + col] = gl64_t((uint64_t)data[index1 * ncols + col]) + odd_sub;
            // DEGUG: assert(data[index2 * ncols + col] < 18446744069414584321ULL);
            // DEBUG: assert(data[index1 * ncols + col] < 18446744069414584321ULL);
      }
}

__global__ void intt_scale(gl64_t *data, gl64_t *r, uint32_t domain_size, uint32_t log_domain_size, uint32_t ncols, bool extend)
{
      uint32_t j = blockIdx.x;    // domain_size
      uint32_t col = threadIdx.x; // cols
      uint32_t index = j * ncols + col;
      gl64_t factor = gl64_t(domain_size_inverse[log_domain_size]);
      if (extend)
      {
          factor = factor * r[domain_size + j];
      }
      if (index < domain_size * ncols)
      {
          data[index] = gl64_t((uint64_t)data[index]) * factor;
          // DEBUG: assert(data[index] < 18446744069414584321ULL);
      }
}

__global__ void reverse_permutation(gl64_t *data, uint32_t log_domain_size, uint32_t ncols)
{
    uint32_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    uint32_t ibr = __brev(idx) >> (32 - log_domain_size);
    if (ibr > idx)
    {
        gl64_t tmp;
        for (uint32_t i = 0; i < ncols; i++)
        {
            tmp = data[idx * ncols + i];
            data[idx * ncols + i] = data[ibr * ncols + i];
            data[ibr * ncols + i] = tmp;
        }
    }
}

__global__ void init_twiddle_factors_small_size(gl64_t *fwd_twiddles, gl64_t *inv_twiddles, uint32_t log_domain_size)
{
    gl64_t omega = gl64_t(omegas[log_domain_size]);
    gl64_t omega_inv = gl64_t(omegas_inv[log_domain_size]);

    uint32_t start = 1 << log_domain_size - 1;

    fwd_twiddles[start] = gl64_t::one();
    inv_twiddles[start] = gl64_t::one();

    for (uint32_t i = start + 1; i < start + (1 << log_domain_size - 1); i++)
    {
        fwd_twiddles[i] = fwd_twiddles[i - 1] * omega;
        inv_twiddles[i] = inv_twiddles[i - 1] * omega_inv;
    }
}

__global__ void init_twiddle_factors_first_step(gl64_t *fwd_twiddles, gl64_t *inv_twiddles, uint32_t log_domain_size)
{
    gl64_t omega = gl64_t(omegas[log_domain_size]);
    gl64_t omega_inv = gl64_t(omegas_inv[log_domain_size]);

    uint32_t start = 1 << log_domain_size - 1;

    fwd_twiddles[start] = gl64_t::one();
    inv_twiddles[start] = gl64_t::one();

    for (uint32_t i = start + 1; i <= start + (1 << 12); i++)
    {
        fwd_twiddles[i] = fwd_twiddles[i - 1] * omega;
        inv_twiddles[i] = inv_twiddles[i - 1] * omega_inv;
    }
}

__global__ void init_twiddle_factors_second_step(gl64_t *fwd_twiddles, gl64_t *inv_twiddles, uint32_t log_domain_size)
{
    uint32_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    uint32_t start = 1 << log_domain_size - 1;
    for (uint32_t i = 1; i < 1 << log_domain_size - 13; i++)
    {
        fwd_twiddles[start + i * 4096 + idx] = fwd_twiddles[start + (i - 1) * 4096 + idx] * fwd_twiddles[start + 4096];
        inv_twiddles[start + i * 4096 + idx] = inv_twiddles[start + (i - 1) * 4096 + idx] * inv_twiddles[start + 4096];
    }
}

void init_twiddle_factors(gl64_t *fwd_twiddles, gl64_t *inv_twiddles, uint32_t log_domain_size)
{
    if (log_domain_size <= 13)
    {
        init_twiddle_factors_small_size<<<1, 1>>>(fwd_twiddles, inv_twiddles, log_domain_size);
        CHECKCUDAERR(cudaGetLastError());
    }
    else
    {
        init_twiddle_factors_first_step<<<1, 1>>>(fwd_twiddles, inv_twiddles, log_domain_size);
        CHECKCUDAERR(cudaGetLastError());
        init_twiddle_factors_second_step<<<1 << 12, 1>>>(fwd_twiddles, inv_twiddles, log_domain_size);
        CHECKCUDAERR(cudaGetLastError());
    }
}

__global__ void init_r_small_size(gl64_t *r, uint32_t log_domain_size)
{
    uint32_t start = 1 << log_domain_size;
    r[start] = gl64_t::one();
    for (uint32_t i = start + 1; i < start + (1 << log_domain_size); i++)
    {
        r[i] = r[i - 1] * gl64_t(SHIFT);
    }
}

__global__ void init_r_first_step(gl64_t *r, uint32_t log_domain_size)
{
  uint32_t start = 1 << log_domain_size;
  r[start] = gl64_t::one();
  // init first 4097 elements and then init others in parallel
  for (uint32_t i = start + 1; i <= start + (1 << 12); i++)
  {
      r[i] = r[i - 1] * gl64_t(SHIFT);
  }
}

__global__ void init_r_second_step(gl64_t *r, uint32_t log_domain_size)
{
    uint32_t start = 1 << log_domain_size;
    uint32_t idx = blockIdx.x * blockDim.x + threadIdx.x;
    for (uint32_t i = 1; i < 1 << log_domain_size - 12; i++)
    {
        r[start + i * 4096 + idx] = r[start + (i - 1) * 4096 + idx] * r[start + 4096];
    }
}

void init_r(gl64_t *r, uint32_t log_domain_size)
{
    if (log_domain_size <= 12)
    {
        init_r_small_size<<<1, 1>>>(r, log_domain_size);
        CHECKCUDAERR(cudaGetLastError());
    }
    else
    {
        init_r_first_step<<<1, 1>>>(r, log_domain_size);
        CHECKCUDAERR(cudaGetLastError());
        init_r_second_step<<<1 << 12, 1>>>(r, log_domain_size);
        CHECKCUDAERR(cudaGetLastError());
    }
}

void ntt_cuda(cudaStream_t stream, gl64_t *data, gl64_t *r, gl64_t *fwd_twiddles, gl64_t *inv_twiddles, uint32_t log_domain_size, uint32_t ncols, bool inverse, bool extend)
{

    uint32_t domain_size = 1 << log_domain_size;

    dim3 blockDim;
    dim3 gridDim;
    if (domain_size > TPB_NTT)
    {
        blockDim = dim3(TPB_NTT);
        gridDim = dim3(domain_size / TPB_NTT);
    }
    else
    {
        blockDim = dim3(domain_size);
        gridDim = dim3(1);
    }

#ifdef GPU_TIMING
    TimerStart(NTT_Core_ReversePermutation);
#endif
    reverse_permutation<<<gridDim, blockDim, 0, stream>>>(data, log_domain_size, ncols);
    CHECKCUDAERR(cudaGetLastError());
#ifdef GPU_TIMING
    cudaStreamSynchronize(stream);
    TimerStopAndLog(NTT_Core_ReversePermutation);
#endif

    gl64_t *ptr_twiddles = fwd_twiddles;
    if (inverse)
    {
        ptr_twiddles = inv_twiddles;
    }
#ifdef GPU_TIMING
    TimerStart(NTT_Core_BRNTTGroup);
#endif
    for (uint32_t i = 0; i < log_domain_size; i++)
    {
        br_ntt_group<<<domain_size / 2, ncols, 0, stream>>>(data, ptr_twiddles, i, domain_size, ncols);
        CHECKCUDAERR(cudaGetLastError());
    }
#ifdef GPU_TIMING
    cudaStreamSynchronize(stream);
    TimerStopAndLog(NTT_Core_BRNTTGroup);
#endif

    if (inverse)
    {
#ifdef GPU_TIMING
        TimerStart(NTT_Core_INTTScale);
#endif
        intt_scale<<<domain_size, ncols, 0, stream>>>(data, r, domain_size, log_domain_size, ncols, extend);
        CHECKCUDAERR(cudaGetLastError());
#ifdef GPU_TIMING
        cudaStreamSynchronize(stream);
        TimerStopAndLog(NTT_Core_INTTScale);
#endif
    }
}
