// =====================================================================
// CountAndPlan — implementation
// =====================================================================

#include "count_and_plan.cuh"

#include <cub/device/device_radix_sort.cuh>
#include <cub/device/device_run_length_encode.cuh>
#include <cub/device/device_scan.cuh>
#include <thrust/iterator/discard_iterator.h>

#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <iostream>

// =====================================================================
// Preprocessing constants
// =====================================================================

#define MOPS_WRITE_FLAG               0x10
#define MOPS_WRITE_BYTE_CLEAR_FLAG    0x20

#define MOPS_READ_8                   0x08
#define MOPS_READ_4                   0x04
#define MOPS_READ_2                   0x02
#define MOPS_READ_1                   0x01

#define MOPS_WRITE_8                  0x18
#define MOPS_WRITE_4                  0x14
#define MOPS_WRITE_2                  0x12
#define MOPS_WRITE_1                  0x11

#define MOPS_CWRITE_1                 0x31

#define MOPS_BLOCK_READ               0x0A
#define MOPS_BLOCK_WRITE              0x0B
#define MOPS_ALIGNED_READ             0x0C
#define MOPS_ALIGNED_WRITE            0x0D
#define MOPS_ALIGNED_BLOCK_READ       0x0E
#define MOPS_ALIGNED_BLOCK_WRITE      0x0F

#define MOPS_BLOCK_COUNT_SBITS        4

constexpr uint32_t ZISK_ROM_ADDR_BASE     = 0x80000000u;
constexpr uint32_t ZISK_INPUT_ADDR_BASE   = 0x40000000u;
constexpr uint32_t ZISK_RAM_ADDR_BASE     = 0xA0000000u;
constexpr uint32_t ZISK_RAM_SIZE_BYTES    = 512u * 1024u * 1024u;
constexpr uint32_t ZISK_RAM_ADDR_END      = ZISK_RAM_ADDR_BASE + ZISK_RAM_SIZE_BYTES;
constexpr uint32_t ZISK_ALIGN_MASK        = 0xFFFFFFF8u;

constexpr uint32_t MAX_BLOCKOP_SPILL_PER_CHUNK = 16u * 1024u;
constexpr uint32_t BLOCKOP_SPILL_THRESH_VAL    = 64u;

// =====================================================================
// Full definitions for the types forward-declared in the header.
// =====================================================================

struct __align__(4) PotentialEmit {
    uint32_t aligned_addr_packed;
};

#define POT_FLAG_IS_RAM   0x1u
#define POT_FLAG_KIND_W   0x2u
#define POT_FLAG_MASK     0x7u

__host__ __device__ __forceinline__
uint32_t emit_aligned_addr(PotentialEmit p) { return p.aligned_addr_packed & ~POT_FLAG_MASK; }

__host__ __device__ __forceinline__
bool emit_is_ram(PotentialEmit p) { return (p.aligned_addr_packed & POT_FLAG_IS_RAM) != 0; }

__host__ __device__ __forceinline__
bool emit_kind_w(PotentialEmit p) { return (p.aligned_addr_packed & POT_FLAG_KIND_W) != 0; }

struct BlockOpSpill {
    uint32_t memop_idx;
    uint32_t aligned_base;
    uint32_t count;
    uint32_t kind_w;
};


// 64-bit RAM sort-key bit layout (see kernels below).
#define KIND_W_BIT          0u
#define ORIG_POS_SHIFT      1u
#define ORIG_POS_BITS       21u
#define ORIG_POS_MASK       ((1u << ORIG_POS_BITS) - 1u)
#define COMPACT_ADDR_SHIFT  (ORIG_POS_SHIFT + ORIG_POS_BITS)   // 22
#define RAM_KEY_END_BIT     48

#define CUDA_CHECK(call) do {                                                  \
    cudaError_t _err = (call);                                                 \
    if (_err != cudaSuccess) {                                                 \
        fprintf(stderr, "CUDA error %s at %s:%d: %s\n",                        \
                cudaGetErrorString(_err), __FILE__, __LINE__, #call);          \
        exit(1);                                                               \
    }                                                                          \
} while (0)

__host__ __device__ __forceinline__
bool is_ram_addr(uint32_t addr) {
    return (addr >= ZISK_RAM_ADDR_BASE) && (addr < ZISK_RAM_ADDR_END);
}

__host__ __device__ __forceinline__
uint32_t ram_compact(uint32_t aligned_addr) {
    return (aligned_addr - ZISK_RAM_ADDR_BASE) >> 3;
}

__device__ __forceinline__
bool decode(MemOp op,
            uint32_t* count_out,
            ChunkCounters& counters_out,
            uint32_t* d_invalid_mode_flag) {
    const uint32_t addr        = op.addr;
    const uint32_t aligned     = addr & ZISK_ALIGN_MASK;
    const uint8_t  mode        = op.flags & 0x3Fu;
    const uint32_t off_in_word = addr & 0x07u;

    counters_out = ChunkCounters{0,0,0,0,0};

    switch (mode) {
        case MOPS_READ_1:
            *count_out = 1; counters_out.read_byte = 1; return true;
        case MOPS_CWRITE_1:
            *count_out = 2; counters_out.write_byte = 1; return true;
        case MOPS_WRITE_1:
            *count_out = 2; counters_out.full_3 = 1; return true;

        case MOPS_READ_2:
            if (off_in_word > 6) { *count_out = 2; counters_out.full_3 = 1; }
            else                 { *count_out = 1; counters_out.full_2 = 1; }
            return true;
        case MOPS_WRITE_2:
            if (off_in_word > 6) { *count_out = 4; counters_out.full_5 = 1; }
            else                 { *count_out = 2; counters_out.full_3 = 1; }
            return true;

        case MOPS_READ_4:
            if (off_in_word > 4) { *count_out = 2; counters_out.full_3 = 1; }
            else                 { *count_out = 1; counters_out.full_2 = 1; }
            return true;
        case MOPS_WRITE_4:
            if (off_in_word > 4) { *count_out = 4; counters_out.full_5 = 1; }
            else                 { *count_out = 2; counters_out.full_3 = 1; }
            return true;

        case MOPS_READ_8:
            if (off_in_word > 0) { *count_out = 2; counters_out.full_3 = 1; }
            else                 { *count_out = 1; }
            return true;
        case MOPS_WRITE_8:
            if (addr == aligned) { *count_out = 1; }
            else                 { *count_out = 4; counters_out.full_5 = 1; }
            return true;

        case MOPS_ALIGNED_READ  + 0x00: case MOPS_ALIGNED_READ  + 0x10:
        case MOPS_ALIGNED_READ  + 0x20: case MOPS_ALIGNED_READ  + 0x30:
        case MOPS_ALIGNED_WRITE + 0x00: case MOPS_ALIGNED_WRITE + 0x10:
        case MOPS_ALIGNED_WRITE + 0x20: case MOPS_ALIGNED_WRITE + 0x30:
            *count_out = 1; return true;

        case MOPS_BLOCK_READ        + 0x00: case MOPS_BLOCK_READ        + 0x10:
        case MOPS_BLOCK_READ        + 0x20: case MOPS_BLOCK_READ        + 0x30:
        case MOPS_ALIGNED_BLOCK_READ+ 0x00: case MOPS_ALIGNED_BLOCK_READ+ 0x10:
        case MOPS_ALIGNED_BLOCK_READ+ 0x20: case MOPS_ALIGNED_BLOCK_READ+ 0x30:
        case MOPS_BLOCK_WRITE        + 0x00: case MOPS_BLOCK_WRITE        + 0x10:
        case MOPS_BLOCK_WRITE        + 0x20: case MOPS_BLOCK_WRITE        + 0x30:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x00: case MOPS_ALIGNED_BLOCK_WRITE+ 0x10:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x20: case MOPS_ALIGNED_BLOCK_WRITE+ 0x30:
            *count_out = op.flags >> MOPS_BLOCK_COUNT_SBITS; return true;

        default:
            atomicOr(d_invalid_mode_flag, 1u);
            *count_out = 0;
            return false;
    }
}

__device__ __forceinline__
void emit_pair_rw(uint32_t aligned, PotentialEmit* out) {
    const uint32_t ram_bit = is_ram_addr(aligned) ? POT_FLAG_IS_RAM : 0u;
    out[0].aligned_addr_packed = aligned | ram_bit;                       // R
    out[1].aligned_addr_packed = aligned | ram_bit | POT_FLAG_KIND_W;     // W
}

__device__ __forceinline__
void emit_one_r(uint32_t aligned, PotentialEmit* out) {
    const uint32_t ram_bit = is_ram_addr(aligned) ? POT_FLAG_IS_RAM : 0u;
    out[0].aligned_addr_packed = aligned | ram_bit;
}

__device__ __forceinline__
void emit_one_w(uint32_t aligned, PotentialEmit* out) {
    const uint32_t ram_bit = is_ram_addr(aligned) ? POT_FLAG_IS_RAM : 0u;
    out[0].aligned_addr_packed = aligned | ram_bit | POT_FLAG_KIND_W;
}

