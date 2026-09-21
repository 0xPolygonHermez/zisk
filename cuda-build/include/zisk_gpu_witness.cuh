// Shared C ABI for zisk GPU witness kernels.
//
// A kernel crate supplies only the two things that are actually its own — how
// many u64 its packed trace occupies, and how to launch it — and this header
// generates the four entry points the prover and the test harness expect:
//
//   <prefix>_gpu_available()                   -> int
//   <prefix>_gpu_out_words(num_ops)            -> uint64_t
//   <prefix>_gpu_fill(d_ops, n, d_dst, dev, stream)  -> int  [registry ABI]
//   <prefix>_gpu_fill_probe(ops, n, out_host)  -> int
//
// `zisk_cuda_build` puts this directory on the include path, so a kernel just
// does `#include <zisk_gpu_witness.cuh>`.
//
// The contract `<prefix>_gpu_fill` must honour, and the reason it is generated
// here rather than written per crate: it allocates nothing, synchronises with
// nothing, creates no stream, and adds exactly one kernel launch to the
// caller's stream — which is what makes it legal inside a CUDA graph capture
// region. Every kernel getting that wrong independently is the failure mode
// this header exists to prevent.

#pragma once

#include <cstdint>
#include <cstdio>
#include <cuda_runtime.h>

/// Bail to `cleanup` with `rc = -1` on a failed CUDA call. Requires an `rc`
/// variable and a `cleanup:` label in scope.
#define ZISK_CUDA_TRY(call)                                                              \
    do {                                                                                 \
        const cudaError_t _e = (call);                                                   \
        if (_e != cudaSuccess) {                                                         \
            fprintf(stderr, "[zisk-gpu] %s:%d %s -> %s\n", __FILE__, __LINE__, #call,    \
                    cudaGetErrorString(_e));                                             \
            rc = -1;                                                                     \
            goto cleanup;                                                                \
        }                                                                                \
    } while (0)

