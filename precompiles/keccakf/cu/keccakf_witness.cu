// GPU Keccakf witness generation.
//
// Mapping: one Keccak-f[1600] permutation per warp lane, so a warp evaluates 32
// independent permutations in lockstep. The trace pairs two operations per slot
// (cells hold v = a + 8·b), so lanes 2i / 2i+1 form slot i and exchange their
// values with `pair` (`__shfl_xor_sync(..., 1)`).
//
// Output is the packed `KeccakfTraceRowPacked` layout, bit-identical to the CPU
// path: 57 u64 per row, LSB-first, fields in declaration order —
//   [0,2) flags | [2,1282) state[320]×4b | [1282,1538) c[64]×4b
//   | [1538,1922) xor5_acc[16]×24b | [1922,3586) chi_acc[64]×26b | [3586,3626) step_addr

#include <cstdint>
#include <cuda_runtime.h>
#include <zisk_gpu_witness.cuh>

namespace {

constexpr int LANES          = 25;
constexpr int ROUNDS         = 24;
constexpr int LANES_PER_ROW  = 5;
constexpr int ROWS_PER_STATE = LANES / LANES_PER_ROW;      // 5
// Op B has no boundary groups: the AIR reads its bits as (v - a) / 8.
constexpr int GROUPS         = 1 + (1 + ROUNDS) + 1;       // 27
constexpr int CLOCKS         = GROUPS * ROWS_PER_STATE;    // 135
constexpr int ROW_WORDS      = 57;
constexpr int GROUP_IN_A     = 0;
constexpr int GROUP_ROUND_0  = ROWS_PER_STATE;
constexpr int GROUP_OUT_A    = (2 + ROUNDS) * ROWS_PER_STATE;

constexpr uint32_t CHI_BASE     = 28;
constexpr uint32_t CHI_SPAN     = 17210368u;               // 28^5
constexpr uint64_t STEP_ADDR_MASK = (1ull << 40) - 1;

// Packed row layout, mirroring the `trace_row!` declaration: two flag bits, the
// row's state cells, its parity cells, the packed xor5 keys, the packed chi
// inputs, then step_addr.
// The static_asserts are what keeps this file honest if the PIL row ever moves.
constexpr int FLAG_BITS   = 2;
constexpr int STATE_CELLS = LANES_PER_ROW * 64;  // 320
constexpr int C_CELLS     = 64;
constexpr int XOR5_BATCH  = 4;
constexpr int XOR5_CELLS  = C_CELLS / XOR5_BATCH;  // 16
constexpr int XOR5_DIGIT  = 6;                     // base 64: a sum sA + 8·sB <= 45
constexpr int CHI_CELLS   = 64;
constexpr int ROW_BITS =
    FLAG_BITS + STATE_CELLS * 4 + C_CELLS * 4 + XOR5_CELLS * 24 + CHI_CELLS * 26 + 40;
static_assert(ROW_BITS == 3626, "packed Keccakf row is 3626 bits");
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

/// Flags, then the row's five state lanes and its parity lane for ops A and B
/// (group-row k holds parity column k, since C_PER_ROW == 64).
__device__ __forceinline__ void emit_cells(Emitter& e, const uint64_t* av, const uint64_t* bv,
                                           uint64_t ca, uint64_t cb) {
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
}

/// step_addr and the final partial word. ROW_BITS is not a multiple of 64, so a
/// partial word always remains; the guard keeps the store in bounds otherwise.
__device__ __forceinline__ void emit_tail(Emitter& e, uint64_t step_addr) {
    emit(e, step_addr & STEP_ADDR_MASK, 40);
    if (e.bits != 0) {
        e.out[e.idx] = e.carry;
    }
}

/// A row without xor5 or χ inputs: the boundary groups and the last round's output group.
__device__ __forceinline__ void emit_state_row(uint64_t* __restrict__ row, uint64_t flags,
                                               const uint64_t* av, const uint64_t* bv,
                                               uint64_t step_addr) {
    Emitter e{row, flags, FLAG_BITS, 0};
    emit_cells(e, av, bv, 0, 0);
    for (int g = 0; g < XOR5_CELLS; ++g) {
        emit(e, 0, 24);
    }
    for (int z = 0; z < CHI_CELLS; ++z) {
        emit(e, 0, 26);
    }
    emit_tail(e, step_addr);
}

/// A round row: parity cells, the xor5 keys, then the χ inputs, the row's five
/// θ-outputs per op split into low/high planes, with ι's round-constant bit on row 0.
/// `sa` / `sb` are the bit planes of the row's column sums (column x = group-row k).
__device__ __forceinline__ void emit_round_row(uint64_t* __restrict__ row, uint64_t flags,
                                               const uint64_t* av, const uint64_t* bv,
                                               uint64_t ca, uint64_t cb,
                                               const uint64_t* sa, const uint64_t* sb,
                                               const uint64_t* la, const uint64_t* ha,
                                               const uint64_t* lb, const uint64_t* hb,
                                               uint64_t rc_bits) {
    Emitter e{row, flags, FLAG_BITS, 0};
    emit_cells(e, av, bv, ca, cb);
    // xor5_acc[g]: the sliced sums of positions z = 4g..4g+3, digit k at base 64^k.
    for (int g = 0; g < XOR5_CELLS; ++g) {
        uint32_t acc = 0;
#pragma unroll
        for (int k = 0; k < XOR5_BATCH; ++k) {
            const int z = g * XOR5_BATCH + k;
            uint32_t s_a = 0, s_b = 0;
#pragma unroll
            for (int p = 0; p < 3; ++p) {
                s_a |= (uint32_t)((sa[p] >> z) & 1) << p;
                s_b |= (uint32_t)((sb[p] >> z) & 1) << p;
            }
            acc |= (s_a + 8u * s_b) << (XOR5_DIGIT * k);
        }
        emit(e, acc, 24);
    }
    for (int z = 0; z < CHI_CELLS; ++z) {
        uint32_t v = 0;
#pragma unroll
        for (int x = LANES_PER_ROW - 1; x >= 0; --x) {
            const uint32_t ta = (uint32_t)((la[x] >> z) & 1) | ((uint32_t)((ha[x] >> z) & 1) << 1);
            const uint32_t tb = (uint32_t)((lb[x] >> z) & 1) | ((uint32_t)((hb[x] >> z) & 1) << 1);
            v = v * CHI_BASE + ta + 8u * tb;
        }
        v += (uint32_t)((rc_bits >> z) & 1) * CHI_SPAN;
        emit(e, v, 26);
    }
    emit_tail(e, 0);
}

/// Exchange `mine` with the pair lane and order the two values as (op A, op B).
__device__ __forceinline__ void pair(uint64_t mine, bool is_a, uint64_t& a, uint64_t& b) {
    const uint64_t theirs = __shfl_xor_sync(FULL_MASK, mine, 1);
    a = is_a ? mine : theirs;
    b = is_a ? theirs : mine;
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

    // An inactive lane loads zeros, so op B's step and addr are 0 when it is absent.
    uint64_t step_a, step_b, addr_a, addr_b;
    pair(active ? src->step : 0ull, is_a, step_a, step_b);
    pair(active ? src->addr : 0ull, is_a, addr_a, addr_b);
    uint64_t in_use_a, in_use_b;
    pair((uint64_t)active, is_a, in_use_a, in_use_b);
    const uint64_t flags = in_use_a | (in_use_b << 1);

    // Row-major. Column-major was tried and costs this kernel 0.86 -> 3.73 ms per instance
    // (498 -> 115 GB/s: 8 useful bytes per 128-byte line) and cannot be coalesced here -- the
    // algorithm is slot-parallel, so neighbouring warp lanes own rows CLOCKS apart. The commit
    // transposes instead, where one thread per row makes both sides coalesce.
    uint64_t* const slot_out = out + (size_t)slot * CLOCKS * ROW_WORDS;
    auto row_at = [&](int clock) { return slot_out + (size_t)clock * ROW_WORDS; };
    // The pair splits a slot's rows: the A lane writes even group-rows, the B lane odd ones.
    auto owns = [&](int k) { return in_range && (((k & 1) == 0) == is_a); };
    const uint64_t zeros[LANES_PER_ROW] = {0, 0, 0, 0, 0};

    // Boundary groups: op A's plain state, the b-slot empty. Rows 0..3 of the
    // input group carry the two ops' step and addr.
    auto emit_boundary = [&](int group_a, bool with_step_addr) {
        const uint64_t step_addr[ROWS_PER_STATE] = {step_a, addr_a, step_b, addr_b, 0};
        for (int k = 0; k < ROWS_PER_STATE; ++k) {
            uint64_t a[LANES_PER_ROW], b[LANES_PER_ROW];
#pragma unroll
            for (int l = 0; l < LANES_PER_ROW; ++l) {
                pair(st[k * LANES_PER_ROW + l], is_a, a[l], b[l]);
            }
            if (owns(k)) {
                emit_state_row(row_at(group_a + k), flags, a, zeros, with_step_addr ? step_addr[k] : 0);
            }
        }
    };

    emit_boundary(GROUP_IN_A, true);

    for (int r = 0; r <= ROUNDS; ++r) {
        const int group = GROUP_ROUND_0 + r * ROWS_PER_STATE;
        // The 25th group holds the permutation's output, with no θ/χ of its own.
        if (r == ROUNDS) {
            for (int k = 0; k < ROWS_PER_STATE; ++k) {
                uint64_t sa[LANES_PER_ROW], sb[LANES_PER_ROW];
#pragma unroll
                for (int l = 0; l < LANES_PER_ROW; ++l) {
                    pair(st[k * LANES_PER_ROW + l], is_a, sa[l], sb[l]);
                }
                if (owns(k)) {
                    emit_state_row(row_at(group + k), flags, sa, sb, 0);
                }
            }
            break;
        }

        uint64_t par[LANES_PER_ROW];
#pragma unroll
        for (int x = 0; x < 5; ++x) {
            par[x] = st[x] ^ st[x + 5] ^ st[x + 10] ^ st[x + 15] ^ st[x + 20];
        }
        uint64_t pa[LANES_PER_ROW], pb[LANES_PER_ROW];
#pragma unroll
        for (int x = 0; x < 5; ++x) {
            pair(par[x], is_a, pa[x], pb[x]);
        }

        uint64_t next[LANES];
        for (int k = 0; k < ROWS_PER_STATE; ++k) {
            uint64_t sa[LANES_PER_ROW], sb[LANES_PER_ROW];
#pragma unroll
            for (int l = 0; l < LANES_PER_ROW; ++l) {
                pair(st[k * LANES_PER_ROW + l], is_a, sa[l], sb[l]);
            }

            // Column x = k's sums Σ_y st[k + 5y] in [0,5], as three bit planes.
            uint64_t sum_a[3], sum_b[3];
            {
                const uint64_t v0 = st[k], v1 = st[k + 5], v2 = st[k + 10];
                const uint64_t v3 = st[k + 15], v4 = st[k + 20];
                const uint64_t s1 = v0 ^ v1 ^ v2, c1 = (v0 & v1) | (v0 & v2) | (v1 & v2);
                const uint64_t s2 = s1 ^ v3 ^ v4, c2 = (s1 & v3) | (s1 & v4) | (v3 & v4);
                pair(s2, is_a, sum_a[0], sum_b[0]);
                pair(c1 ^ c2, is_a, sum_a[1], sum_b[1]);
                pair(c1 & c2, is_a, sum_a[2], sum_b[2]);
            }

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
            uint64_t la[LANES_PER_ROW], ha[LANES_PER_ROW], lb[LANES_PER_ROW], hb[LANES_PER_ROW];
#pragma unroll
            for (int x = 0; x < 5; ++x) {
                pair(lo[x], is_a, la[x], lb[x]);
                pair(hi[x], is_a, ha[x], hb[x]);
            }
            // χ and ι over the clean low θ plane, one state-row at a time.
#pragma unroll
            for (int x = 0; x < 5; ++x) {
                next[x + 5 * k] = lo[x] ^ ((~lo[(x + 1) % 5]) & lo[(x + 2) % 5]);
            }
            if (k == 0) {
                next[0] ^= d_rc[r];
            }

            if (owns(k)) {
                emit_round_row(row_at(group + k), flags, sa, sb, pa[k], pb[k], sum_a, sum_b, la, ha, lb, hb,
                               k == 0 ? d_rc[r] : 0ull);
            }
        }

#pragma unroll
        for (int i = 0; i < LANES; ++i) {
            st[i] = next[i];
        }
    }

    emit_boundary(GROUP_OUT_A, false);
}

}  // namespace

/// u64 the packed trace of `num_ops` operations occupies: two ops share a slot,
/// each slot spans CLOCKS rows of ROW_WORDS words.
static uint64_t keccakf_out_words(uint32_t num_ops) {
    const uint64_t num_slots = ((uint64_t)num_ops + 1) / 2;
    return num_slots * CLOCKS * ROW_WORDS;
}

/// One thread per operation, so a slot's two ops are adjacent lanes. Sized in 64 bits:
/// `num_ops` may be UINT32_MAX, whose thread count does not fit 32.
static int keccakf_launch(const KeccakfGpuOp* d_ops, uint32_t num_ops, uint64_t* d_dst,
                          int block, cudaStream_t stream) {
    const uint64_t num_slots = ((uint64_t)num_ops + 1) / 2;
    const uint64_t grid = (num_slots * 2 + block - 1) / block;
    keccakf_witness_kernel<<<(uint32_t)grid, block, 0, stream>>>(d_ops, num_ops, (uint32_t)num_slots, d_dst);
    return 0;
}

// available / out_words / row_words / fill / fill_probe.
ZISK_GPU_WITNESS_ENTRIES(keccakf, KeccakfGpuOp, ROW_WORDS, 128, 256, keccakf_out_words, keccakf_launch)