__device__ __forceinline__
void decode_emit_inline(MemOp op, PotentialEmit* out, bool skip_block) {
    const uint32_t addr        = op.addr;
    const uint32_t aligned     = addr & ZISK_ALIGN_MASK;
    const uint8_t  mode        = op.flags & 0x3Fu;
    const uint32_t off_in_word = addr & 0x07u;

    switch (mode) {
        case MOPS_READ_1:                                         emit_one_r(aligned, out); break;
        case MOPS_CWRITE_1: case MOPS_WRITE_1:                    emit_pair_rw(aligned, out); break;

        case MOPS_READ_2:
            emit_one_r(aligned, out);
            if (off_in_word > 6) emit_one_r(aligned + 8, out + 1);
            break;
        case MOPS_WRITE_2:
            emit_pair_rw(aligned, out);
            if (off_in_word > 6) emit_pair_rw(aligned + 8, out + 2);
            break;

        case MOPS_READ_4:
            emit_one_r(aligned, out);
            if (off_in_word > 4) emit_one_r(aligned + 8, out + 1);
            break;
        case MOPS_WRITE_4:
            emit_pair_rw(aligned, out);
            if (off_in_word > 4) emit_pair_rw(aligned + 8, out + 2);
            break;

        case MOPS_READ_8:
            emit_one_r(aligned, out);
            if (off_in_word > 0) emit_one_r(aligned + 8, out + 1);
            break;
        case MOPS_WRITE_8:
            if (addr == aligned) {
                emit_one_w(aligned, out);
            } else {
                emit_pair_rw(aligned, out);
                emit_pair_rw(aligned + 8, out + 2);
            }
            break;

        case MOPS_ALIGNED_READ  + 0x00: case MOPS_ALIGNED_READ  + 0x10:
        case MOPS_ALIGNED_READ  + 0x20: case MOPS_ALIGNED_READ  + 0x30:
            emit_one_r(addr, out); break;

        case MOPS_ALIGNED_WRITE + 0x00: case MOPS_ALIGNED_WRITE + 0x10:
        case MOPS_ALIGNED_WRITE + 0x20: case MOPS_ALIGNED_WRITE + 0x30:
            emit_one_w(addr, out); break;

        case MOPS_BLOCK_READ        + 0x00: case MOPS_BLOCK_READ        + 0x10:
        case MOPS_BLOCK_READ        + 0x20: case MOPS_BLOCK_READ        + 0x30:
        case MOPS_ALIGNED_BLOCK_READ+ 0x00: case MOPS_ALIGNED_BLOCK_READ+ 0x10:
        case MOPS_ALIGNED_BLOCK_READ+ 0x20: case MOPS_ALIGNED_BLOCK_READ+ 0x30: {
            if (skip_block) break;
            const uint32_t count = op.flags >> MOPS_BLOCK_COUNT_SBITS;
            for (uint32_t i = 0; i < count; i++) emit_one_r(addr + i * 8, out + i);
            break;
        }
        case MOPS_BLOCK_WRITE        + 0x00: case MOPS_BLOCK_WRITE        + 0x10:
        case MOPS_BLOCK_WRITE        + 0x20: case MOPS_BLOCK_WRITE        + 0x30:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x00: case MOPS_ALIGNED_BLOCK_WRITE+ 0x10:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x20: case MOPS_ALIGNED_BLOCK_WRITE+ 0x30: {
            if (skip_block) break;
            const uint32_t count = op.flags >> MOPS_BLOCK_COUNT_SBITS;
            for (uint32_t i = 0; i < count; i++) emit_one_w(addr + i * 8, out + i);
            break;
        }
        default: break;
    }
}

__device__ __forceinline__
void block_reduce_counters(const ChunkCounters& my, ChunkCounters* g_dst) {
    __shared__ ChunkCounters s;
    if (threadIdx.x == 0) { s.full_5 = 0; s.full_3 = 0; s.full_2 = 0; s.read_byte = 0; s.write_byte = 0; }
    __syncthreads();
    if (my.full_5)     atomicAdd(&s.full_5,     my.full_5);
    if (my.full_3)     atomicAdd(&s.full_3,     my.full_3);
    if (my.full_2)     atomicAdd(&s.full_2,     my.full_2);
    if (my.read_byte)  atomicAdd(&s.read_byte,  my.read_byte);
    if (my.write_byte) atomicAdd(&s.write_byte, my.write_byte);
    __syncthreads();
    if (threadIdx.x == 0) {
        if (s.full_5)     atomicAdd(&g_dst->full_5,     s.full_5);
        if (s.full_3)     atomicAdd(&g_dst->full_3,     s.full_3);
        if (s.full_2)     atomicAdd(&g_dst->full_2,     s.full_2);
        if (s.read_byte)  atomicAdd(&g_dst->read_byte,  s.read_byte);
        if (s.write_byte) atomicAdd(&g_dst->write_byte, s.write_byte);
    }
}

__global__
void decode_count_kernel(const MemOp* __restrict__ memops,
                         uint32_t n_memops,
                         uint32_t* __restrict__ d_counts,
                         uint8_t* __restrict__ d_spill_status,
                         ChunkCounters* __restrict__ d_chunk_counters_entry,
                         BlockOpSpill* __restrict__ d_spill,
                         uint32_t* __restrict__ d_spill_count,
                         uint32_t* __restrict__ d_invalid_mode_flag) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    ChunkCounters my{0,0,0,0,0};
    if (i < n_memops) {
        MemOp op = memops[i];
        if (decode(op, &d_counts[i], my, d_invalid_mode_flag)) {
            const uint8_t mode = op.flags & 0x3Fu;
            const uint8_t base = mode & 0x0Fu;
            const bool is_block_read  = (base == (MOPS_BLOCK_READ  & 0x0Fu)) ||
                                        (base == (MOPS_ALIGNED_BLOCK_READ  & 0x0Fu));
            const bool is_block_write = (base == (MOPS_BLOCK_WRITE & 0x0Fu)) ||
                                        (base == (MOPS_ALIGNED_BLOCK_WRITE & 0x0Fu));
            if (is_block_read || is_block_write) {
                const uint32_t count = op.flags >> MOPS_BLOCK_COUNT_SBITS;
                if (count > BLOCKOP_SPILL_THRESH_VAL) {
                    uint32_t slot = atomicAdd(d_spill_count, 1u);
                    if (slot < MAX_BLOCKOP_SPILL_PER_CHUNK) {
                        BlockOpSpill s;
                        s.memop_idx    = i;
                        s.aligned_base = op.addr;
                        s.count        = count;
                        s.kind_w       = is_block_write ? 1u : 0u;
                        d_spill[slot] = s;
                        d_spill_status[i] = 1;
                    }
                }
            }
        }
    }
    block_reduce_counters(my, d_chunk_counters_entry);
}

__global__
void decode_emit_kernel(const MemOp* __restrict__ memops,
                        uint32_t n_memops,
                        const uint32_t* __restrict__ d_potential_offsets,
                        const uint8_t* __restrict__ d_spill_status,
                        PotentialEmit* __restrict__ d_potentials) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n_memops) return;
    MemOp op = memops[i];
    PotentialEmit* out_ptr = d_potentials + d_potential_offsets[i];
    decode_emit_inline(op, out_ptr, /*skip_block=*/d_spill_status[i] != 0);
}

__global__
void blockop_emit_kernel(const BlockOpSpill* __restrict__ d_spill,
                         const uint32_t* __restrict__ d_spill_count,
                         const uint32_t* __restrict__ d_potential_offsets,
                         PotentialEmit* __restrict__ d_potentials) {
    const uint32_t cap = min(*d_spill_count, MAX_BLOCKOP_SPILL_PER_CHUNK);
    if (blockIdx.x >= cap) return;
    const BlockOpSpill s = d_spill[blockIdx.x];
    const uint32_t base_addr   = s.aligned_base;
    const uint32_t count       = s.count;
    const uint32_t base_offset = d_potential_offsets[s.memop_idx];
    PotentialEmit* base = d_potentials + base_offset;
    const uint32_t kind_bit = (s.kind_w ? POT_FLAG_KIND_W : 0u);
    for (uint32_t i = threadIdx.x; i < count; i += blockDim.x) {
        const uint32_t a = base_addr + i * 8u;
        const uint32_t ram_bit = is_ram_addr(a) ? POT_FLAG_IS_RAM : 0u;
        base[i].aligned_addr_packed = a | kind_bit | ram_bit;
    }
}

__global__
void extract_sorted_addr_kernel(const uint64_t* __restrict__ d_sorted_keys,
                                uint32_t n_events,
                                uint32_t* __restrict__ d_sorted_addr) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n_events) return;
    d_sorted_addr[i] = (uint32_t)(d_sorted_keys[i] >> COMPACT_ADDR_SHIFT);
}

__global__
void extract_sorted_packed_kernel(const uint64_t* __restrict__ d_sorted_keys,
                                  uint32_t n_events,
                                  uint32_t* __restrict__ d_sorted_packed) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n_events) return;
    const uint64_t k = d_sorted_keys[i];
    const uint32_t orig_pos = (uint32_t)((k >> ORIG_POS_SHIFT) & ORIG_POS_MASK);
    const uint32_t kind_w_bit = (uint32_t)(k & 1ull);
    d_sorted_packed[i] = (kind_w_bit << 31) | orig_pos;
}

// =====================================================================
// PairSortGPU constants (those NOT in count_and_plan.cuh)
// =====================================================================

constexpr uint32_t N_ADDR_ROM   = 1u << 27;   // 128M
constexpr uint32_t N_ADDR_INPUT = 1u << 30;   // 1G
constexpr uint32_t N_ADDR_RAM   = 1u << 29;   // 512M
constexpr uint32_t N_ADDR = N_ADDR_ROM + N_ADDR_INPUT + N_ADDR_RAM;

constexpr uint32_t INSTANCE_SIZE[3]  = {1u << 21, 1u << 21, 1u << 22};
constexpr uint32_t INSTANCE_SIZE_MAX = 1u << 22;

constexpr uint8_t REGION_ROM            = 0;
constexpr uint8_t REGION_INPUT          = 1;
constexpr uint8_t REGION_RAM            = 2;
constexpr const char* REGION_NAME[3]    = {"ROM", "INPUT", "RAM"};
constexpr uint32_t REGION_ADDR_START[3] = {0, N_ADDR_ROM, N_ADDR_ROM + N_ADDR_INPUT};

// =====================================================================
// Address-region helpers
// Region-test order: RAM first, then ROM, INPUT as fall-through.
// =====================================================================

inline uint32_t compact_addr(uint32_t raw) {
    if (raw >= ZISK_RAM_ADDR_BASE)
        return ((raw - ZISK_RAM_ADDR_BASE) >> 3) + N_ADDR_ROM + N_ADDR_INPUT;
    if (raw >= ZISK_ROM_ADDR_BASE)
        return (raw - ZISK_ROM_ADDR_BASE) >> 3;
    return ((raw - ZISK_INPUT_ADDR_BASE) >> 3) + N_ADDR_ROM;
}