/// Generate the witness-kernel entry points for `prefix`.
///
/// * `OpT`           — the per-operation input struct, laid out to match Rust.
/// * `DEFAULT_BLOCK` — launch width used when the caller passes 0.
/// * `MAX_BLOCK`     — the kernel's `__launch_bounds__`.
/// * `OUT_WORDS_FN`  — `uint64_t f(uint32_t num_ops)`.
/// * `LAUNCH_FN`     — `int f(const OpT*, uint32_t, uint64_t*, int block, cudaStream_t)`,
///                     which computes its own grid and launches. It must not
///                     allocate or synchronise.
///
/// Block widths are constrained to whole warps: kernels here pair operations
/// across neighbouring lanes with `__shfl_*_sync` under a full mask, where a
/// partial warp is undefined behaviour.
#define ZISK_GPU_WITNESS_ENTRIES(prefix, OpT, DEFAULT_BLOCK, MAX_BLOCK, OUT_WORDS_FN, LAUNCH_FN) \
                                                                                                 \
    extern "C" int prefix##_gpu_available() {                                                    \
        int count = 0;                                                                           \
        return cudaGetDeviceCount(&count) == cudaSuccess && count > 0 ? 1 : 0;                   \
    }                                                                                            \
                                                                                                 \
    extern "C" uint64_t prefix##_gpu_out_words(uint32_t num_ops) {                               \
        return OUT_WORDS_FN(num_ops);                                                            \
    }                                                                                            \
                                                                                                 \
    /* Enqueue-only; see the contract note at the top of this header.                            \
     * The signature is fixed by the prover's registry (GpuWitnessFillFn in                       \
     * gpu_witness.hpp): `const void*` inputs, a 64-bit count, and NO launch width --             \
     * the prover has no business choosing one, so DEFAULT_BLOCK is used and checked              \
     * at compile time. Any drift here is a function-pointer type mismatch that no                \
     * compiler can see across the FFI, so the two must be edited together. */                    \
    static_assert((DEFAULT_BLOCK) % 32 == 0 && (DEFAULT_BLOCK) > 0 &&                             \
                      (DEFAULT_BLOCK) <= (MAX_BLOCK),                                             \
                  #prefix ": DEFAULT_BLOCK must be whole warps and within MAX_BLOCK");            \
    extern "C" int prefix##_gpu_fill(const void* d_ops, uint64_t num_ops, uint64_t* d_dst,       \
                                     int device_id, void* stream) {                              \
        if (d_ops == nullptr || d_dst == nullptr || num_ops == 0) return -1;                     \
        /* The launch geometry below is 32-bit; a count that large is a caller bug. */           \
        if (num_ops > 0xffffffffull) {                                                            \
            fprintf(stderr, "[zisk-gpu] %s: %llu operations exceeds the 32-bit launch grid\n",    \
                    #prefix, (unsigned long long)num_ops);                                        \
            return -1;                                                                           \
        }                                                                                        \
        /* The caller already selected this device; setting it again is free and is what */      \
        /* keeps the launch on the right GPU should a second CUDA runtime ever end up in */      \
        /* the process, since each carries its own thread-local current device. */               \
        if (device_id >= 0 && cudaSetDevice(device_id) != cudaSuccess) return -2;                \
        const int lrc = LAUNCH_FN((const OpT*)d_ops, (uint32_t)num_ops, d_dst, (DEFAULT_BLOCK),  \
                                  (cudaStream_t)stream);                                         \
        if (lrc != 0) return lrc;                                                                \
        return cudaGetLastError() == cudaSuccess ? 0 : -3;                                       \
    }                                                                                            \
                                                                                                 \
    /* Test-only: drives the entry above against memory it owns, so the correctness test */      \
    /* covers the path the prover takes rather than a separate benchmark driver. */              \
    extern "C" int prefix##_gpu_fill_probe(const OpT* ops, uint32_t num_ops,                     \
                                           uint64_t* out_host) {                                 \
        if (ops == nullptr || out_host == nullptr || num_ops == 0) return -1;                    \
        const size_t in_bytes = (size_t)num_ops * sizeof(OpT);                                   \
        const size_t out_bytes = (size_t)OUT_WORDS_FN(num_ops) * sizeof(uint64_t);               \
        int rc = 0;                                                                              \
        int device = -1;                                                                         \
        cudaStream_t stream = nullptr;                                                           \
        OpT* d_ops = nullptr;                                                                    \
        uint64_t* d_out = nullptr;                                                               \
        ZISK_CUDA_TRY(cudaGetDevice(&device));                                                   \
        ZISK_CUDA_TRY(cudaStreamCreate(&stream));                                                \
        ZISK_CUDA_TRY(cudaMalloc(&d_ops, in_bytes));                                             \
        ZISK_CUDA_TRY(cudaMalloc(&d_out, out_bytes));                                            \
        /* Poison, so a word the kernel never writes fails the comparison instead of */          \
        /* passing as a plausible zero. */                                                       \
        ZISK_CUDA_TRY(cudaMemsetAsync(d_out, 0xAA, out_bytes, stream));                          \
        ZISK_CUDA_TRY(cudaMemcpyAsync(d_ops, ops, in_bytes, cudaMemcpyHostToDevice, stream));    \
        rc = prefix##_gpu_fill((const void*)d_ops, (uint64_t)num_ops, d_out, device, stream);                        \
        if (rc != 0) goto cleanup;                                                               \
        ZISK_CUDA_TRY(cudaMemcpyAsync(out_host, d_out, out_bytes, cudaMemcpyDeviceToHost,        \
                                      stream));                                                  \
        ZISK_CUDA_TRY(cudaStreamSynchronize(stream));                                            \
    cleanup:                                                                                     \
        if (d_out != nullptr) cudaFree(d_out);                                                   \
        if (d_ops != nullptr) cudaFree(d_ops);                                                   \
        if (stream != nullptr) cudaStreamDestroy(stream);                                        \
        return rc;                                                                               \
    }
