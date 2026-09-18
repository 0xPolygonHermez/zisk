// GPU Keccakf witness generation — spike implementation.
//
// Mapping: one Keccak-f[1600] permutation per warp lane, so a warp evaluates 32
// independent permutations in lockstep. The trace pairs two operations per slot
// (cells hold v = a + 8·b), so lanes 2i / 2i+1 form slot i and exchange their
// values with `__shfl_xor_sync(..., 1)`. The pair also splits the slot's rows:
// the A lane writes even group-rows, the B lane odd ones.
//
// Output is the packed `KeccakfTraceRowPacked` layout, bit-identical to the CPU
// path: 51 u64 per row, LSB-first, fields in declaration order —
//   [0,4) flags | [4,1284) state[320]×4b | [1284,1540) c[64]×4b
//   | [1540,3204) chi_acc[64]×26b | [3204,3244) step_addr

#include <cstdint>
#include <cstdio>
#include <cstring>
#include <cuda_runtime.h>

namespace {

constexpr int LANES          = 25;
constexpr int ROUNDS         = 24;
constexpr int LANES_PER_ROW  = 5;
constexpr int ROWS_PER_STATE = LANES / LANES_PER_ROW;      // 5
constexpr int GROUPS         = 2 + (1 + ROUNDS) + 2;       // 29
constexpr int CLOCKS         = GROUPS * ROWS_PER_STATE;    // 145
constexpr int ROW_WORDS      = 51;
constexpr int GROUP_IN_A     = 0;
constexpr int GROUP_IN_B     = ROWS_PER_STATE;
constexpr int GROUP_ROUND_0  = 2 * ROWS_PER_STATE;
constexpr int GROUP_OUT_A    = (3 + ROUNDS) * ROWS_PER_STATE;
constexpr int GROUP_OUT_B    = (4 + ROUNDS) * ROWS_PER_STATE;

constexpr uint32_t CHI_BASE     = 28;
constexpr uint32_t CHI_SPAN     = 17210368u;               // 28^5
constexpr uint64_t STEP_ADDR_MASK = (1ull << 40) - 1;

// Packed row layout, mirroring the `trace_row!` declaration: four flag bits, the
// row's state cells, its parity cells, the packed chi inputs, then step_addr.
// The static_asserts are what keeps this file honest if the PIL row ever moves.
constexpr int FLAG_BITS   = 4;
constexpr int STATE_CELLS = LANES_PER_ROW * 64;  // 320
constexpr int C_CELLS     = 64;
constexpr int CHI_CELLS   = 64;
constexpr int ROW_BITS =
    FLAG_BITS + STATE_CELLS * 4 + C_CELLS * 4 + CHI_CELLS * 26 + 40;
static_assert(ROW_BITS == 3244, "packed Keccakf row is 3244 bits");
static_assert(ROW_WORDS == (ROW_BITS + 63) / 64, "ROW_WORDS must cover ROW_BITS");
constexpr unsigned FULL_MASK    = 0xffffffffu;

__device__ __constant__ uint64_t d_rc[ROUNDS] = {
    0x0000000000000001ull, 0x0000000000008082ull, 0x800000000000808Aull, 0x8000000080008000ull,
    0x000000000000808Bull, 0x0000000080000001ull, 0x8000000080008081ull, 0x8000000000008009ull,
    0x000000000000008Aull, 0x0000000000000088ull, 0x0000000080008009ull, 0x000000008000000Aull,
    0x000000008000808Bull, 0x800000000000008Bull, 0x8000000000008089ull, 0x8000000000008003ull,
    0x8000000000008002ull, 0x8000000000000080ull, 0x000000000000800Aull, 0x800000008000000Aull,
    0x8000000080008081ull, 0x8000000000008080ull, 0x0000000080000001ull, 0x8000000080008008ull,
};

// rho offsets flattened as r[sx * 5 + sy], matching RHO_OFFSETS[sx][sy] on the CPU.
__device__ __constant__ int d_rho[LANES] = {
     0, 36,  3, 41, 18,
     1, 44, 10, 45,  2,
    62,  6, 43, 15, 61,
    28, 55, 25, 21, 56,
    27, 20, 39,  8, 14,
};

__device__ __forceinline__ uint64_t rotl64(uint64_t v, int n) {
    return n == 0 ? v : ((v << n) | (v >> (64 - n)));
}

// Spread sixteen bits into the low bit of sixteen consecutive nibbles.
__device__ __forceinline__ uint64_t spread16(uint64_t v) {
    v &= 0xffffull;
    v = (v | (v << 24)) & 0x000000ff000000ffull;
    v = (v | (v << 12)) & 0x000f000f000f000full;
    v = (v | (v << 6)) & 0x0303030303030303ull;
    return (v | (v << 3)) & 0x1111111111111111ull;
}

// One packed word: sixteen sliced cells a + 8·b taken from bit chunk `chunk`.
__device__ __forceinline__ uint64_t slice_word(uint64_t a, uint64_t b, int chunk) {
    const int sh = 16 * chunk;
    return spread16(a >> sh) | (spread16(b >> sh) << 3);
}

// Streaming bit writer: the row is emitted field by field, one u64 store per
// full word, so no read-modify-write and no staging buffer is needed.
struct Emitter {
    uint64_t* out;
    uint64_t carry;
    int bits;
    int idx;
};

__device__ __forceinline__ void emit(Emitter& e, uint64_t v, int n) {
    e.carry |= v << e.bits;  // e.bits < 64 by invariant
    const int total = e.bits + n;
    if (total >= 64) {
        e.out[e.idx++] = e.carry;
        // e.bits == 0 only when n == 64, and then v >> 64 contributes nothing.
        e.carry = (e.bits == 0) ? 0ull : (v >> (64 - e.bits));
        e.bits = total - 64;
    } else {
        e.bits = total;
    }
}

/// Emits one complete packed group-row.
///
/// `av`/`bv` are the row's five state lanes for ops A and B; `ca`/`cb` the
/// row's parity lane (group-row k holds parity column k, since C_PER_ROW == 64).
/// The χ inputs are the row's five θ-outputs per op, split into low/high planes.
__device__ __forceinline__ void emit_row(uint64_t* __restrict__ row,
                                         uint64_t flags,
                                         const uint64_t* av,
                                         const uint64_t* bv,
                                         uint64_t ca,
                                         uint64_t cb,
                                         bool has_chi,
                                         const uint64_t* la,
                                         const uint64_t* ha,
                                         const uint64_t* lb,
                                         const uint64_t* hb,
                                         uint64_t rc_bits,
                                         uint64_t step_addr) {
    Emitter e{row, flags, 4, 0};

#pragma unroll
    for (int l = 0; l < LANES_PER_ROW; ++l) {
#pragma unroll
        for (int c = 0; c < 4; ++c) {
            emit(e, slice_word(av[l], bv[l], c), 64);
        }
    }
#pragma unroll
    for (int c = 0; c < 4; ++c) {
        emit(e, slice_word(ca, cb, c), 64);
    }

    for (int z = 0; z < 64; ++z) {
        uint32_t v = 0;
        if (has_chi) {
#pragma unroll
            for (int x = LANES_PER_ROW - 1; x >= 0; --x) {
                const uint32_t ta =
                    (uint32_t)((la[x] >> z) & 1) | ((uint32_t)((ha[x] >> z) & 1) << 1);
                const uint32_t tb =
                    (uint32_t)((lb[x] >> z) & 1) | ((uint32_t)((hb[x] >> z) & 1) << 1);
                v = v * CHI_BASE + ta + 8u * tb;
            }
            v += (uint32_t)((rc_bits >> z) & 1) * CHI_SPAN;
        }
        emit(e, v, 26);
    }

    emit(e, step_addr & STEP_ADDR_MASK, 40);
    // ROW_BITS is not a multiple of 64, so a partial word always remains; the
    // guard keeps the store in bounds if that ever stops being true.
    if (e.bits != 0) {
        row[e.idx] = e.carry;
    }
}

}  // namespace