inline uint32_t expand_addr(uint32_t compact) {
    if (compact >= N_ADDR_ROM + N_ADDR_INPUT)
        return ((compact - N_ADDR_ROM - N_ADDR_INPUT) << 3) + ZISK_RAM_ADDR_BASE;
    if (compact < N_ADDR_ROM)
        return (compact << 3) + ZISK_ROM_ADDR_BASE;
    return ((compact - N_ADDR_ROM) << 3) + ZISK_INPUT_ADDR_BASE;
}

__device__ __forceinline__ uint32_t compact_addr_dev(uint32_t raw) {
    if (raw >= ZISK_RAM_ADDR_BASE)
        return ((raw - ZISK_RAM_ADDR_BASE) >> 3) + N_ADDR_ROM + N_ADDR_INPUT;
    if (raw >= ZISK_ROM_ADDR_BASE)
        return (raw - ZISK_ROM_ADDR_BASE) >> 3;
    return ((raw - ZISK_INPUT_ADDR_BASE) >> 3) + N_ADDR_ROM;
}

// =====================================================================
// Specialised kernels (variants of the generic mem_preprocess.cuh ones)
// =====================================================================

__global__ void add_const_kernel(uint32_t* arr, const uint32_t* d_offset, uint32_t n) {
    uint32_t off = *d_offset;
    uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) arr[i] += off;
}

__global__ void compact_kernel_with_shift(const PotentialEmit* __restrict__ d_potentials,
                                          const uint32_t* __restrict__ d_emit_bits,
                                          const uint32_t* __restrict__ d_final_offsets,
                                          uint32_t n_potentials,
                                          uint32_t* __restrict__ d_out) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n_potentials) return;
    if (d_emit_bits[i]) {
        const uint32_t raw = emit_aligned_addr(d_potentials[i]);
        d_out[d_final_offsets[i]] = compact_addr_dev(raw);
    }
}

__global__
void gather_ram_events_with_hist_kernel(const PotentialEmit* __restrict__ d_potentials,
                                        uint32_t n_potentials,
                                        uint64_t* __restrict__ d_ram_keys,
                                        uint32_t* __restrict__ d_ram_count,
                                        uint32_t* __restrict__ d_emit_bits,
                                        uint32_t* __restrict__ d_histogram,
                                        uint32_t* __restrict__ d_max_compact) {
    uint32_t local_max_rom   = 0;
    uint32_t local_max_input = 0;

    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n_potentials) {
        PotentialEmit p = d_potentials[i];
        if (emit_is_ram(p)) {
            const uint32_t compact_ram = ram_compact(emit_aligned_addr(p));
            const uint64_t key = ((uint64_t)compact_ram << COMPACT_ADDR_SHIFT)
                               | ((uint64_t)i           << ORIG_POS_SHIFT)
                               | (emit_kind_w(p) ? 1ull : 0ull);
            const uint32_t slot = atomicAdd(d_ram_count, 1u);
            d_ram_keys[slot] = key;
            d_emit_bits[i] = 0;
        } else {
            d_emit_bits[i] = 1;
            const uint32_t raw = emit_aligned_addr(p);
            const uint32_t compact = compact_addr_dev(raw);
            atomicAdd(&d_histogram[compact], 1u);
            if (compact < N_ADDR_ROM) {
                local_max_rom = compact;
            } else {
                local_max_input = compact - N_ADDR_ROM;
            }
        }
    }

    __shared__ uint32_t s_max[3];
    if (threadIdx.x < 3) s_max[threadIdx.x] = 0;
    __syncthreads();
    if (local_max_rom   > 0) atomicMax(&s_max[REGION_ROM],   local_max_rom);
    if (local_max_input > 0) atomicMax(&s_max[REGION_INPUT], local_max_input);
    __syncthreads();
    if (threadIdx.x < 2 && s_max[threadIdx.x] > 0)
        atomicMax(&d_max_compact[threadIdx.x], s_max[threadIdx.x]);
}

__global__
void state_machine_by_run_with_hist_kernel(const uint32_t* __restrict__ d_run_offsets,
                                           const uint32_t* __restrict__ d_num_unique,
                                           const uint32_t* __restrict__ d_sorted_vals,
                                           const uint32_t* __restrict__ d_sorted_addr,
                                           uint32_t* __restrict__ d_emit_bits,
                                           uint32_t* __restrict__ d_histogram,
                                           uint32_t* __restrict__ d_max_compact) {
    uint32_t local_max_ram = 0;

    const uint32_t t = blockIdx.x * blockDim.x + threadIdx.x;
    if (t < *d_num_unique) {
        const uint32_t start = d_run_offsets[t];
        const uint32_t end   = d_run_offsets[t + 1];

        bool state = false;
        uint32_t n_emit = 0;
        for (uint32_t j = start; j < end; j++) {
            const uint32_t v = d_sorted_vals[j];
            const bool kind_w       = (v >> 31);
            const uint32_t orig_pos = v & 0x7FFFFFFFu;
            uint32_t emit;
            if (kind_w) {
                emit = 1;
                state = true;
            } else {
                if (state) { emit = 0; state = false; }
                else       { emit = 1; state = true;  }
            }
            d_emit_bits[orig_pos] = emit;
            n_emit += emit;
        }
        if (n_emit > 0) {
            const uint32_t compact_ram = d_sorted_addr[start];
            atomicAdd(&d_histogram[compact_ram + N_ADDR_ROM + N_ADDR_INPUT], n_emit);
            local_max_ram = compact_ram;
        }
    }

    __shared__ uint32_t s_max_ram;
    if (threadIdx.x == 0) s_max_ram = 0;
    __syncthreads();
    if (local_max_ram > 0) atomicMax(&s_max_ram, local_max_ram);
    __syncthreads();
    if (threadIdx.x == 0 && s_max_ram > 0)
        atomicMax(&d_max_compact[REGION_RAM], s_max_ram);
}

// =====================================================================
// PairSortGPU kernels (verbatim from main_real.cu)
// =====================================================================

__global__ void instance_boundaries_kernel(
    const uint32_t* prefix,
    uint32_t prefix_base_addr, uint32_t num_addr_region,
    uint32_t num_ops_region,
    uint32_t instance_size,
    const uint32_t* active_ids,
    uint32_t* active_first, uint32_t* active_last,
    uint32_t* offset_starts,
    uint32_t num_active)
{
    uint32_t idx = threadIdx.x;
    if (idx >= num_active) return;

    uint32_t local_inst  = active_ids[idx];
    uint32_t region_start = prefix[prefix_base_addr];
    uint32_t base_pos    = region_start + local_inst * instance_size;
    uint32_t inst_size   = min(instance_size, num_ops_region - local_inst * instance_size);
    uint32_t inst_start  = (local_inst == 0) ? base_pos : base_pos - 1;
    uint32_t inst_end    = base_pos + inst_size;

    uint32_t lo = prefix_base_addr, hi = prefix_base_addr + num_addr_region;
    while (lo < hi) {
        uint32_t mid = lo + (hi - lo + 1) / 2;
        if (prefix[mid] <= inst_start) lo = mid;
        else hi = mid - 1;
    }
    active_first[idx] = lo;

    lo = active_first[idx];
    hi = prefix_base_addr + num_addr_region;
    while (lo < hi) {
        uint32_t mid = lo + (hi - lo + 1) / 2;
        if (prefix[mid] < inst_end) lo = mid;
        else hi = mid - 1;
    }

    {
        uint32_t max_prefix = prefix[lo + 1];
        uint32_t tlo = active_first[idx], thi = lo;
        while (tlo < thi) {
            uint32_t mid = tlo + (thi - tlo + 1) / 2;
            if (prefix[mid] < max_prefix) tlo = mid;
            else thi = mid - 1;
        }
        lo = tlo;
    }
    active_last[idx] = lo;

    __syncthreads();
    if (idx == 0) {
        uint32_t offset = 0;
        for (uint32_t i = 0; i < num_active; i++) {
            offset_starts[i] = offset;
            offset += active_last[i] - active_first[i] + 1;
        }
    }
}

__global__ void chunk_fml_count_gappy_kernel(
    const uint32_t* __restrict__ ops,
    const uint32_t* __restrict__ chunk_starts,
    const uint32_t* __restrict__ packed_chunk_offsets,
    const uint32_t* __restrict__ active_first,
    const uint32_t* __restrict__ active_last,
    uint32_t* __restrict__ d_fml,
    uint32_t num_active, uint32_t num_chunks, uint32_t total_valid_ops)
{
    __shared__ uint32_t s_first[MAX_INSTANCES];
    __shared__ uint32_t s_last[MAX_INSTANCES];
    if (threadIdx.x < num_active) {
        s_first[threadIdx.x] = active_first[threadIdx.x];
        s_last[threadIdx.x]  = active_last[threadIdx.x];
    }
    __syncthreads();

    uint32_t i        = blockIdx.x * blockDim.x + threadIdx.x;
    uint32_t stride   = gridDim.x * blockDim.x;
    uint32_t lane     = threadIdx.x & 31;
    uint32_t chunk_id = 0;

    for (; i < total_valid_ops; i += stride) {
        while (chunk_id + 1 < num_chunks && i >= packed_chunk_offsets[chunk_id + 1])
            chunk_id++;
        const uint32_t off_in_chunk = i - packed_chunk_offsets[chunk_id];
        const uint32_t addr         = ops[chunk_starts[chunk_id] + off_in_chunk];

        uint32_t warp_chunk_first = __shfl_sync(0xFFFFFFFFu, chunk_id, 0);
        uint32_t warp_chunk_last  = __shfl_sync(0xFFFFFFFFu, chunk_id, 31);
        bool same_chunk = (warp_chunk_first == warp_chunk_last);

        for (uint32_t ai = 0; ai < num_active; ai++) {
            uint32_t fa = s_first[ai];
            uint32_t la = s_last[ai];

            if (__all_sync(0xFFFFFFFFu, addr < fa)) break;

            bool in_range = (addr >= fa && addr <= la);
            if (!__any_sync(0xFFFFFFFFu, in_range)) continue;

            uint32_t cat = 3;
            if (in_range) {
                if (addr == fa)      cat = 0;
                else if (addr == la) cat = 2;
                else                 cat = 1;
            }

            if (same_chunk) {
                for (uint32_t c = 0; c < 3; c++) {
                    unsigned mask = __ballot_sync(0xFFFFFFFFu, cat == c);
                    if (mask && lane == 0)
                        atomicAdd(&d_fml[(ai * num_chunks + chunk_id) * 3 + c], __popc(mask));
                }
            } else if (in_range) {
                atomicAdd(&d_fml[(ai * num_chunks + chunk_id) * 3 + cat], 1);
            }
        }
    }
}

__global__ void build_metas_kernel(
    const uint32_t* d_fml,
    const uint32_t* prefix,
    uint32_t prefix_base_addr, uint32_t num_ops_region,
    uint32_t instance_size,
    const uint32_t* active_ids,
    const uint32_t* active_first, const uint32_t* active_last,
    uint32_t* result_nops,
    uint32_t* meta_scalars,
    uint32_t num_active, uint32_t num_chunks)
{
    const uint32_t ai = blockIdx.x;
    if (ai >= num_active) return;
    const uint32_t tid = threadIdx.x;
    const uint32_t nthreads = blockDim.x;

    __shared__ uint32_t s_total_compacted;
    __shared__ uint32_t s_first_addr_total_skip;
    __shared__ uint32_t s_last_addr_total_include;
    __shared__ bool     s_single_addr;
    __shared__ uint32_t s_fa_chunk, s_fa_skip, s_la_chunk, s_la_include;
    __shared__ uint32_t s_scan[256];

    const uint32_t* fml_base = d_fml + (size_t)ai * num_chunks * 3;
    uint32_t* scratch = result_nops + (size_t)ai * num_chunks;

    uint32_t chunks_per_thread = (num_chunks + nthreads - 1) / nthreads;
    uint32_t c_start = tid * chunks_per_thread;
    uint32_t c_end   = min(c_start + chunks_per_thread, num_chunks);

    uint32_t my_count = 0;
    for (uint32_t c = c_start; c < c_end; c++) {
        uint32_t base = c * 3;
        if (fml_base[base] + fml_base[base + 1] + fml_base[base + 2] > 0)
            my_count++;
    }

    s_scan[tid] = my_count;
    __syncthreads();
    if (tid == 0) {
        uint32_t total = 0;
        for (uint32_t i = 0; i < nthreads; i++) {
            uint32_t val = s_scan[i];
            s_scan[i] = total;
            total += val;
        }
        s_total_compacted = total;
    }
    __syncthreads();

    uint32_t write_pos = s_scan[tid];
    for (uint32_t c = c_start; c < c_end; c++) {
        uint32_t base = c * 3;
        if (fml_base[base] + fml_base[base + 1] + fml_base[base + 2] > 0)
            scratch[write_pos++] = c;
    }
    __syncthreads();

    uint32_t nc = s_total_compacted;

    if (tid == 0) {
        uint32_t fa = active_first[ai];
        uint32_t la = active_last[ai];
        bool single_addr   = (fa == la);
        uint32_t num_addrs = la - fa + 1;

        s_single_addr = single_addr;

        uint32_t local_inst   = active_ids[ai];
        uint32_t region_start = prefix[prefix_base_addr];
        uint32_t base_pos     = region_start + local_inst * instance_size;
        uint32_t inst_size    = min(instance_size, num_ops_region - local_inst * instance_size);
        uint32_t halo_base    = (local_inst == 0) ? base_pos : base_pos - 1;
        s_first_addr_total_skip = halo_base - prefix[fa];

        if (!single_addr) {
            uint32_t filled_before_last = prefix[fa + num_addrs - 1] - base_pos;
            s_last_addr_total_include = inst_size - filled_before_last;
        } else {
            s_last_addr_total_include = inst_size;
            if (halo_base != base_pos) s_last_addr_total_include++;
        }
    }
    __syncthreads();

    uint32_t first_addr_total_skip    = s_first_addr_total_skip;
    uint32_t last_addr_total_include  = s_last_addr_total_include;
    bool single_addr = s_single_addr;

    uint32_t nc_per_thread = (nc + nthreads - 1) / nthreads;
    uint32_t ci_start = tid * nc_per_thread;
    uint32_t ci_end   = min(ci_start + nc_per_thread, nc);

    uint32_t my_count_first = 0;
    for (uint32_t ci = ci_start; ci < ci_end; ci++)
        my_count_first += fml_base[scratch[ci] * 3 + 0];

    s_scan[tid] = my_count_first;
    __syncthreads();

    if (tid == 0) {
        uint32_t cum = 0;
        for (uint32_t t = 0; t < nthreads; t++) {
            if (cum + s_scan[t] > first_addr_total_skip) {
                uint32_t t_start = t * nc_per_thread;
                uint32_t t_end   = min(t_start + nc_per_thread, nc);
                uint32_t local_cum = cum;
                for (uint32_t ci = t_start; ci < t_end; ci++) {
                    uint32_t cf = fml_base[scratch[ci] * 3 + 0];
                    if (local_cum + cf > first_addr_total_skip) {
                        s_fa_chunk = scratch[ci];
                        s_fa_skip  = first_addr_total_skip - local_cum;
                        break;
                    }
                    local_cum += cf;
                }
                break;
            }
            cum += s_scan[t];
        }
    }
    __syncthreads();

    uint32_t la_cat = single_addr ? 0 : 2;
    uint32_t la_threshold = single_addr
        ? (first_addr_total_skip + last_addr_total_include)
        : last_addr_total_include;

    uint32_t my_cum_last = 0;
    for (uint32_t ci = ci_start; ci < ci_end; ci++)
        my_cum_last += fml_base[scratch[ci] * 3 + la_cat];

    s_scan[tid] = my_cum_last;
    __syncthreads();

    if (tid == 0) {
        uint32_t cum = 0;
        for (uint32_t t = 0; t < nthreads; t++) {
            if (cum + s_scan[t] >= la_threshold) {
                uint32_t t_start = t * nc_per_thread;
                uint32_t t_end   = min(t_start + nc_per_thread, nc);
                uint32_t local_cum = cum;
                for (uint32_t ci = t_start; ci < t_end; ci++) {
                    uint32_t cv = fml_base[scratch[ci] * 3 + la_cat];
                    if (local_cum + cv >= la_threshold) {
                        s_la_chunk   = scratch[ci];
                        s_la_include = la_threshold - local_cum;
                        break;
                    }
                    local_cum += cv;
                }
                break;
            }
            cum += s_scan[t];
        }
    }
    __syncthreads();

    uint32_t fa_chunk   = s_fa_chunk;
    uint32_t fa_skip    = s_fa_skip;
    uint32_t la_chunk   = s_la_chunk;
    uint32_t la_include = s_la_include;

    uint32_t* out_nops = result_nops + (size_t)ai * num_chunks;
    for (uint32_t c = tid; c < num_chunks; c += nthreads)
        out_nops[c] = 0;
    __syncthreads();

    for (uint32_t c = c_start; c < c_end; c++) {
        uint32_t base = c * 3;
        uint32_t cf = fml_base[base + 0];
        uint32_t cm = fml_base[base + 1];
        uint32_t cl = fml_base[base + 2];
        if (cf + cm + cl == 0) continue;

        bool needed = (cm > 0);
        if (cf > 0) {
            if (single_addr) {
                if (c >= fa_chunk && c <= la_chunk) needed = true;
            } else {
                if (c >= fa_chunk) needed = true;
            }
        }
        if (cl > 0 && c <= la_chunk)
            needed = true;
        if (needed)
            out_nops[c] = cf + cm + cl;
    }

    if (tid == 0) {
        uint32_t* out = meta_scalars + ai * 4;
        out[0] = fa_chunk;
        out[1] = fa_skip;
        out[2] = la_chunk;
        out[3] = la_include;
    }
}

__global__ void compute_addr_offsets_kernel(
    const uint32_t* prefix,
    uint32_t prefix_base_addr, uint32_t num_ops_region,
    uint32_t instance_size,
    const uint32_t* active_ids,
    const uint32_t* active_first, const uint32_t* active_last,
    uint32_t* addr_offsets, const uint32_t* offset_starts,
    uint32_t num_active)
{
    uint32_t ai = blockIdx.x;
    if (ai >= num_active) return;

    uint32_t fa = active_first[ai];
    uint32_t la = active_last[ai];
    uint32_t num_addrs = la - fa + 1;

    uint32_t local_inst   = active_ids[ai];
    uint32_t region_start = prefix[prefix_base_addr];
    uint32_t base_pos     = region_start + local_inst * instance_size;
    uint32_t halo_base    = (local_inst == 0) ? base_pos : base_pos - 1;

    uint32_t* out = addr_offsets + offset_starts[ai];
    uint32_t tid = threadIdx.x;
    uint32_t stride = blockDim.x;

    if (tid == 0)
        out[0] = (halo_base == base_pos) ? 1 : 0;

    for (uint32_t j = tid + 1; j < num_addrs; j += stride)
        out[j] = prefix[fa + j] - (base_pos - 1);
}

// =====================================================================
// Binary save helpers (declared in count_and_plan.cuh)
// =====================================================================

FILE* save_metas_begin(const std::string& path) {
    FILE* f = std::fopen(path.c_str(), "wb");
    if (!f) { std::cerr << "ERROR: open " << path << " for write" << std::endl; std::exit(1); }
    uint32_t placeholder = 0;
    if (std::fwrite(&placeholder, sizeof(uint32_t), 1, f) != 1) {
        std::cerr << "ERROR: short write " << path << std::endl; std::exit(1);
    }
    return f;
}

void save_metas_append(FILE* f, const InstanceMeta& m) {
    auto wr = [&](const void* p, size_t bytes) {
        if (std::fwrite(p, 1, bytes, f) != bytes) {
            std::cerr << "ERROR: short write" << std::endl; std::exit(1);
        }
    };
    uint32_t cps = m.n_chunks;
    uint32_t aos = m.addr_offsets_size;
    wr(&m.inst_id,            sizeof(uint32_t));
    wr(&m.kind,               sizeof(uint32_t));
    wr(&m.first_addr,         sizeof(uint32_t));
    wr(&m.last_addr,          sizeof(uint32_t));
    wr(&m.first_addr_chunk,   sizeof(uint32_t));
    wr(&m.first_addr_skip,    sizeof(uint32_t));
    wr(&m.last_addr_chunk,    sizeof(uint32_t));
    wr(&m.last_addr_include,  sizeof(uint32_t));
    wr(&cps, sizeof(uint32_t));
    wr(&aos, sizeof(uint32_t));
    wr(m.count_per_chunk, cps * sizeof(uint32_t));
    wr(m.addr_offsets,    aos * sizeof(uint32_t));
}