extern "C" struct KeccakfGpuOp {
    uint64_t state[LANES];
    uint64_t step;
    uint64_t addr;
};

namespace {

__global__ __launch_bounds__(256) void keccakf_witness_kernel(
    const KeccakfGpuOp* __restrict__ ops,
    uint32_t num_ops,
    uint32_t num_slots,
    uint64_t* __restrict__ out) {
    const uint32_t op = blockIdx.x * blockDim.x + threadIdx.x;
    const uint32_t slot = op >> 1;
    const bool is_a = (op & 1u) == 0u;
    // Threads past the last slot stay resident: every `__shfl_xor_sync` below
    // uses the full warp mask, so no lane may exit early.
    const bool in_range = slot < num_slots;
    const bool active = op < num_ops;

    uint64_t st[LANES];
    // Aims an inactive lane at op 0 rather than null: the guarded loads below are
    // predicated today, but if-conversion would turn a null base into a real
    // dereference. num_ops >= 1 is checked by the caller, so op 0 always exists.
    const KeccakfGpuOp* src = active ? &ops[op] : &ops[0];
#pragma unroll
    for (int i = 0; i < LANES; ++i) {
        st[i] = active ? src->state[i] : 0ull;
    }

    const uint64_t my_step = active ? src->step : 0ull;
    const uint64_t my_addr = active ? src->addr : 0ull;
    const uint64_t ot_step = __shfl_xor_sync(FULL_MASK, my_step, 1);
    const uint64_t ot_addr = __shfl_xor_sync(FULL_MASK, my_addr, 1);
    const uint64_t step_a = is_a ? my_step : ot_step;
    const uint64_t addr_a = is_a ? my_addr : ot_addr;
    const uint64_t step_b = is_a ? ot_step : my_step;
    const uint64_t addr_b = is_a ? ot_addr : my_addr;

    const uint32_t ot_active = __shfl_xor_sync(FULL_MASK, (uint32_t)active, 1);
    const bool in_use_a = is_a ? active : (ot_active != 0);
    const bool in_use_b = is_a ? (ot_active != 0) : active;
    const uint64_t flags = (uint64_t)in_use_a | ((uint64_t)in_use_b << 1);

    uint64_t* const slot_out = out + (size_t)slot * CLOCKS * ROW_WORDS;
    const uint64_t zeros[LANES_PER_ROW] = {0, 0, 0, 0, 0};

    // ── Boundary input groups: plain bits of each op, b-slot empty ──────────
    for (int k = 0; k < ROWS_PER_STATE; ++k) {
        uint64_t a_in[LANES_PER_ROW];
        uint64_t b_in[LANES_PER_ROW];
#pragma unroll
        for (int l = 0; l < LANES_PER_ROW; ++l) {
            const uint64_t mine = st[k * LANES_PER_ROW + l];
            const uint64_t theirs = __shfl_xor_sync(FULL_MASK, mine, 1);
            a_in[l] = is_a ? mine : theirs;
            b_in[l] = is_a ? theirs : mine;
        }
        const bool owns = ((k & 1) == 0) == is_a;
        if (in_range && owns) {
            // Rows 0..3 of the slot carry the two ops' step and addr.
            uint64_t step_addr = 0;
            if (k == 0) {
                step_addr = step_a;
            } else if (k == 1) {
                step_addr = addr_a;
            } else if (k == 2) {
                step_addr = in_use_b ? step_b : 0ull;
            } else if (k == 3) {
                step_addr = in_use_b ? addr_b : 0ull;
            }
            emit_row(slot_out + (size_t)(GROUP_IN_A + k) * ROW_WORDS, flags, a_in, zeros, 0, 0,
                     false, zeros, zeros, zeros, zeros, 0, step_addr);
            emit_row(slot_out + (size_t)(GROUP_IN_B + k) * ROW_WORDS, flags, b_in, zeros, 0, 0,
                     false, zeros, zeros, zeros, zeros, 0, 0);
        }
    }

    // ── Round groups ───────────────────────────────────────────────────────
    for (int r = 0; r <= ROUNDS; ++r) {
        const bool last = (r == ROUNDS);
        uint64_t* const grp = slot_out + (size_t)(GROUP_ROUND_0 + r * ROWS_PER_STATE) * ROW_WORDS;

        uint64_t par[LANES_PER_ROW];
        uint64_t pa[LANES_PER_ROW];
        uint64_t pb[LANES_PER_ROW];
        if (!last) {
#pragma unroll
            for (int x = 0; x < 5; ++x) {
                par[x] = st[x] ^ st[x + 5] ^ st[x + 10] ^ st[x + 15] ^ st[x + 20];
            }
#pragma unroll
            for (int x = 0; x < 5; ++x) {
                const uint64_t theirs = __shfl_xor_sync(FULL_MASK, par[x], 1);
                pa[x] = is_a ? par[x] : theirs;
                pb[x] = is_a ? theirs : par[x];
            }
        }

        uint64_t next[LANES];
        for (int k = 0; k < ROWS_PER_STATE; ++k) {
            uint64_t sa[LANES_PER_ROW];
            uint64_t sb[LANES_PER_ROW];
#pragma unroll
            for (int l = 0; l < LANES_PER_ROW; ++l) {
                const uint64_t mine = st[k * LANES_PER_ROW + l];
                const uint64_t theirs = __shfl_xor_sync(FULL_MASK, mine, 1);
                sa[l] = is_a ? mine : theirs;
                sb[l] = is_a ? theirs : mine;
            }

            uint64_t la[LANES_PER_ROW], ha[LANES_PER_ROW];
            uint64_t lb[LANES_PER_ROW], hb[LANES_PER_ROW];
            if (!last) {
                uint64_t lo[LANES_PER_ROW], hi[LANES_PER_ROW];
#pragma unroll
                for (int x = 0; x < 5; ++x) {
                    // χ-position (x, k) reads ρπ from source (x + 3k, x).
                    const int sx = (x + 3 * k) % 5;
                    const int sy = x;
                    const uint64_t a = st[sx + 5 * sy];
                    const uint64_t b = par[(sx + 4) % 5];
                    const uint64_t c = rotl64(par[(sx + 1) % 5], 1);
                    const int rot = d_rho[sx * 5 + sy];
                    lo[x] = rotl64(a ^ b ^ c, rot);
                    hi[x] = rotl64((a & b) | (a & c) | (b & c), rot);
                }
#pragma unroll
                for (int x = 0; x < 5; ++x) {
                    const uint64_t pl = __shfl_xor_sync(FULL_MASK, lo[x], 1);
                    const uint64_t ph = __shfl_xor_sync(FULL_MASK, hi[x], 1);
                    la[x] = is_a ? lo[x] : pl;
                    lb[x] = is_a ? pl : lo[x];
                    ha[x] = is_a ? hi[x] : ph;
                    hb[x] = is_a ? ph : hi[x];
                }
                // χ and ι over the clean low θ plane, one state-row at a time.
#pragma unroll
                for (int x = 0; x < 5; ++x) {
                    next[x + 5 * k] = lo[x] ^ ((~lo[(x + 1) % 5]) & lo[(x + 2) % 5]);
                }
                if (k == 0) {
                    next[0] ^= d_rc[r];
                }
            }

            const bool owns = ((k & 1) == 0) == is_a;
            if (in_range && owns) {
                emit_row(grp + (size_t)k * ROW_WORDS, flags, sa, sb, last ? 0 : pa[k],
                         last ? 0 : pb[k], !last, la, ha, lb, hb,
                         (!last && k == 0) ? d_rc[r] : 0ull, 0);
            }
        }

        if (!last) {
#pragma unroll
            for (int i = 0; i < LANES; ++i) {
                st[i] = next[i];
            }
        }
    }

    // ── Boundary output groups ─────────────────────────────────────────────
    for (int k = 0; k < ROWS_PER_STATE; ++k) {
        uint64_t a_out[LANES_PER_ROW];
        uint64_t b_out[LANES_PER_ROW];
#pragma unroll
        for (int l = 0; l < LANES_PER_ROW; ++l) {
            const uint64_t mine = st[k * LANES_PER_ROW + l];
            const uint64_t theirs = __shfl_xor_sync(FULL_MASK, mine, 1);
            a_out[l] = is_a ? mine : theirs;
            b_out[l] = is_a ? theirs : mine;
        }
        const bool owns = ((k & 1) == 0) == is_a;
        if (in_range && owns) {
            emit_row(slot_out + (size_t)(GROUP_OUT_A + k) * ROW_WORDS, flags, a_out, zeros, 0, 0,
                     false, zeros, zeros, zeros, zeros, 0, 0);
            emit_row(slot_out + (size_t)(GROUP_OUT_B + k) * ROW_WORDS, flags, b_out, zeros, 0, 0,
                     false, zeros, zeros, zeros, zeros, 0, 0);
        }
    }
}

}  // namespace