void save_metas_end(FILE* f, uint32_t total) {
    if (std::fseek(f, 0, SEEK_SET) != 0) {
        std::cerr << "ERROR: seek failed" << std::endl; std::exit(1);
    }
    if (std::fwrite(&total, sizeof(uint32_t), 1, f) != 1) {
        std::cerr << "ERROR: short write of header count" << std::endl; std::exit(1);
    }
    std::fclose(f);
}

// =====================================================================
// CountAndPlan — member-function bodies
// =====================================================================

CountAndPlan::CountAndPlan()
    : h_active_first_(MAX_INSTANCES),
      h_active_last_(MAX_INSTANCES),
      metas_(MAX_INSTANCES)
{}

CountAndPlan::~CountAndPlan() { free_all_(); }

bool CountAndPlan::setup(void* d_buf, size_t bytes,
                         uint32_t n_workers, uint32_t worker_id) {
    free_all_();

    if (n_workers == 0 || worker_id >= n_workers) {
        fprintf(stderr,
                "CountAndPlan::setup ERROR: invalid n_workers=%u worker_id=%u\n",
                n_workers, worker_id);
        return false;
    }
    n_workers_  = n_workers;
    worker_id_  = worker_id;
    max_active_ = (MAX_INSTANCES + n_workers - 1) / n_workers;

    size_t scan_counts_b, scan_emit_b, scan_runs_b, sort_b, rle_b, hist_scan_b;
    query_cub_sizes_(scan_counts_b, scan_emit_b, scan_runs_b, sort_b, rle_b, hist_scan_b);
    cub_temp_bytes_    = std::max({scan_counts_b, scan_emit_b, scan_runs_b, sort_b, rle_b});
    d_temp_hist_bytes_ = hist_scan_b;

    auto fixed_bytes = [&]() -> size_t {
        size_t cur = 0;
        auto take = [&](size_t b) { cur = (cur + 255) & ~(size_t)255; cur += b; };
        take((size_t)N_ADDR     * 4);
        take(((size_t)N_ADDR + 1) * 4);
        take(d_temp_hist_bytes_);
        take((size_t)max_active_ * 4);
        take((size_t)max_active_ * 4);
        take((size_t)max_active_ * 4);
        take((size_t)max_active_ * MAX_CHUNKS * 3 * 4);
        take((size_t)max_active_ * MAX_CHUNKS * 4);
        take((size_t)max_active_ * 4 * 4);
        take(((size_t)N_ADDR + max_active_) * 4);
        take((size_t)max_active_ * 4);
        take(3 * 4);
        take(4);
        take(sizeof(ChunkCounters));
        take((size_t)MAX_CHUNKS * 4);
        take((size_t)MAX_CHUNKS * 4);
        take(((size_t)MAX_CHUNKS + 1) * 4);
        for (int s = 0; s < N_STREAMS; s++) {
            take((size_t)MAX_POT_PER_CHUNK    * sizeof(PotentialEmit));
            take((size_t)MAX_POT_PER_CHUNK    * 4);
            take(((size_t)MAX_POT_PER_CHUNK + 1) * 4);
            take((size_t)MAX_MEMOPS_PER_CHUNK * sizeof(MemOp));
            take(((size_t)MAX_MEMOPS_PER_CHUNK + 1) * 4);
            take(((size_t)MAX_MEMOPS_PER_CHUNK + 1) * 4);
            take((size_t)MAX_BLOCKOP_SPILL_PER_CHUNK * sizeof(BlockOpSpill));
            take(4);
            take((size_t)MAX_MEMOPS_PER_CHUNK);
            take((size_t)MAX_POT_PER_CHUNK * 8);
            take((size_t)MAX_POT_PER_CHUNK * 8);
            take((size_t)MAX_POT_PER_CHUNK * 4);
            take((size_t)MAX_POT_PER_CHUNK * 4);
            take((size_t)MAX_POT_PER_CHUNK * 4);
            take(((size_t)MAX_POT_PER_CHUNK + 1) * 4);
            take(4);
            take(4);
            take(cub_temp_bytes_);
        }
        cur = (cur + 255) & ~(size_t)255;       // mirror the final round-up below
        return cur;
    }();

    if (d_buf != nullptr && bytes > 0) {
        if (bytes < fixed_bytes) {
            fprintf(stderr,
                    "CountAndPlan::setup ERROR: caller buffer is %zu bytes, need at least "
                    "%zu for fixed regions (MAX_CHUNKS=%u, MAX_MEMOPS_PER_CHUNK=%u)\n",
                    bytes, fixed_bytes, MAX_CHUNKS, MAX_MEMOPS_PER_CHUNK);
            return false;
        }
        arena_       = (uint8_t*)d_buf;
        arena_bytes_ = bytes;
        arena_owned_ = false;
    } else {
        size_t want = fixed_bytes + ((size_t)2 << 30);
        cudaError_t err = cudaMalloc(&arena_, want);
        if (err != cudaSuccess) {
            err = cudaMalloc(&arena_, fixed_bytes);
            if (err != cudaSuccess) {
                fprintf(stderr,
                        "CountAndPlan::setup ERROR: cudaMalloc failed for %zu bytes: %s\n",
                        fixed_bytes, cudaGetErrorString(err));
                return false;
            }
            arena_bytes_ = fixed_bytes;
        } else {
            arena_bytes_ = want;
        }
        arena_owned_ = true;
    }

    cursor_ = 0;
    auto take = [&](size_t b) -> uint8_t* {
        cursor_ = (cursor_ + 255) & ~(size_t)255;
        uint8_t* p = arena_ + cursor_;
        cursor_ += b;
        return p;
    };

    d_histogram_              = (uint32_t*)take((size_t)N_ADDR * 4);
    d_prefix_                 = (uint32_t*)take(((size_t)N_ADDR + 1) * 4);
    d_temp_hist_              = (void*)    take(d_temp_hist_bytes_);
    d_active_ids_             = (uint32_t*)take((size_t)max_active_ * 4);
    d_active_first_           = (uint32_t*)take((size_t)max_active_ * 4);
    d_active_last_            = (uint32_t*)take((size_t)max_active_ * 4);
    d_fml_                    = (uint32_t*)take((size_t)max_active_ * MAX_CHUNKS * 3 * 4);
    d_result_nops_            = (uint32_t*)take((size_t)max_active_ * MAX_CHUNKS * 4);
    d_meta_scalars_           = (uint32_t*)take((size_t)max_active_ * 4 * 4);
    d_addr_offsets_           = (uint32_t*)take(((size_t)N_ADDR + max_active_) * 4);
    d_offset_starts_          = (uint32_t*)take((size_t)max_active_ * 4);
    d_max_compact_            = (uint32_t*)take(3 * 4);
    d_invalid_mode_flag_        = (uint32_t*)take(4);
    d_chunk_counters_per_chunk_ = (ChunkCounters*)take((size_t)MAX_CHUNKS * sizeof(ChunkCounters));
    d_gappy_offsets_          = (uint32_t*)take((size_t)MAX_CHUNKS * 4);
    d_chunk_lens_             = (uint32_t*)take((size_t)MAX_CHUNKS * 4);
    d_packed_chunk_offsets_   = (uint32_t*)take(((size_t)MAX_CHUNKS + 1) * 4);

    for (int s = 0; s < N_STREAMS; s++) {
        d_potentials_[s]        = (PotentialEmit*)take((size_t)MAX_POT_PER_CHUNK    * sizeof(PotentialEmit));
        d_emit_bits_[s]         = (uint32_t*)     take((size_t)MAX_POT_PER_CHUNK    * 4);
        d_final_offsets_[s]     = (uint32_t*)     take(((size_t)MAX_POT_PER_CHUNK + 1) * 4);
        d_memops_[s]            = (MemOp*)        take((size_t)MAX_MEMOPS_PER_CHUNK * sizeof(MemOp));
        d_counts_[s]            = (uint32_t*)     take(((size_t)MAX_MEMOPS_PER_CHUNK + 1) * 4);
        d_potential_offsets_[s] = (uint32_t*)     take(((size_t)MAX_MEMOPS_PER_CHUNK + 1) * 4);
        d_spill_[s]             = (BlockOpSpill*) take((size_t)MAX_BLOCKOP_SPILL_PER_CHUNK * sizeof(BlockOpSpill));
        d_spill_count_[s]       = (uint32_t*)     take(4);
        d_spill_status_[s]      = (uint8_t*)      take((size_t)MAX_MEMOPS_PER_CHUNK);
        d_ram_keys_[s]          = (uint64_t*)     take((size_t)MAX_POT_PER_CHUNK * 8);
        d_ram_keys_sorted_[s]   = (uint64_t*)     take((size_t)MAX_POT_PER_CHUNK * 8);
        d_sorted_addr_[s]       = (uint32_t*)     take((size_t)MAX_POT_PER_CHUNK * 4);
        d_ram_vals_sorted_[s]   = (uint32_t*)     take((size_t)MAX_POT_PER_CHUNK * 4);
        d_run_lengths_[s]       = (uint32_t*)     take((size_t)MAX_POT_PER_CHUNK * 4);
        d_run_offsets_[s]       = (uint32_t*)     take(((size_t)MAX_POT_PER_CHUNK + 1) * 4);
        d_num_unique_[s]        = (uint32_t*)     take(4);
        d_ram_count_[s]         = (uint32_t*)     take(4);
        d_cub_temp_[s]          = (void*)         take(cub_temp_bytes_);
    }

    cursor_ = (cursor_ + 255) & ~(size_t)255;
    if (cursor_ > arena_bytes_) {
        fprintf(stderr, "CountAndPlan::setup INTERNAL ERROR: cursor %zu > arena %zu\n",
                cursor_, arena_bytes_);
        return false;
    }
    d_ops_pool_          = (uint32_t*)(arena_ + cursor_);
    d_ops_pool_cap_u32_  = (arena_bytes_ - cursor_) / 4;
    d_ops_pool_used_u32_ = 0;

    for (int s = 0; s < N_STREAMS; s++)
        CUDA_CHECK(cudaStreamCreate(&streams_[s]));
    CUDA_CHECK(cudaStreamCreate(&d2h_stream_));
    CUDA_CHECK(cudaStreamCreate(&meta_stream_));
    CUDA_CHECK(cudaEventCreate(&e_last_chunk_start_));
    CUDA_CHECK(cudaEventCreate(&e_after_preproc_));
    CUDA_CHECK(cudaEventCreate(&e_after_prepare_));
    CUDA_CHECK(cudaEventCreate(&e_metas_ready_));

    CUDA_CHECK(cudaMallocHost(&h_memops_,
        (size_t)MAX_TOTAL_MEMOPS * sizeof(MemOp)));
    CUDA_CHECK(cudaMallocHost(&h_n_emits_all_, (size_t)MAX_CHUNKS * 4));
    CUDA_CHECK(cudaMallocHost(&h_result_nops_, (size_t)max_active_ * MAX_CHUNKS * 4));
    CUDA_CHECK(cudaMallocHost(&h_meta_scalars_, (size_t)max_active_ * 4 * 4));
    CUDA_CHECK(cudaMallocHost(&h_chunk_counters_per_chunk_,
        (size_t)MAX_CHUNKS * sizeof(ChunkCounters)));
    h_offsets_buf_size_ = 1ull << 30;
    CUDA_CHECK(cudaMallocHost(&h_offsets_buf_, h_offsets_buf_size_));
    for (int s = 0; s < N_STREAMS; s++)
        CUDA_CHECK(cudaMallocHost(&h_n_emits_[s], sizeof(uint32_t)));

    reset();
    return true;
}

bool CountAndPlan::add_chunk(const MemOp* memops, uint32_t n) {
    if (n_chunks_ >= MAX_CHUNKS) {
        fprintf(stderr,
                "CountAndPlan::add_chunk ERROR: MAX_CHUNKS=%u exceeded\n", MAX_CHUNKS);
        std::abort();
    }
    if (n > MAX_MEMOPS_PER_CHUNK) {
        fprintf(stderr,
                "CountAndPlan::add_chunk FATAL: chunk has %u memops > "
                "MAX_MEMOPS_PER_CHUNK=%u (per-stream reception region too small)\n",
                n, MAX_MEMOPS_PER_CHUNK);
        std::abort();
    }
    if (h_memops_used_ + n > MAX_TOTAL_MEMOPS) {
        fprintf(stderr,
                "CountAndPlan::add_chunk FATAL: pinned memop pool exhausted "
                "(used %zu + need %u > MAX_TOTAL_MEMOPS=%u)\n",
                h_memops_used_, n, MAX_TOTAL_MEMOPS);
        std::abort();
    }

    const uint32_t c = n_chunks_;
    const int      s = c % N_STREAMS;

    //add potencial emission
    auto add_pot = [](uint32_t addr, uint32_t count, size_t& pot, uint32_t& ram) {
        pot += count;
        if (is_ram_addr(addr)) ram += count;
    };
    size_t   pot = 0;
    uint32_t ram = 0;
    for (uint32_t k = 0; k < n; k++) {
        const MemOp& op = memops[k];
        const uint32_t addr    = op.addr;
        const uint32_t aligned = addr & ZISK_ALIGN_MASK;
        const uint8_t  mode    = op.flags & 0x3Fu;
        const uint32_t off     = addr & 0x07u;
        switch (mode) {
            case MOPS_READ_1:                                add_pot(aligned, 1, pot, ram); break;
            case MOPS_CWRITE_1: case MOPS_WRITE_1:           add_pot(aligned, 2, pot, ram); break;
            case MOPS_READ_2:   add_pot(aligned, 1, pot, ram); if (off > 6) add_pot(aligned + 8, 1, pot, ram); break;
            case MOPS_WRITE_2:  add_pot(aligned, 2, pot, ram); if (off > 6) add_pot(aligned + 8, 2, pot, ram); break;
            case MOPS_READ_4:   add_pot(aligned, 1, pot, ram); if (off > 4) add_pot(aligned + 8, 1, pot, ram); break;
            case MOPS_WRITE_4:  add_pot(aligned, 2, pot, ram); if (off > 4) add_pot(aligned + 8, 2, pot, ram); break;
            case MOPS_READ_8:   add_pot(aligned, 1, pot, ram); if (off > 0) add_pot(aligned + 8, 1, pot, ram); break;
            case MOPS_WRITE_8:  if (addr == aligned) add_pot(aligned, 1, pot, ram);
                                else { add_pot(aligned, 2, pot, ram); add_pot(aligned + 8, 2, pot, ram); } break;
            case MOPS_ALIGNED_READ  + 0x00: case MOPS_ALIGNED_READ  + 0x10:
            case MOPS_ALIGNED_READ  + 0x20: case MOPS_ALIGNED_READ  + 0x30:
            case MOPS_ALIGNED_WRITE + 0x00: case MOPS_ALIGNED_WRITE + 0x10:
            case MOPS_ALIGNED_WRITE + 0x20: case MOPS_ALIGNED_WRITE + 0x30:
                add_pot(addr, 1, pot, ram); break;
            default: { uint32_t cnt = op.flags >> MOPS_BLOCK_COUNT_SBITS;
                       add_pot(addr, cnt, pot, ram); break; }
        }
    }

    if (pot > MAX_POT_PER_CHUNK) {
        fprintf(stderr,
                "CountAndPlan::add_chunk ERROR: chunk %u has %zu potentials > "
                "MAX_POT_PER_CHUNK=%u\n", c, pot, MAX_POT_PER_CHUNK);
        return false;
    }
    if (d_ops_pool_used_u32_ + pot > d_ops_pool_cap_u32_) {
        fprintf(stderr,
                "CountAndPlan::add_chunk ERROR: ops pool exhausted at chunk %u "
                "(used %zu + need %zu > capacity %zu u32 entries). "
                "Increase the buffer size passed to setup().\n",
                c, d_ops_pool_used_u32_, pot, d_ops_pool_cap_u32_);
        return false;
    }

    n_potentials_per_chunk_.push_back((uint32_t)pot);
    n_ram_per_chunk_.push_back(ram);
    out_offsets_.push_back(out_offsets_.back() + pot);
    d_ops_pool_used_u32_ += pot;

    MemOp* memops_dst = h_memops_ + h_memops_used_;
    std::memcpy(memops_dst, memops, n * sizeof(MemOp));
    h_memops_used_ += n;

    cudaStream_t st = streams_[s];

    CUDA_CHECK(cudaEventRecord(e_last_chunk_start_, st));

    uint32_t* d_chunk_out = d_ops_pool_ + out_offsets_[c];

    if (n == 0) {
        *(h_n_emits_[s]) = 0;
        n_chunks_++;
        return true;
    }

    constexpr int BLOCK = 256;
    const int g_memops = (n + BLOCK - 1) / BLOCK;
    const int g_pot    = ((uint32_t)pot + BLOCK - 1) / BLOCK;
    const int g_ram    = ram == 0 ? 0 : (int)((ram + BLOCK - 1) / BLOCK);

    CUDA_CHECK(cudaMemcpyAsync(d_memops_[s], memops_dst,
                               sizeof(MemOp) * n, cudaMemcpyHostToDevice, st));
    CUDA_CHECK(cudaMemsetAsync(d_ram_count_[s],    0, 4, st));
    CUDA_CHECK(cudaMemsetAsync(d_spill_count_[s],  0, 4, st));
    CUDA_CHECK(cudaMemsetAsync(d_spill_status_[s], 0, n, st));

    decode_count_kernel<<<g_memops, BLOCK, 0, st>>>(
        d_memops_[s], n, d_counts_[s], d_spill_status_[s],
        &d_chunk_counters_per_chunk_[c],
        d_spill_[s], d_spill_count_[s],
        d_invalid_mode_flag_);

    {
        size_t bytes = cub_temp_bytes_;
        cub::DeviceScan::ExclusiveSum(d_cub_temp_[s], bytes,
            d_counts_[s], d_potential_offsets_[s], n + 1, st);
    }

    decode_emit_kernel<<<g_memops, BLOCK, 0, st>>>(
        d_memops_[s], n, d_potential_offsets_[s], d_spill_status_[s], d_potentials_[s]);

    blockop_emit_kernel<<<MAX_BLOCKOP_SPILL_PER_CHUNK, 256, 0, st>>>(
        d_spill_[s], d_spill_count_[s], d_potential_offsets_[s], d_potentials_[s]);

    gather_ram_events_with_hist_kernel<<<g_pot, BLOCK, 0, st>>>(
        d_potentials_[s], (uint32_t)pot,
        d_ram_keys_[s], d_ram_count_[s], d_emit_bits_[s],
        d_histogram_, d_max_compact_);

    if (ram > 0) {
        size_t bytes_sort = cub_temp_bytes_;
        cub::DeviceRadixSort::SortKeys(d_cub_temp_[s], bytes_sort,
            d_ram_keys_[s], d_ram_keys_sorted_[s], ram, 0, RAM_KEY_END_BIT, st);

        extract_sorted_addr_kernel<<<g_ram, BLOCK, 0, st>>>(
            d_ram_keys_sorted_[s], ram, d_sorted_addr_[s]);

        extract_sorted_packed_kernel<<<g_ram, BLOCK, 0, st>>>(
            d_ram_keys_sorted_[s], ram, d_ram_vals_sorted_[s]);

        size_t bytes_rle = cub_temp_bytes_;
        cub::DeviceRunLengthEncode::Encode(d_cub_temp_[s], bytes_rle,
            d_sorted_addr_[s], thrust::discard_iterator<>{},
            d_run_lengths_[s], d_num_unique_[s], ram, st);

        size_t bytes_sr = cub_temp_bytes_;
        cub::DeviceScan::ExclusiveSum(d_cub_temp_[s], bytes_sr,
            d_run_lengths_[s], d_run_offsets_[s], ram + 1, st);

        state_machine_by_run_with_hist_kernel<<<g_ram, BLOCK, 0, st>>>(
            d_run_offsets_[s], d_num_unique_[s], d_ram_vals_sorted_[s],
            d_sorted_addr_[s], d_emit_bits_[s], d_histogram_, d_max_compact_);
    }

    {
        size_t bytes = cub_temp_bytes_;
        cub::DeviceScan::ExclusiveSum(d_cub_temp_[s], bytes,
            d_emit_bits_[s], d_final_offsets_[s], (uint32_t)pot + 1, st);
    }

    CUDA_CHECK(cudaMemcpyAsync(&h_n_emits_all_[c],
        d_final_offsets_[s] + pot, 4, cudaMemcpyDeviceToHost, st));

    compact_kernel_with_shift<<<g_pot, BLOCK, 0, st>>>(
        d_potentials_[s], d_emit_bits_[s], d_final_offsets_[s],
        (uint32_t)pot, d_chunk_out);

    n_chunks_++;
    return true;
}