extern "C" struct KeccakfGpuTimings {
    double alloc_ms;      // device allocation + poison memset
    double h2d_ms;        // upload of the operation inputs
    double kernel_ms;     // best of `iters` kernel launches
    double kernel_avg_ms; // mean over `iters` launches
    double d2h_ms;        // download of the packed trace into pinned host memory
    uint64_t out_bytes;   // size of the packed trace produced
};

// Every CUDA failure unwinds through one cleanup path: an OOM on the output
// allocation must not strand the input buffer, or a caller retrying with a
// smaller batch leaks again on every attempt.
#define CUDA_TRY(call)                                                                  \
    do {                                                                                \
        const cudaError_t _e = (call);                                                  \
        if (_e != cudaSuccess) {                                                        \
            fprintf(stderr, "[keccakf-gpu] %s:%d %s -> %s\n", __FILE__, __LINE__, #call, \
                    cudaGetErrorString(_e));                                            \
            rc = -1;                                                                    \
            goto cleanup;                                                               \
        }                                                                               \
    } while (0)

extern "C" int keccakf_gpu_available() {
    int count = 0;
    return cudaGetDeviceCount(&count) == cudaSuccess && count > 0 ? 1 : 0;
}

extern "C" int keccakf_gpu_row_words() { return ROW_WORDS; }
extern "C" int keccakf_gpu_clocks() { return CLOCKS; }