bool CountAndPlan::run(InstanceMeta** metas_out, uint32_t& n_metas) {
    if (n_chunks_ == 0) {
        fprintf(stderr, "CountAndPlan::run ERROR: no chunks added\n");
        return false;
    }

    if (!preprocessed_) {
        for (int s = 0; s < N_STREAMS; s++)
            CUDA_CHECK(cudaStreamSynchronize(streams_[s]));
        CUDA_CHECK(cudaEventRecord(e_after_preproc_, 0));

        // Pull per-chunk mem-align counters back to host (only the touched
        // range; max 8192 * 20 B = 160 KB). Streams are already synced above,
        // so a plain synchronous memcpy is fine.
        if (h_chunk_counters_per_chunk_ && d_chunk_counters_per_chunk_) {
            CUDA_CHECK(cudaMemcpy(
                h_chunk_counters_per_chunk_,
                d_chunk_counters_per_chunk_,
                (size_t)n_chunks_ * sizeof(ChunkCounters),
                cudaMemcpyDeviceToHost));
        }

        packed_chunk_offsets_h_.assign(n_chunks_ + 1, 0);
        for (uint32_t c = 0; c < n_chunks_; c++)
            packed_chunk_offsets_h_[c + 1] = packed_chunk_offsets_h_[c] + h_n_emits_all_[c];
        num_ops_ = packed_chunk_offsets_h_[n_chunks_];

        std::vector<uint32_t> gappy_u32(n_chunks_);
        for (uint32_t c = 0; c < n_chunks_; c++)
            gappy_u32[c] = (uint32_t)out_offsets_[c];
        CUDA_CHECK(cudaMemcpy(d_gappy_offsets_, gappy_u32.data(),
                              n_chunks_ * 4, cudaMemcpyHostToDevice));
        CUDA_CHECK(cudaMemcpy(d_chunk_lens_, h_n_emits_all_,
                              n_chunks_ * 4, cudaMemcpyHostToDevice));
        CUDA_CHECK(cudaMemcpy(d_packed_chunk_offsets_, packed_chunk_offsets_h_.data(),
                              (n_chunks_ + 1) * 4, cudaMemcpyHostToDevice));
        CUDA_CHECK(cudaMemcpy(h_max_compact_, d_max_compact_, 3 * 4, cudaMemcpyDeviceToHost));

        uint32_t h_invalid = 0;
        CUDA_CHECK(cudaMemcpy(&h_invalid, d_invalid_mode_flag_, 4, cudaMemcpyDeviceToHost));
        if (h_invalid != 0) {
            fprintf(stderr, "CountAndPlan::run FATAL: unrecognised opcode in input\n");
            std::exit(1);
        }

        prepare_global_();
        CUDA_CHECK(cudaEventRecord(e_after_prepare_, 0));
        preprocessed_ = true;
    }

    process_worker_();

    if (!metas_ready_recorded_) {
        CUDA_CHECK(cudaEventRecord(e_metas_ready_, 0));
        CUDA_CHECK(cudaEventSynchronize(e_metas_ready_));
        cudaEventElapsedTime(&last_chunk_to_final_ms_, e_last_chunk_start_, e_metas_ready_);
        metas_ready_recorded_ = true;
    }

    // Hand back the internal pointer + count (no copy). The records — and
    // the pinned-host buffers their pointers reference — stay alive until
    // the next run/reset on this instance.
    if (metas_out) *metas_out = metas_.data();
    n_metas = num_active_;
    return true;
}

void CountAndPlan::reset() {
    n_chunks_              = 0;
    num_ops_               = 0;
    h_memops_used_         = 0;
    d_ops_pool_used_u32_   = 0;
    out_offsets_.clear();
    out_offsets_.push_back(0);
    n_potentials_per_chunk_.clear();
    n_ram_per_chunk_.clear();
    packed_chunk_offsets_h_.clear();
    metas_.assign(MAX_INSTANCES, InstanceMeta{});
    metas_ready_recorded_   = false;
    last_chunk_to_final_ms_ = 0.f;
    preprocessed_           = false;
    prepared_               = false;

    if (d_histogram_)                CUDA_CHECK(cudaMemset(d_histogram_, 0, (size_t)N_ADDR * 4));
    if (d_max_compact_)              CUDA_CHECK(cudaMemset(d_max_compact_, 0, 3 * 4));
    if (d_invalid_mode_flag_)        CUDA_CHECK(cudaMemset(d_invalid_mode_flag_, 0, 4));
    if (d_chunk_counters_per_chunk_) CUDA_CHECK(cudaMemset(d_chunk_counters_per_chunk_, 0,
        (size_t)MAX_CHUNKS * sizeof(ChunkCounters)));
}

void CountAndPlan::free_pinned_() {
    if (h_memops_)                   { cudaFreeHost(h_memops_);                   h_memops_                   = nullptr; }
    if (h_n_emits_all_)              { cudaFreeHost(h_n_emits_all_);              h_n_emits_all_              = nullptr; }
    if (h_offsets_buf_)              { cudaFreeHost(h_offsets_buf_);              h_offsets_buf_              = nullptr; }
    if (h_result_nops_)              { cudaFreeHost(h_result_nops_);              h_result_nops_              = nullptr; }
    if (h_meta_scalars_)             { cudaFreeHost(h_meta_scalars_);             h_meta_scalars_             = nullptr; }
    if (h_chunk_counters_per_chunk_) { cudaFreeHost(h_chunk_counters_per_chunk_); h_chunk_counters_per_chunk_ = nullptr; }
    for (int s = 0; s < N_STREAMS; s++)
        if (h_n_emits_[s]) { cudaFreeHost(h_n_emits_[s]); h_n_emits_[s] = nullptr; }
}

void CountAndPlan::free_all_() {
    for (int s = 0; s < N_STREAMS; s++)
        if (streams_[s]) { cudaStreamDestroy(streams_[s]); streams_[s] = nullptr; }
    if (d2h_stream_)         { cudaStreamDestroy(d2h_stream_);  d2h_stream_  = nullptr; }
    if (meta_stream_)        { cudaStreamDestroy(meta_stream_); meta_stream_ = nullptr; }
    if (e_last_chunk_start_) { cudaEventDestroy(e_last_chunk_start_); e_last_chunk_start_ = nullptr; }
    if (e_after_preproc_)    { cudaEventDestroy(e_after_preproc_);    e_after_preproc_    = nullptr; }
    if (e_after_prepare_)    { cudaEventDestroy(e_after_prepare_);    e_after_prepare_    = nullptr; }
    if (e_metas_ready_)      { cudaEventDestroy(e_metas_ready_);      e_metas_ready_      = nullptr; }
    free_pinned_();
    if (arena_owned_ && arena_) cudaFree(arena_);
    arena_       = nullptr;
    arena_bytes_ = 0;
    arena_owned_ = false;
    cursor_      = 0;
}

void CountAndPlan::query_cub_sizes_(size_t& scan_counts_b, size_t& scan_emit_b,
                                    size_t& scan_runs_b,   size_t& sort_b,
                                    size_t& rle_b,         size_t& hist_scan_b) {
    const uint32_t MAX_POT = MAX_POT_PER_CHUNK;
    scan_counts_b = scan_emit_b = scan_runs_b = sort_b = rle_b = hist_scan_b = 0;
    cub::DeviceScan::ExclusiveSum(nullptr, scan_counts_b,
        (uint32_t*)nullptr, (uint32_t*)nullptr, MAX_MEMOPS_PER_CHUNK + 1);
    cub::DeviceScan::ExclusiveSum(nullptr, scan_emit_b,
        (uint32_t*)nullptr, (uint32_t*)nullptr, MAX_POT + 1);
    cub::DeviceScan::ExclusiveSum(nullptr, scan_runs_b,
        (uint32_t*)nullptr, (uint32_t*)nullptr, MAX_POT + 1);
    cub::DeviceRadixSort::SortKeys(nullptr, sort_b,
        (uint64_t*)nullptr, (uint64_t*)nullptr, MAX_POT);
    cub::DeviceRunLengthEncode::Encode(nullptr, rle_b,
        (uint32_t*)nullptr, thrust::discard_iterator<>{},
        (uint32_t*)nullptr, (uint32_t*)nullptr, MAX_POT);
    cub::DeviceScan::ExclusiveSum(nullptr, hist_scan_b,
        (uint32_t*)nullptr, (uint32_t*)nullptr, N_ADDR);
}