/// Runs the witness kernel for `num_ops` operations.
///
/// `out_host`, when non-null, receives the packed trace (num_slots · CLOCKS ·
/// ROW_WORDS u64) and the download is timed separately. The device buffer is
/// poisoned with 0xAA before the first launch, so any word the kernel fails to
/// write shows up as a mismatch rather than as an accidental zero.
extern "C" int keccakf_gpu_witness(const KeccakfGpuOp* ops,
                                   uint32_t num_ops,
                                   uint64_t* out_host,
                                   uint32_t iters,
                                   int block,
                                   KeccakfGpuTimings* t) {
    if (num_ops == 0 || iters == 0) {
        return -1;
    }
    if (block <= 0) {
        block = 64;
    }
    // The two ops of a slot are warp neighbours exchanged with
    // `__shfl_xor_sync(FULL_MASK, v, 1)`: a block that is not a whole number of
    // warps leaves partial warps (undefined behaviour under the full mask), and
    // an odd block would split a slot across blocks entirely.
    if (block % 32 != 0 || block > 256) {
        fprintf(stderr, "[keccakf-gpu] block must be a multiple of 32 and <= 256, got %d\n", block);
        return -1;
    }

    const uint32_t num_slots = (num_ops + 1) / 2;
    const size_t out_words = (size_t)num_slots * CLOCKS * ROW_WORDS;
    const size_t out_bytes = out_words * sizeof(uint64_t);
    const size_t in_bytes = (size_t)num_ops * sizeof(KeccakfGpuOp);

    int rc = 0;
    cudaEvent_t e0 = nullptr, e1 = nullptr;
    KeccakfGpuOp* d_ops = nullptr;
    uint64_t* d_out = nullptr;
    KeccakfGpuOp* h_ops = nullptr;
    uint64_t* h_out = nullptr;
    float ms = 0.0f;

    CUDA_TRY(cudaEventCreate(&e0));
    CUDA_TRY(cudaEventCreate(&e1));

    CUDA_TRY(cudaEventRecord(e0));
    CUDA_TRY(cudaMalloc(&d_ops, in_bytes));
    CUDA_TRY(cudaMalloc(&d_out, out_bytes));
    // Poison, so a word the kernel never writes shows up as a mismatch against
    // the CPU trace rather than as a plausible zero.
    CUDA_TRY(cudaMemset(d_out, 0xAA, out_bytes));
    CUDA_TRY(cudaEventRecord(e1));
    CUDA_TRY(cudaEventSynchronize(e1));
    CUDA_TRY(cudaEventElapsedTime(&ms, e0, e1));
    t->alloc_ms = ms;

    CUDA_TRY(cudaHostAlloc(&h_ops, in_bytes, cudaHostAllocDefault));
    memcpy(h_ops, ops, in_bytes);
    CUDA_TRY(cudaEventRecord(e0));
    CUDA_TRY(cudaMemcpy(d_ops, h_ops, in_bytes, cudaMemcpyHostToDevice));
    CUDA_TRY(cudaEventRecord(e1));
    CUDA_TRY(cudaEventSynchronize(e1));
    CUDA_TRY(cudaEventElapsedTime(&ms, e0, e1));
    t->h2d_ms = ms;

    {
        const uint32_t threads = num_slots * 2;
        const uint32_t grid = (threads + block - 1) / block;
        double best = 1e30;
        double sum = 0.0;
        for (uint32_t i = 0; i < iters; ++i) {
            CUDA_TRY(cudaEventRecord(e0));
            keccakf_witness_kernel<<<grid, block>>>(d_ops, num_ops, num_slots, d_out);
            CUDA_TRY(cudaEventRecord(e1));
            CUDA_TRY(cudaEventSynchronize(e1));
            CUDA_TRY(cudaGetLastError());
            CUDA_TRY(cudaEventElapsedTime(&ms, e0, e1));
            best = ms < best ? ms : best;
            sum += ms;
        }
        t->kernel_ms = best;
        t->kernel_avg_ms = sum / iters;
    }
    t->out_bytes = out_bytes;
    t->d2h_ms = 0.0;

    if (out_host != nullptr) {
        CUDA_TRY(cudaHostAlloc(&h_out, out_bytes, cudaHostAllocDefault));
        CUDA_TRY(cudaEventRecord(e0));
        CUDA_TRY(cudaMemcpy(h_out, d_out, out_bytes, cudaMemcpyDeviceToHost));
        CUDA_TRY(cudaEventRecord(e1));
        CUDA_TRY(cudaEventSynchronize(e1));
        CUDA_TRY(cudaEventElapsedTime(&ms, e0, e1));
        t->d2h_ms = ms;
        memcpy(out_host, h_out, out_bytes);
    }

cleanup:
    if (h_out != nullptr) cudaFreeHost(h_out);
    if (h_ops != nullptr) cudaFreeHost(h_ops);
    if (d_ops != nullptr) cudaFree(d_ops);
    if (d_out != nullptr) cudaFree(d_out);
    if (e0 != nullptr) cudaEventDestroy(e0);
    if (e1 != nullptr) cudaEventDestroy(e1);
    return rc;
}