void CountAndPlan::prepare_global_() {
    if (prepared_) return;
    {
        uint32_t n_rom = h_max_compact_[REGION_ROM] + 2;
        cub::DeviceScan::ExclusiveSum(d_temp_hist_, d_temp_hist_bytes_,
            d_histogram_ + 0, d_prefix_ + 0, n_rom);

        uint32_t n_in = h_max_compact_[REGION_INPUT] + 2;
        cub::DeviceScan::ExclusiveSum(d_temp_hist_, d_temp_hist_bytes_,
            d_histogram_ + N_ADDR_ROM, d_prefix_ + N_ADDR_ROM, n_in);
        add_const_kernel<<<(n_in + 255) / 256, 256>>>(
            d_prefix_ + N_ADDR_ROM, d_prefix_ + h_max_compact_[REGION_ROM] + 1, n_in);

        uint32_t n_ram = h_max_compact_[REGION_RAM] + 2;
        cub::DeviceScan::ExclusiveSum(d_temp_hist_, d_temp_hist_bytes_,
            d_histogram_ + N_ADDR_ROM + N_ADDR_INPUT,
            d_prefix_ + N_ADDR_ROM + N_ADDR_INPUT, n_ram);
        add_const_kernel<<<(n_ram + 255) / 256, 256>>>(
            d_prefix_ + N_ADDR_ROM + N_ADDR_INPUT,
            d_prefix_ + N_ADDR_ROM + h_max_compact_[REGION_INPUT] + 1, n_ram);
    }

    uint32_t h_boundary[3];
    cudaMemcpy(&h_boundary[0],
               d_prefix_ + h_max_compact_[REGION_ROM] + 1, 4, cudaMemcpyDeviceToHost);
    cudaMemcpy(&h_boundary[1],
               d_prefix_ + N_ADDR_ROM + h_max_compact_[REGION_INPUT] + 1, 4, cudaMemcpyDeviceToHost);
    h_boundary[2] = num_ops_;

    region_n_ops_[REGION_ROM]   = h_boundary[0];
    region_n_ops_[REGION_INPUT] = h_boundary[1] - h_boundary[0];
    region_n_ops_[REGION_RAM]   = h_boundary[2] - h_boundary[1];

    num_instances_ = 0;
    for (uint8_t r = 0; r < 3; r++) {
        num_inst_[r] = region_n_ops_[r] ? (region_n_ops_[r] + INSTANCE_SIZE[r] - 1) / INSTANCE_SIZE[r] : 0;
        num_instances_ += num_inst_[r];
    }
    if (num_instances_ > MAX_INSTANCES) {
        std::cerr << "CountAndPlan: too many instances ("
                  << num_instances_ << " > " << MAX_INSTANCES
                  << " ROM=" << num_inst_[0]
                  << " INPUT=" << num_inst_[1]
                  << " RAM=" << num_inst_[2] << ")" << std::endl;
        std::exit(1);
    }
    region_ops_start_[REGION_ROM]   = 0;
    region_ops_start_[REGION_INPUT] = h_boundary[0];
    region_ops_start_[REGION_RAM]   = h_boundary[1];
    prepared_ = true;
}

void CountAndPlan::set_active_worker_() {
    std::memset(active_mask_, 0, sizeof(active_mask_));
    for (uint32_t i = 0; i < MAX_INSTANCES; i++)
        if (i % n_workers_ == worker_id_)
            active_mask_[i / 32] |= (1u << (i % 32));
}

void CountAndPlan::pick_active_instances_() {
    uint32_t pos = 0, gid_base = 0;
    for (uint8_t r = 0; r < 3; r++) {
        num_active_per_[r] = 0;
        active_offset_[r]  = pos;
        for (uint32_t lid = 0; lid < num_inst_[r]; lid++) {
            uint32_t gid = gid_base + lid;
            if (active_mask_[gid / 32] & (1u << (gid % 32)))
                h_active_local_ids_[pos + num_active_per_[r]++] = lid;
        }
        pos      += num_active_per_[r];
        gid_base += num_inst_[r];
    }
    num_active_ = num_active_per_[0] + num_active_per_[1] + num_active_per_[2];
    CUDA_CHECK(cudaMemcpy(d_active_ids_, h_active_local_ids_,
                          num_active_ * 4, cudaMemcpyHostToDevice));
}

void CountAndPlan::process_worker_() {
    set_active_worker_();
    pick_active_instances_();

    CUDA_CHECK(cudaMemset(d_fml_, 0, (size_t)num_active_ * n_chunks_ * 3 * 4));

    for (uint8_t r = 0; r < 3; r++) {
        if (num_active_per_[r] == 0) continue;
        uint32_t na  = num_active_per_[r];
        uint32_t off = active_offset_[r];
        instance_boundaries_kernel<<<1, na>>>(
            d_prefix_, REGION_ADDR_START[r], h_max_compact_[r] + 1,
            region_n_ops_[r], INSTANCE_SIZE[r],
            d_active_ids_ + off, d_active_first_ + off, d_active_last_ + off,
            d_offset_starts_ + off, na);
    }

    int fml_block, fml_grid;
    cudaOccupancyMaxPotentialBlockSize(&fml_grid, &fml_block,
        chunk_fml_count_gappy_kernel, 0, 0);
    chunk_fml_count_gappy_kernel<<<fml_grid, fml_block>>>(
        d_ops_pool_, d_gappy_offsets_, d_packed_chunk_offsets_,
        d_active_first_, d_active_last_,
        d_fml_, num_active_, n_chunks_, num_ops_);

    CUDA_CHECK(cudaDeviceSynchronize());
    CUDA_CHECK(cudaMemcpy(h_active_first_.data(), d_active_first_, num_active_ * 4, cudaMemcpyDeviceToHost));
    CUDA_CHECK(cudaMemcpy(h_active_last_.data(),  d_active_last_,  num_active_ * 4, cudaMemcpyDeviceToHost));

    std::vector<uint32_t> h_offset_starts(num_active_);
    uint32_t total_addrs = 0;
    for (uint32_t i = 0; i < num_active_; i++) {
        h_offset_starts[i] = total_addrs;
        total_addrs += h_active_last_[i] - h_active_first_[i] + 1;
    }

    // h_offsets_buf_ is the pinned destination for the addr_offsets D2H copy.
    // Its size must cover total_addrs * sizeof(uint32_t); otherwise the async
    // copy fails (destination not fully pinned) and metas point at stale zero
    // memory. Grow it on demand with a small headroom to avoid re-alloc churn.
    size_t needed_offsets_bytes = (size_t)total_addrs * sizeof(uint32_t);
    if (needed_offsets_bytes > h_offsets_buf_size_) {
        if (h_offsets_buf_) CUDA_CHECK(cudaFreeHost(h_offsets_buf_));
        h_offsets_buf_size_ = needed_offsets_bytes + (needed_offsets_bytes / 4);
        CUDA_CHECK(cudaMallocHost(&h_offsets_buf_, h_offsets_buf_size_));
    }

    // instance_boundaries_kernel writes d_offset_starts_ per-region (each region
    // resets `offset = 0`). compute_addr_offsets_kernel writes into d_addr_offsets_
    // at those offsets, but the host reads d_addr_offsets_ with cross-region
    // cumulative indices (h_offset_starts) → without this push, RAM/INPUT
    // instances overwrite the start of d_addr_offsets_ that ROM already filled,
    // and downstream addr_offsets values are garbage. Sync d_offset_starts_ with
    // the host's cross-region layout before launching compute_addr_offsets_kernel.
    CUDA_CHECK(cudaMemcpy(d_offset_starts_, h_offset_starts.data(),
                          num_active_ * 4, cudaMemcpyHostToDevice));

    for (uint8_t r = 0; r < 3; r++) {
        if (num_active_per_[r] == 0) continue;
        uint32_t na  = num_active_per_[r];
        uint32_t off = active_offset_[r];

        build_metas_kernel<<<na, 256, 0, meta_stream_>>>(
            d_fml_ + (size_t)off * n_chunks_ * 3, d_prefix_,
            REGION_ADDR_START[r], region_n_ops_[r], INSTANCE_SIZE[r],
            d_active_ids_ + off, d_active_first_ + off, d_active_last_ + off,
            d_result_nops_ + (size_t)off * n_chunks_,
            d_meta_scalars_ + off * 4, na, n_chunks_);

        compute_addr_offsets_kernel<<<na, 1024, 0, d2h_stream_>>>(
            d_prefix_, REGION_ADDR_START[r], region_n_ops_[r], INSTANCE_SIZE[r],
            d_active_ids_ + off, d_active_first_ + off, d_active_last_ + off,
            d_addr_offsets_, d_offset_starts_ + off, na);
    }

    CUDA_CHECK(cudaMemcpyAsync(h_meta_scalars_, d_meta_scalars_,
                               num_active_ * 4 * 4, cudaMemcpyDeviceToHost, meta_stream_));
    CUDA_CHECK(cudaMemcpyAsync(h_result_nops_, d_result_nops_,
                               (size_t)num_active_ * n_chunks_ * 4, cudaMemcpyDeviceToHost, meta_stream_));
    CUDA_CHECK(cudaMemcpyAsync(h_offsets_buf_, d_addr_offsets_,
                               (size_t)total_addrs * 4, cudaMemcpyDeviceToHost, d2h_stream_));

    CUDA_CHECK(cudaStreamSynchronize(meta_stream_));
    CUDA_CHECK(cudaStreamSynchronize(d2h_stream_));

    uint32_t ai = 0;
    for (uint8_t r = 0; r < 3; r++) {
        for (uint32_t j = 0; j < num_active_per_[r]; j++, ai++) {
            uint32_t* scalars = h_meta_scalars_ + ai * 4;
            metas_[ai].inst_id           = h_active_local_ids_[active_offset_[r] + j];
            metas_[ai].kind              = r;
            metas_[ai].first_addr        = expand_addr(h_active_first_[ai]);
            metas_[ai].last_addr         = expand_addr(h_active_last_[ai]);
            metas_[ai].first_addr_chunk  = scalars[0];
            metas_[ai].first_addr_skip   = scalars[1];
            metas_[ai].last_addr_chunk   = scalars[2];
            metas_[ai].last_addr_include = scalars[3];
            const uint32_t num_addrs = h_active_last_[ai] - h_active_first_[ai] + 1;
            metas_[ai].count_per_chunk      = h_result_nops_ + (size_t)ai * n_chunks_;
            metas_[ai].n_chunks = n_chunks_;
            metas_[ai].addr_offsets         = h_offsets_buf_ + h_offset_starts[ai];
            metas_[ai].addr_offsets_size    = num_addrs;
        }
    }
}
