// =====================================================================
//
// CLI:
//   ./pair_sort_full_gpu <block_number> [--save-metas <path>] [--verify-metas <path>]
//
// =====================================================================

// (No local includes — main_full.cu is self-contained. Constants, structs,
// helpers, and the preprocessing kernels that previously lived in
// mem_preprocess.cuh are inlined below.)

#include <cuda_runtime.h>
#include <cub/device/device_radix_sort.cuh>
#include <cub/device/device_run_length_encode.cuh>
#include <cub/device/device_scan.cuh>
#include <thrust/iterator/discard_iterator.h>

#include <algorithm>
#include <cctype>
#include <chrono>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <dirent.h>
#include <iomanip>
#include <iostream>
#include <omp.h>
#include <random>
#include <span>
#include <stdexcept>
#include <string>
#include <sys/stat.h>
#include <vector>

// =====================================================================
// Preprocessing constants, structs, helpers, and kernels
// (inlined from the previous mem_preprocess.cuh; main_full.cu owns its own
// copy so it can be built standalone)
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

// Raw-address bases — referenced by compact_addr / expand_addr / compact_addr_dev
// further down, plus the preprocessing helpers below.
constexpr uint32_t ZISK_ROM_ADDR_BASE     = 0x80000000u;
constexpr uint32_t ZISK_INPUT_ADDR_BASE   = 0x40000000u;
constexpr uint32_t ZISK_RAM_ADDR_BASE     = 0xA0000000u;
constexpr uint32_t ZISK_RAM_SIZE_BYTES    = 512u * 1024u * 1024u;
constexpr uint32_t ZISK_RAM_ADDR_END      = ZISK_RAM_ADDR_BASE + ZISK_RAM_SIZE_BYTES;
constexpr uint32_t ZISK_ALIGN_MASK        = 0xFFFFFFF8u;

constexpr uint32_t CHUNK_MAX_MEMOPS            = 1u << 18;     // 256K — matches zisk CHUNK_SIZE
constexpr uint32_t POTENTIAL_CAP_PER_CHUNK     = 1u << 21;     // 2M — 4× memops + headroom
constexpr uint32_t MAX_BLOCKOP_SPILL_PER_CHUNK = 16u * 1024u;  // big-block spill capacity per chunk
constexpr uint32_t BLOCKOP_SPILL_THRESH_VAL    = 64u;          // count > thresh → spill kernel
constexpr int      ZISK_N_STREAMS              = 4;

// Per-memop record decoded by preprocessing (8-byte aligned for vectorised H2D).
struct __align__(8) MemOp {
    uint32_t addr;
    uint32_t flags;
};

// One potential emission, packed into a single uint32:
//   bit  0     POT_FLAG_IS_RAM   1 if address falls in RAM region
//   bit  1     POT_FLAG_KIND_W   1 if Write event (else Read)
//   bit  2     reserved
//   bits 3..31 aligned 8-byte address (low 3 bits naturally 0)
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

// Big-block spill record: queued by decode_count_kernel, drained by
// blockop_emit_kernel one CTA at a time.
struct BlockOpSpill {
    uint32_t memop_idx;
    uint32_t aligned_base;
    uint32_t count;
    uint32_t kind_w;
};

// Per-chunk diagnostic counters (mirrors mem_counter_single.cpp accumulators).
struct ChunkCounters {
    uint32_t full_5;
    uint32_t full_3;
    uint32_t full_2;
    uint32_t read_byte;
    uint32_t write_byte;
};

// Bit layout for the 64-bit RAM sort key used by the preprocessing pipeline:
//   bit  0          = kind_w        (Read/Write)
//   bits [1..21]    = orig_pos      (≤ 21 bits, fits POTENTIAL_CAP_PER_CHUNK=2M)
//   bits [22..47]   = compact_addr  (26 bits, fits 64M RAM slots)
// 48 bits total → RAM_KEY_END_BIT passed to cub::DeviceRadixSort.
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

// True iff `addr` is in the RAM region. Gates which potentials need the
// duality state machine (RAM only) versus unconditional emit (ROM/INPUT).
__host__ __device__ __forceinline__
bool is_ram_addr(uint32_t addr) {
    return (addr >= ZISK_RAM_ADDR_BASE) && (addr < ZISK_RAM_ADDR_END);
}

// Map a RAM aligned address to a 0-based slot index (used as the high bits
// of the sort key). Result fits in 26 bits.
__host__ __device__ __forceinline__
uint32_t ram_compact(uint32_t aligned_addr) {
    return (aligned_addr - ZISK_RAM_ADDR_BASE) >> 3;
}

// Fused per-memop validity check + potential-emission count + counter delta.
// Unknown opcode → atomicOr's *d_invalid_mode_flag, zeros *count_out, returns
// false (host inspects the flag at end of pipeline and aborts).
__device__ __forceinline__
bool decode(MemOp op,
            uint32_t* count_out,
            ChunkCounters& counters_out,
            uint32_t* d_invalid_mode_flag) {
    const uint32_t addr        = op.addr;
    const uint32_t aligned     = addr & ZISK_ALIGN_MASK;
    const uint8_t  mode        = op.flags & 0x3F;
    const uint32_t off_in_word = addr & 0x07;

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

// Decode one memop into 1..4 PotentialEmit records (or 0..count for blocks).
// `skip_block == true` means a big block was claimed by blockop_emit_kernel
// and should NOT be filled inline here (also serves as the correctness
// fallback when the spill table overflows).
__device__ __forceinline__
void decode_emit_inline(MemOp op, PotentialEmit* out, bool skip_block) {
    const uint32_t addr        = op.addr;
    const uint32_t aligned     = addr & ZISK_ALIGN_MASK;
    const uint8_t  mode        = op.flags & 0x3F;
    const uint32_t off_in_word = addr & 0x07;

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

// Per-block ChunkCounters reduce: each thread atomicAdd's into shared mem,
// thread 0 atomicAdd's the block-private result into the global slot.
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

// [Step 1] One thread per memop. Writes potential-emission count, accumulates
// counter deltas via block_reduce_counters, queues big block ops for the
// spill kernel. d_spill_status[i] = 1 iff the memop was claimed by spill.
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
            const uint8_t mode = op.flags & 0x3F;
            const uint8_t base = mode & 0x0Fu;
            const bool is_block_read  = (base == (MOPS_BLOCK_READ  & 0x0F)) ||
                                        (base == (MOPS_ALIGNED_BLOCK_READ  & 0x0F));
            const bool is_block_write = (base == (MOPS_BLOCK_WRITE & 0x0F)) ||
                                        (base == (MOPS_ALIGNED_BLOCK_WRITE & 0x0F));
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
                    // Else: spill table full, d_spill_status[i] stays 0 →
                    // decode_emit_kernel falls back to inline (slow but correct).
                }
            }
        }
    }
    block_reduce_counters(my, d_chunk_counters_entry);
}

// [Step 4] One thread per memop. Writes the memop's PotentialEmit slots into
// d_potentials[d_potential_offsets[i] ..]. Spilled big blocks (status==1)
// are skipped here — blockop_emit_kernel handles them.
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

// [Step 5] One CTA per spilled big block. Threads stride through the block's
// `count` aligned addresses and write a PotentialEmit each. Slot base looked
// up on the fly via d_potential_offsets[memop_idx].
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

// [Step 8a] Strip the high `compact_addr` bits from each sorted RAM key into
// a dense uint32 array, ready for cub::DeviceRunLengthEncode.
__global__
void extract_sorted_addr_kernel(const uint64_t* __restrict__ d_sorted_keys,
                                uint32_t n_events,
                                uint32_t* __restrict__ d_sorted_addr) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n_events) return;
    d_sorted_addr[i] = (uint32_t)(d_sorted_keys[i] >> COMPACT_ADDR_SHIFT);
}

// [Step 8b] Extract a 32-bit (kind_w << 31) | orig_pos value per sorted key,
// so state_machine_by_run_with_hist_kernel reads 4 bytes per event instead
// of the 8-byte sort keys (matches main_preprocess.cu's bandwidth profile).
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
// PairSortGPU constants (mirrors main_real.cu)
// =====================================================================

constexpr uint32_t N_ADDR_ROM   = 1u << 27;   // 128M
constexpr uint32_t N_ADDR_INPUT = 1u << 30;   // 1G
constexpr uint32_t N_ADDR_RAM   = 1u << 29;   // 512M
constexpr uint32_t N_ADDR = N_ADDR_ROM + N_ADDR_INPUT + N_ADDR_RAM;

constexpr uint32_t INSTANCE_SIZE[3]  = {1u << 21, 1u << 21, 1u << 22};
constexpr uint32_t INSTANCE_SIZE_MAX = 1u << 22;

constexpr uint32_t MAX_INST_ROM   = 32;
constexpr uint32_t MAX_INST_INPUT = 32;
constexpr uint32_t MAX_INST_RAM   = 256;
constexpr uint32_t MAX_INSTANCES  = MAX_INST_ROM + MAX_INST_INPUT + MAX_INST_RAM;
constexpr uint32_t MAX_INST[3]    = {MAX_INST_ROM, MAX_INST_INPUT, MAX_INST_RAM};
constexpr uint32_t MASK_WORDS     = (MAX_INSTANCES + 31) / 32;

constexpr uint32_t MAX_OPS = (uint32_t)MAX_INST_ROM   * INSTANCE_SIZE[0]
                           + (uint32_t)MAX_INST_INPUT * INSTANCE_SIZE[1]
                           + (uint32_t)MAX_INST_RAM   * INSTANCE_SIZE[2];
constexpr uint32_t N_WORKERS  = 16;
constexpr uint32_t MAX_ACTIVE = (MAX_INSTANCES + N_WORKERS - 1) / N_WORKERS;
constexpr uint32_t MAX_CHUNKS = 4096;

constexpr uint8_t REGION_ROM            = 0;
constexpr uint8_t REGION_INPUT          = 1;
constexpr uint8_t REGION_RAM            = 2;
constexpr const char* REGION_NAME[3]    = {"ROM", "INPUT", "RAM"};
constexpr uint32_t REGION_ADDR_START[3] = {0, N_ADDR_ROM, N_ADDR_ROM + N_ADDR_INPUT};

// =====================================================================
// Address-region helpers (host + device)
//
// Region-test order matters: ROM (0x80000000+) and INPUT (0x40000000+)
// both satisfy `addr >= 0x40000000`, so RAM must be tested first, then
// ROM, with INPUT as the fallthrough.
//
// Raw-address bases (ZISK_*_ADDR_BASE) come from mem_preprocess.cuh.
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

// Device-side mirror of `compact_addr` (same region-test order).
__device__ __forceinline__ uint32_t compact_addr_dev(uint32_t raw) {
    if (raw >= ZISK_RAM_ADDR_BASE)
        return ((raw - ZISK_RAM_ADDR_BASE) >> 3) + N_ADDR_ROM + N_ADDR_INPUT;
    if (raw >= ZISK_ROM_ADDR_BASE)
        return (raw - ZISK_ROM_ADDR_BASE) >> 3;
    return ((raw - ZISK_INPUT_ADDR_BASE) >> 3) + N_ADDR_ROM;
}

// =====================================================================
// New / specialised kernels for main_full.cu
//
// add_const_kernel              — fix-up for region-scoped prefix sums
// compact_kernel_with_shift     — variant of mem_preprocess.cuh's compact_kernel
// gather_ram_events_with_hist_kernel
//                               — variant of mem_preprocess.cuh's gather_ram_events_kernel
// state_machine_by_run_with_hist_kernel
//                               — variant of mem_preprocess.cuh's state_machine_by_run_kernel
// chunk_fml_count_gappy_kernel  — variant of main_real.cu's chunk_fml_count_kernel
//
// The remaining PairSortGPU kernels (instance_boundaries, build_metas,
// compute_addr_offsets) are copied verbatim from main_real.cu.
// =====================================================================

// Adds a constant offset (read from device pointer) to every cell in [0, n).
// Used to lift region-local prefix sums into the unified prefix layout
// (INPUT prefix needs +total_rom; RAM prefix needs +total_rom+total_input)
// in PairSortGPU::prepare_global.
//
// Input:  d_offset    — device pointer to the constant to add (one uint32)
// In/Out: arr[0..n)   — incremented by *d_offset
__global__ void add_const_kernel(uint32_t* arr, const uint32_t* d_offset, uint32_t n) {
    uint32_t off = *d_offset;
    uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n) arr[i] += off;
}

// Replaces mem_preprocess.cuh's compact_kernel: the final per-chunk scatter
// that writes the surviving emits to d_ops_global. Differs only in writing
// COMPACT addresses (compact_addr_dev(raw)) instead of raw aligned addresses,
// so PairSortGPU's downstream kernels can consume d_ops_global directly with
// no post-pass shift.
//
// Input:  d_potentials[0..n_potentials)   — populated by decode_emit + blockop_emit
// Input:  d_emit_bits[0..n_potentials)    — 0 = dropped, 1 = surviving
// Input:  d_final_offsets[0..n_potentials)— exclusive prefix sum of d_emit_bits
// Input:  n_potentials                    — exact upper bound (CPU pre-pass)
// Output: d_out[d_final_offsets[i]]       — compact_addr(emit_aligned_addr(p))
//                                            when d_emit_bits[i] == 1
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

// Variant of mem_preprocess.cuh's gather_ram_events_kernel that ALSO builds
// the global compact-address histogram (for non-RAM slots — RAM is handled
// by state_machine_by_run_with_hist_kernel below) and tracks the per-region
// max populated index via a block-level reduce.
//
// Per non-RAM emit: one atomicAdd(d_histogram[compact], 1) and one
// register-only update of local_max[ROM|INPUT]. The block reduces local_maxes
// through shared memory and emits ONE atomicMax(d_max_compact[r], ...) per
// (block, region). Compared with a per-emit global atomicMax this cuts the
// global atomic count by ~256× on contended cells.
//
// Input:  d_potentials[0..n_potentials)
// Output: d_ram_keys[0..*d_ram_count)         — packed (compact_addr, orig_pos, kind_w)
// In/Out: d_ram_count                         — atomic compaction counter
// Output: d_emit_bits[0..n_potentials)        — 1 for non-RAM emits,
//                                                0 for RAM (state_machine writes later)
// In/Out: d_histogram[N_ADDR]                 — atomically incremented for non-RAM emits
// In/Out: d_max_compact[3]                    — region-local max populated index (ROM, INPUT)
__global__
void gather_ram_events_with_hist_kernel(const PotentialEmit* __restrict__ d_potentials,
                                        uint32_t n_potentials,
                                        uint64_t* __restrict__ d_ram_keys,
                                        uint32_t* __restrict__ d_ram_count,
                                        uint32_t* __restrict__ d_emit_bits,
                                        uint32_t* __restrict__ d_histogram,
                                        uint32_t* __restrict__ d_max_compact) {
    // Per-thread local max for ROM and INPUT regions only. RAM is updated
    // by state_machine_by_run_with_hist_kernel.
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
            d_emit_bits[i] = 0;  // overwritten by state_machine
        } else {
            d_emit_bits[i] = 1;
            // Non-RAM (ROM/INPUT) always emits exactly once.
            const uint32_t raw = emit_aligned_addr(p);
            const uint32_t compact = compact_addr_dev(raw);
            atomicAdd(&d_histogram[compact], 1u);
            // compact in [0, N_ADDR_ROM) → ROM (region-local = compact)
            // compact in [N_ADDR_ROM, N_ADDR_ROM+N_ADDR_INPUT) → INPUT (region-local = compact - N_ADDR_ROM)
            if (compact < N_ADDR_ROM) {
                local_max_rom = compact;
            } else {
                local_max_input = compact - N_ADDR_ROM;
            }
        }
    }

    // Block reduce: shared atomic, then one global atomic per (block, region).
    __shared__ uint32_t s_max[3];
    if (threadIdx.x < 3) s_max[threadIdx.x] = 0;
    __syncthreads();
    if (local_max_rom   > 0) atomicMax(&s_max[REGION_ROM],   local_max_rom);
    if (local_max_input > 0) atomicMax(&s_max[REGION_INPUT], local_max_input);
    __syncthreads();
    if (threadIdx.x < 2 && s_max[threadIdx.x] > 0)
        atomicMax(&d_max_compact[threadIdx.x], s_max[threadIdx.x]);
}

// Variant of mem_preprocess.cuh's state_machine_by_run_kernel that ALSO,
// at end of segment, atomicAdd's the segment's surviving emit count into
// d_histogram and atomicMax's d_max_compact[REGION_RAM] via a block-reduce.
//
// The original kernel reads only d_sorted_vals (32-bit packed kind+orig_pos).
// This one additionally needs d_sorted_addr to recover the per-segment
// compact_ram for the histogram bucket.
//
// Per RAM address segment: ONE atomicAdd(d_histogram[compact_unified],
// n_emit) (sum of segment's surviving emits) instead of one per RAM emit.
// For block 24628611: ~tens of millions of atomics collapse to ~hundreds of
// thousands.
//
// Input:  d_run_offsets[0..*d_num_unique]
// Input:  d_sorted_vals[0..n_ram)             — packed (kind_w << 31) | orig_pos
// Input:  d_sorted_addr[0..n_ram)             — region-local RAM compact addr
// Output: d_emit_bits[orig_pos]               — duality-collapsed survival bit
// In/Out: d_histogram[N_ADDR]                 — accumulated for RAM
// In/Out: d_max_compact[REGION_RAM]           — max region-local RAM compact addr
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
            // RAM addresses live at the end of the unified compact space.
            const uint32_t compact_ram = d_sorted_addr[start];
            atomicAdd(&d_histogram[compact_ram + N_ADDR_ROM + N_ADDR_INPUT], n_emit);
            local_max_ram = compact_ram;  // region-local index
        }
    }

    // Block reduce: shared atomic, then one global atomic per block.
    __shared__ uint32_t s_max_ram;
    if (threadIdx.x == 0) s_max_ram = 0;
    __syncthreads();
    if (local_max_ram > 0) atomicMax(&s_max_ram, local_max_ram);
    __syncthreads();
    if (threadIdx.x == 0 && s_max_ram > 0)
        atomicMax(&d_max_compact[REGION_RAM], s_max_ram);
}

// =====================================================================
// PairSortGPU kernels copied verbatim from main_real.cu
//
// instance_boundaries_kernel — first/last compact addr per active instance
//                              via binary search on the prefix sum array
// chunk_fml_count_gappy_kernel — first/middle/last counts per (instance, chunk)
//                                (gappy variant — see comment block above)
// build_metas_kernel        — fa_chunk / fa_skip / la_chunk / la_include +
//                             per-chunk op counts (chunk elimination)
// compute_addr_offsets_kernel — per-address write offsets for scatter
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

// Counts how many ops in each chunk hit the first/middle/last compact
// address of each active instance. Mirrors main_real.cu's
// chunk_fml_count_kernel one-for-one (grid-stride loop, warp-ballot atomic
// coalescing, sorted-address early-break) — with one extra indirection so
// it can read the GAPPY d_ops_global instead of a packed buffer.
//
// Address mapping for a "valid index" i ∈ [0, total_valid_ops):
//   chunk_id          = first c with packed_chunk_offsets[c+1] > i
//   off_in_chunk      = i - packed_chunk_offsets[chunk_id]
//   addr (compact)    = ops[chunk_starts[chunk_id] + off_in_chunk]
//
// Input:  ops[*]                              — d_ops_global (gappy, COMPACT)
// Input:  chunk_starts[num_chunks]            — gappy start of each chunk in ops
// Input:  packed_chunk_offsets[num_chunks+1]  — cumulative valid lens
// Input:  active_first[num_active]            — first compact addr per instance
// Input:  active_last[num_active]             — last compact addr per instance
// Output: d_fml[num_active * num_chunks * 3]  — [ai][chunk][0/1/2] = first/middle/last counts
// Input:  num_active, num_chunks, total_valid_ops
__global__ void chunk_fml_count_gappy_kernel(
    const uint32_t* __restrict__ ops,                    // d_ops_global (gappy, compact addrs)
    const uint32_t* __restrict__ chunk_starts,           // [num_chunks] gappy start
    const uint32_t* __restrict__ packed_chunk_offsets,   // [num_chunks+1] cumulative valid lens
    const uint32_t* __restrict__ active_first,
    const uint32_t* __restrict__ active_last,
    uint32_t* __restrict__ d_fml,
    uint32_t num_active, uint32_t num_chunks, uint32_t total_valid_ops)
{
    __shared__ uint32_t s_first[MAX_ACTIVE];
    __shared__ uint32_t s_last[MAX_ACTIVE];
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

        uint32_t warp_chunk_first = __shfl_sync(0xFFFFFFFF, chunk_id, 0);
        uint32_t warp_chunk_last  = __shfl_sync(0xFFFFFFFF, chunk_id, 31);
        bool same_chunk = (warp_chunk_first == warp_chunk_last);

        for (uint32_t ai = 0; ai < num_active; ai++) {
            uint32_t fa = s_first[ai];
            uint32_t la = s_last[ai];

            if (__all_sync(0xFFFFFFFF, addr < fa)) break;

            bool in_range = (addr >= fa && addr <= la);
            if (!__any_sync(0xFFFFFFFF, in_range)) continue;

            uint32_t cat = 3;
            if (in_range) {
                if (addr == fa)      cat = 0;
                else if (addr == la) cat = 2;
                else                 cat = 1;
            }

            if (same_chunk) {
                for (uint32_t c = 0; c < 3; c++) {
                    unsigned mask = __ballot_sync(0xFFFFFFFF, cat == c);
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
// InstanceMeta + binary save
// =====================================================================

struct InstanceMeta {
    uint32_t inst_id;
    uint8_t  type;
    uint32_t first_addr;
    uint32_t last_addr;
    std::span<const uint32_t> count_per_chunk;
    std::span<uint32_t>       addr_offsets;
    uint32_t first_addr_chunk;
    uint32_t first_addr_skip;
    uint32_t last_addr_chunk;
    uint32_t last_addr_include;
};

static FILE* save_metas_begin(const std::string& path) {
    FILE* f = std::fopen(path.c_str(), "wb");
    if (!f) { std::cerr << "ERROR: open " << path << " for write" << std::endl; std::exit(1); }
    uint32_t placeholder = 0;
    if (std::fwrite(&placeholder, sizeof(uint32_t), 1, f) != 1) {
        std::cerr << "ERROR: short write " << path << std::endl; std::exit(1);
    }
    return f;
}

static void save_metas_append(FILE* f, const InstanceMeta& m) {
    auto wr = [&](const void* p, size_t bytes) {
        if (std::fwrite(p, 1, bytes, f) != bytes) {
            std::cerr << "ERROR: short write" << std::endl; std::exit(1);
        }
    };
    uint8_t type = m.type;
    uint8_t pad[3] = {0, 0, 0};
    uint32_t cps = (uint32_t)m.count_per_chunk.size();
    uint32_t aos = (uint32_t)m.addr_offsets.size();
    wr(&m.inst_id,            sizeof(uint32_t));
    wr(&type, 1);
    wr(pad, 3);
    wr(&m.first_addr,         sizeof(uint32_t));
    wr(&m.last_addr,          sizeof(uint32_t));
    wr(&m.first_addr_chunk,   sizeof(uint32_t));
    wr(&m.first_addr_skip,    sizeof(uint32_t));
    wr(&m.last_addr_chunk,    sizeof(uint32_t));
    wr(&m.last_addr_include,  sizeof(uint32_t));
    wr(&cps, sizeof(uint32_t));
    wr(&aos, sizeof(uint32_t));
    wr(m.count_per_chunk.data(), cps * sizeof(uint32_t));
    wr(m.addr_offsets.data(),    aos * sizeof(uint32_t));
}

static void save_metas_end(FILE* f, uint32_t total) {
    if (std::fseek(f, 0, SEEK_SET) != 0) {
        std::cerr << "ERROR: seek failed" << std::endl; std::exit(1);
    }
    if (std::fwrite(&total, sizeof(uint32_t), 1, f) != 1) {
        std::cerr << "ERROR: short write of header count" << std::endl; std::exit(1);
    }
    std::fclose(f);
}

// =====================================================================
// Reference-meta loader (inline; mirrors instance_meta_loader.hpp,
// inlined here to avoid a name collision with the in-memory InstanceMeta
// struct above which uses std::span instead of std::vector).
// =====================================================================

struct RefMeta {
    uint32_t inst_id;
    uint8_t  type;
    uint32_t first_addr;
    uint32_t last_addr;
    uint32_t first_addr_chunk;
    uint32_t first_addr_skip;
    uint32_t last_addr_chunk;
    uint32_t last_addr_include;
    std::vector<uint32_t> count_per_chunk;
    std::vector<uint32_t> addr_offsets;
};

static std::vector<RefMeta> load_ref_metas(const std::string& path) {
    FILE* f = std::fopen(path.c_str(), "rb");
    if (!f) throw std::runtime_error("cannot open " + path);

    auto rd = [&](void* p, size_t bytes) {
        if (std::fread(p, 1, bytes, f) != bytes) {
            std::fclose(f);
            throw std::runtime_error("short read on " + path);
        }
    };

    uint32_t n;
    rd(&n, sizeof(uint32_t));
    std::vector<RefMeta> out;
    out.reserve(n);
    for (uint32_t i = 0; i < n; i++) {
        RefMeta m;
        uint8_t type, pad[3];
        rd(&m.inst_id,           sizeof(uint32_t));
        rd(&type, 1);
        rd(pad, 3);
        m.type = type;
        rd(&m.first_addr,        sizeof(uint32_t));
        rd(&m.last_addr,         sizeof(uint32_t));
        rd(&m.first_addr_chunk,  sizeof(uint32_t));
        rd(&m.first_addr_skip,   sizeof(uint32_t));
        rd(&m.last_addr_chunk,   sizeof(uint32_t));
        rd(&m.last_addr_include, sizeof(uint32_t));
        uint32_t cps, aos;
        rd(&cps, sizeof(uint32_t));
        rd(&aos, sizeof(uint32_t));
        m.count_per_chunk.resize(cps);
        m.addr_offsets.resize(aos);
        rd(m.count_per_chunk.data(), cps * sizeof(uint32_t));
        rd(m.addr_offsets.data(),    aos * sizeof(uint32_t));
        out.push_back(std::move(m));
    }
    std::fclose(f);
    return out;
}

// Compares an in-memory meta (span views into pinned host) against the
// loaded reference. Returns true on match; on first mismatch fills `err`
// with a diagnostic and returns false.
static bool compare_meta(const InstanceMeta& a, const RefMeta& b, std::string& err) {
    char buf[256];
    auto fail = [&](const char* field, uint64_t got, uint64_t want) -> bool {
        snprintf(buf, sizeof(buf),
                 "field %s: got 0x%llx want 0x%llx",
                 field, (unsigned long long)got, (unsigned long long)want);
        err = buf;
        return false;
    };
    if (a.inst_id != b.inst_id)                       return fail("inst_id",            a.inst_id, b.inst_id);
    if (a.type    != b.type)                          return fail("type",               a.type,    b.type);
    if (a.first_addr != b.first_addr)                 return fail("first_addr",         a.first_addr, b.first_addr);
    if (a.last_addr  != b.last_addr)                  return fail("last_addr",          a.last_addr,  b.last_addr);
    if (a.first_addr_chunk   != b.first_addr_chunk)   return fail("first_addr_chunk",   a.first_addr_chunk,   b.first_addr_chunk);
    if (a.first_addr_skip    != b.first_addr_skip)    return fail("first_addr_skip",    a.first_addr_skip,    b.first_addr_skip);
    if (a.last_addr_chunk    != b.last_addr_chunk)    return fail("last_addr_chunk",    a.last_addr_chunk,    b.last_addr_chunk);
    if (a.last_addr_include  != b.last_addr_include)  return fail("last_addr_include",  a.last_addr_include,  b.last_addr_include);
    if (a.count_per_chunk.size() != b.count_per_chunk.size())
        return fail("count_per_chunk_size", a.count_per_chunk.size(), b.count_per_chunk.size());
    if (a.addr_offsets.size() != b.addr_offsets.size())
        return fail("addr_offsets_size", a.addr_offsets.size(), b.addr_offsets.size());
    for (size_t i = 0; i < a.count_per_chunk.size(); i++) {
        if (a.count_per_chunk[i] != b.count_per_chunk[i]) {
            snprintf(buf, sizeof(buf), "count_per_chunk[%zu]: got %u want %u",
                     i, a.count_per_chunk[i], b.count_per_chunk[i]);
            err = buf;
            return false;
        }
    }
    for (size_t i = 0; i < a.addr_offsets.size(); i++) {
        if (a.addr_offsets[i] != b.addr_offsets[i]) {
            snprintf(buf, sizeof(buf), "addr_offsets[%zu]: got %u want %u",
                     i, a.addr_offsets[i], b.addr_offsets[i]);
            err = buf;
            return false;
        }
    }
    return true;
}

// =====================================================================
// Preprocessing host driver (copied from main_preprocess.cu, sans CPU
// oracle and verify paths). Produces a packed pinned `h_out_packed` plus
// per-chunk offsets that PairSortGPU can consume directly.
// =====================================================================

struct ChunkRef {
    uint32_t file_idx;
    uint32_t n_memops;
    uint32_t memop_offset;
};

static int parse_idx_from_filename(const char* prefix, const char* name) {
    size_t plen = strlen(prefix);
    if (strncmp(name, prefix, plen) != 0) return -1;
    const char* p = name + plen;
    if (!*p || !isdigit((unsigned char)*p)) return -1;
    char* end = nullptr;
    long v = strtol(p, &end, 10);
    if (!end || strcmp(end, ".bin") != 0) return -1;
    return (int)v;
}

static std::vector<uint32_t> list_indices(const std::string& dir, const char* prefix) {
    std::vector<uint32_t> idxs;
    DIR* d = opendir(dir.c_str());
    if (!d) { fprintf(stderr, "ERROR: cannot open %s\n", dir.c_str()); exit(1); }
    struct dirent* e;
    while ((e = readdir(d)) != nullptr) {
        int idx = parse_idx_from_filename(prefix, e->d_name);
        if (idx >= 0) idxs.push_back((uint32_t)idx);
    }
    closedir(d);
    std::sort(idxs.begin(), idxs.end());
    return idxs;
}

static size_t file_size_of(const std::string& path) {
    struct stat st;
    if (stat(path.c_str(), &st) != 0) { fprintf(stderr, "ERROR: stat %s\n", path.c_str()); exit(1); }
    return (size_t)st.st_size;
}

struct StreamBufs {
    cudaStream_t stream;

    MemOp*         d_memops;
    uint32_t*      d_counts;
    uint32_t*      d_potential_offsets;
    PotentialEmit* d_potentials;
    uint32_t*      d_emit_bits;
    uint32_t*      d_final_offsets;
    uint64_t*      d_ram_keys;
    uint64_t*      d_ram_keys_sorted;
    uint32_t*      d_ram_vals_sorted;
    uint32_t*      d_ram_count;
    BlockOpSpill*  d_spill;
    uint32_t*      d_spill_count;
    uint8_t*       d_spill_status;

    uint32_t*      d_sorted_addr;
    uint32_t*      d_run_lengths;
    uint32_t*      d_run_offsets;
    uint32_t*      d_num_unique;

    void*          d_scan_temp_counts;   size_t scan_temp_counts_bytes = 0;
    void*          d_scan_temp_emit;     size_t scan_temp_emit_bytes   = 0;
    void*          d_scan_temp_runs;     size_t scan_temp_runs_bytes   = 0;
    void*          d_sort_temp;          size_t sort_temp_bytes        = 0;
    void*          d_rle_temp;           size_t rle_temp_bytes         = 0;

    uint32_t*      h_n_emits;
};

static void alloc_stream_bufs(StreamBufs& s) {
    CUDA_CHECK(cudaStreamCreate(&s.stream));
    CUDA_CHECK(cudaMalloc(&s.d_memops,            sizeof(MemOp) * CHUNK_MAX_MEMOPS));
    CUDA_CHECK(cudaMalloc(&s.d_counts,            sizeof(uint32_t) * (CHUNK_MAX_MEMOPS + 1)));
    CUDA_CHECK(cudaMalloc(&s.d_potential_offsets, sizeof(uint32_t) * (CHUNK_MAX_MEMOPS + 1)));
    CUDA_CHECK(cudaMalloc(&s.d_potentials,        sizeof(PotentialEmit) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_emit_bits,         sizeof(uint32_t) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_final_offsets,     sizeof(uint32_t) * (POTENTIAL_CAP_PER_CHUNK + 1)));
    CUDA_CHECK(cudaMalloc(&s.d_ram_keys,          sizeof(uint64_t) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_ram_keys_sorted,   sizeof(uint64_t) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_ram_vals_sorted,   sizeof(uint32_t) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_ram_count,         sizeof(uint32_t)));
    CUDA_CHECK(cudaMalloc(&s.d_spill,             sizeof(BlockOpSpill) * MAX_BLOCKOP_SPILL_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_spill_count,       sizeof(uint32_t)));
    CUDA_CHECK(cudaMalloc(&s.d_spill_status,      sizeof(uint8_t) * CHUNK_MAX_MEMOPS));
    CUDA_CHECK(cudaMalloc(&s.d_sorted_addr,       sizeof(uint32_t) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_run_lengths,       sizeof(uint32_t) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_run_offsets,       sizeof(uint32_t) * (POTENTIAL_CAP_PER_CHUNK + 1)));
    CUDA_CHECK(cudaMalloc(&s.d_num_unique,        sizeof(uint32_t)));

    cub::DeviceScan::ExclusiveSum(nullptr, s.scan_temp_counts_bytes,
        (uint32_t*)nullptr, (uint32_t*)nullptr, CHUNK_MAX_MEMOPS + 1);
    CUDA_CHECK(cudaMalloc(&s.d_scan_temp_counts, s.scan_temp_counts_bytes));

    cub::DeviceScan::ExclusiveSum(nullptr, s.scan_temp_emit_bytes,
        (uint32_t*)nullptr, (uint32_t*)nullptr, POTENTIAL_CAP_PER_CHUNK + 1);
    CUDA_CHECK(cudaMalloc(&s.d_scan_temp_emit, s.scan_temp_emit_bytes));

    cub::DeviceScan::ExclusiveSum(nullptr, s.scan_temp_runs_bytes,
        (uint32_t*)nullptr, (uint32_t*)nullptr, POTENTIAL_CAP_PER_CHUNK + 1);
    CUDA_CHECK(cudaMalloc(&s.d_scan_temp_runs, s.scan_temp_runs_bytes));

    cub::DeviceRadixSort::SortKeys(nullptr, s.sort_temp_bytes,
        (uint64_t*)nullptr, (uint64_t*)nullptr, POTENTIAL_CAP_PER_CHUNK);
    CUDA_CHECK(cudaMalloc(&s.d_sort_temp, s.sort_temp_bytes));

    cub::DeviceRunLengthEncode::Encode(nullptr, s.rle_temp_bytes,
        (uint32_t*)nullptr, thrust::discard_iterator<>{},
        (uint32_t*)nullptr, (uint32_t*)nullptr, POTENTIAL_CAP_PER_CHUNK);
    CUDA_CHECK(cudaMalloc(&s.d_rle_temp, s.rle_temp_bytes));

    CUDA_CHECK(cudaMallocHost(&s.h_n_emits, sizeof(uint32_t)));
}

static void free_stream_bufs(StreamBufs& s) {
    cudaFree(s.d_memops); cudaFree(s.d_counts); cudaFree(s.d_potential_offsets);
    cudaFree(s.d_potentials); cudaFree(s.d_emit_bits); cudaFree(s.d_final_offsets);
    cudaFree(s.d_ram_keys);
    cudaFree(s.d_ram_keys_sorted); cudaFree(s.d_ram_vals_sorted);
    cudaFree(s.d_ram_count); cudaFree(s.d_spill); cudaFree(s.d_spill_count);
    cudaFree(s.d_spill_status);
    cudaFree(s.d_sorted_addr); cudaFree(s.d_run_lengths); cudaFree(s.d_run_offsets);
    cudaFree(s.d_num_unique);
    cudaFree(s.d_scan_temp_counts); cudaFree(s.d_scan_temp_emit);
    cudaFree(s.d_scan_temp_runs); cudaFree(s.d_sort_temp); cudaFree(s.d_rle_temp);
    cudaFreeHost(s.h_n_emits);
    cudaStreamDestroy(s.stream);
}

static void run_chunk(StreamBufs& sb,
                      const MemOp* h_memops_chunk,
                      uint32_t n_memops,
                      uint32_t n_potentials,
                      uint32_t n_ram,
                      ChunkCounters* d_chunk_counters_scratch,
                      uint32_t* d_invalid_mode_flag,
                      uint32_t* d_histogram,
                      uint32_t* d_max_compact,
                      uint32_t* d_chunk_out,           // gappy slice in d_ops_global
                      uint32_t* h_n_emits_slot)
{
    if (n_memops == 0) {
        *h_n_emits_slot = 0;
        return;
    }

    cudaStream_t st = sb.stream;
    constexpr int BLOCK = 256;
    const int g_memops = (n_memops + BLOCK - 1) / BLOCK;
    const int g_pot    = (n_potentials + BLOCK - 1) / BLOCK;
    const int g_ram    = n_ram == 0 ? 0 : (int)((n_ram + BLOCK - 1) / BLOCK);

    CUDA_CHECK(cudaMemcpyAsync(sb.d_memops, h_memops_chunk,
                               sizeof(MemOp) * n_memops, cudaMemcpyHostToDevice, st));

    CUDA_CHECK(cudaMemsetAsync(sb.d_ram_count,   0, sizeof(uint32_t), st));
    CUDA_CHECK(cudaMemsetAsync(sb.d_spill_count, 0, sizeof(uint32_t), st));
    CUDA_CHECK(cudaMemsetAsync(sb.d_spill_status, 0, sizeof(uint8_t) * n_memops, st));

    decode_count_kernel<<<g_memops, BLOCK, 0, st>>>(
        sb.d_memops, n_memops, sb.d_counts, sb.d_spill_status,
        d_chunk_counters_scratch,
        sb.d_spill, sb.d_spill_count,
        d_invalid_mode_flag);

    {
        size_t bytes = sb.scan_temp_counts_bytes;
        cub::DeviceScan::ExclusiveSum(sb.d_scan_temp_counts, bytes,
            sb.d_counts, sb.d_potential_offsets, n_memops + 1, st);
    }

    decode_emit_kernel<<<g_memops, BLOCK, 0, st>>>(
        sb.d_memops, n_memops, sb.d_potential_offsets, sb.d_spill_status, sb.d_potentials);

    blockop_emit_kernel<<<MAX_BLOCKOP_SPILL_PER_CHUNK, 256, 0, st>>>(
        sb.d_spill, sb.d_spill_count, sb.d_potential_offsets, sb.d_potentials);

    gather_ram_events_with_hist_kernel<<<g_pot, BLOCK, 0, st>>>(
        sb.d_potentials, n_potentials,
        sb.d_ram_keys, sb.d_ram_count, sb.d_emit_bits, d_histogram, d_max_compact);

    if (n_ram > 0) {
        size_t bytes_sort = sb.sort_temp_bytes;
        cub::DeviceRadixSort::SortKeys(sb.d_sort_temp, bytes_sort,
            sb.d_ram_keys, sb.d_ram_keys_sorted,
            n_ram, 0, RAM_KEY_END_BIT, st);

        extract_sorted_addr_kernel<<<g_ram, BLOCK, 0, st>>>(
            sb.d_ram_keys_sorted, n_ram, sb.d_sorted_addr);

        extract_sorted_packed_kernel<<<g_ram, BLOCK, 0, st>>>(
            sb.d_ram_keys_sorted, n_ram, sb.d_ram_vals_sorted);

        {
            size_t bytes_rle = sb.rle_temp_bytes;
            cub::DeviceRunLengthEncode::Encode(sb.d_rle_temp, bytes_rle,
                sb.d_sorted_addr,
                thrust::discard_iterator<>{},
                sb.d_run_lengths,
                sb.d_num_unique,
                n_ram, st);
        }
        {
            size_t bytes_scan = sb.scan_temp_runs_bytes;
            cub::DeviceScan::ExclusiveSum(sb.d_scan_temp_runs, bytes_scan,
                sb.d_run_lengths, sb.d_run_offsets, n_ram + 1, st);
        }
        state_machine_by_run_with_hist_kernel<<<g_ram, BLOCK, 0, st>>>(
            sb.d_run_offsets, sb.d_num_unique, sb.d_ram_vals_sorted,
            sb.d_sorted_addr, sb.d_emit_bits, d_histogram, d_max_compact);
    }

    {
        size_t bytes = sb.scan_temp_emit_bytes;
        cub::DeviceScan::ExclusiveSum(sb.d_scan_temp_emit, bytes,
            sb.d_emit_bits, sb.d_final_offsets, n_potentials + 1, st);
    }

    CUDA_CHECK(cudaMemcpyAsync(h_n_emits_slot,
        sb.d_final_offsets + n_potentials, sizeof(uint32_t),
        cudaMemcpyDeviceToHost, st));

    // Final scatter: writes COMPACT addresses (not raw) directly into the
    // per-chunk slice of the persistent d_ops_global. The downstream
    // chunk_fml_count_gappy_kernel reads compact addresses, so no later
    // pack-and-shift pass is needed.
    compact_kernel_with_shift<<<g_pot, BLOCK, 0, st>>>(
        sb.d_potentials, sb.d_emit_bits, sb.d_final_offsets, n_potentials, d_chunk_out);
}

// Output of run_preprocess(): the on-device handles PairSortGPU adopts.
// All pointers are aliases into PreprocCtx (no ownership transfer); the
// PreprocCtx is freed by preprocess_cleanup AFTER end-of-pipeline timing.
struct PreprocOut {
    uint32_t* d_ops_global;                // gappy, COMPACT addresses
    uint32_t* d_chunk_starts;              // [n_chunks]   gappy start positions
    uint32_t* d_chunk_lens;                // [n_chunks]   valid length per chunk
    uint32_t* d_packed_chunk_offsets;      // [n_chunks+1] cumulative valid lens
    uint32_t  n_chunks;
    uint32_t  total_valid_ops;             // sum of d_chunk_lens (host scalar)

    uint32_t* d_histogram;                 // [N_ADDR]    fused histogram
    uint32_t  h_max_compact[3];            // per-region max populated index
};

// Big batch of preprocessing state held across the chunk loop. Allocated
// once at startup, freed once after end-of-pipeline timing. Splitting from
// run_preprocess keeps all the cudaMalloc/cudaFree off the critical path
// between e_last_chunk_start and e_metas_ready.
struct PreprocCtx {
    // Static input metadata (filled by setup).
    std::vector<ChunkRef> chunks;
    size_t                total_memops = 0;
    std::vector<size_t>   out_offsets;            // gappy upper-bound offsets, n_chunks+1
    std::vector<uint32_t> n_ram_per_chunk;        // per-chunk RAM event count
    uint32_t              n_chunks = 0;
    size_t                total_bound = 0;        // sum of per-chunk n_potentials

    // Pinned host buffers.
    MemOp*    h_memops      = nullptr;            // raw memops, all chunks
    uint32_t* h_n_emits_all = nullptr;            // per-chunk actual emit count

    // Device allocations.
    StreamBufs sb[ZISK_N_STREAMS];
    uint32_t* d_histogram               = nullptr;   // sized N_ADDR
    uint32_t* d_ops_global              = nullptr;   // sized total_bound (gappy, COMPACT addrs)
    uint32_t* d_gappy_offsets           = nullptr;   // n_chunks; pre-H2D'd at setup
    uint32_t* d_chunk_lens              = nullptr;   // n_chunks; H2D'd post-chunks
    uint32_t* d_packed_chunk_offsets    = nullptr;   // n_chunks+1 cumulative valid lens; H2D'd post-chunks
    ChunkCounters* d_chunk_counters_scratch = nullptr;
    uint32_t* d_invalid_mode_flag       = nullptr;
    uint32_t* d_max_compact             = nullptr;   // [3] populated by fused kernels
};

// Setup phase: enumerate chunks, load raw memops, CPU pre-pass, allocate ALL
// the GPU/pinned state needed by the chunk loop and the pack pass. Runs
// BEFORE the timing window — none of these allocations cost critical-path
// time.
static void preprocess_setup(const std::string& block, PreprocCtx& ctx) {
    const std::string raw_dir = "data/" + block + "_raw";
    auto raw_idxs = list_indices(raw_dir, "mem_count_data_");
    ctx.n_chunks = raw_idxs.size();
    std::cout << "Discovered " << ctx.n_chunks << " chunks for block " << block << std::endl;
    if (ctx.n_chunks == 0) {
        fprintf(stderr, "ERROR: no mem_count_data_*.bin files in %s\n", raw_dir.c_str());
        std::exit(1);
    }

    ctx.chunks.assign(ctx.n_chunks, {});
    ctx.total_memops = 0;
    for (uint32_t c = 0; c < ctx.n_chunks; c++) {
        ctx.chunks[c].file_idx = raw_idxs[c];
        char rp[512]; snprintf(rp, sizeof(rp), "%s/mem_count_data_%u.bin",
                               raw_dir.c_str(), raw_idxs[c]);
        size_t r_bytes = file_size_of(rp);
        if (r_bytes % sizeof(MemOp) != 0) { fprintf(stderr, "ERROR: bad file size\n"); std::exit(1); }
        ctx.chunks[c].n_memops     = r_bytes / sizeof(MemOp);
        ctx.chunks[c].memop_offset = (uint32_t)ctx.total_memops;
        ctx.total_memops += ctx.chunks[c].n_memops;
        if (ctx.chunks[c].n_memops > CHUNK_MAX_MEMOPS) {
            fprintf(stderr, "ERROR: chunk %u has %u > CHUNK_MAX_MEMOPS\n", c, ctx.chunks[c].n_memops);
            std::exit(1);
        }
    }
    std::cout << "Total memops: " << ctx.total_memops << std::endl;

    CUDA_CHECK(cudaMallocHost(&ctx.h_memops, sizeof(MemOp) * ctx.total_memops));
    for (uint32_t c = 0; c < ctx.n_chunks; c++) {
        char rp[512]; snprintf(rp, sizeof(rp), "%s/mem_count_data_%u.bin",
                               raw_dir.c_str(), raw_idxs[c]);
        FILE* f = fopen(rp, "rb");
        if (!f) { fprintf(stderr, "ERROR: cannot open %s\n", rp); std::exit(1); }
        size_t got = fread(ctx.h_memops + ctx.chunks[c].memop_offset,
                           sizeof(MemOp), ctx.chunks[c].n_memops, f);
        fclose(f);
        if (got != ctx.chunks[c].n_memops) {
            fprintf(stderr, "ERROR: short read %s\n", rp); std::exit(1);
        }
    }
    std::cout << "Loaded raw memops" << std::endl;

    // CPU pre-pass: per-chunk n_potentials (upper bound) and n_ram.
    ctx.out_offsets.assign(ctx.n_chunks + 1, 0);
    ctx.n_ram_per_chunk.assign(ctx.n_chunks, 0);
    auto add_pot = [](uint32_t addr, uint32_t count, size_t& pot, uint32_t& ram) {
        pot += count;
        if (is_ram_addr(addr)) ram += count;
    };
    for (uint32_t c = 0; c < ctx.n_chunks; c++) {
        size_t   pot = 0;
        uint32_t ram = 0;
        const MemOp* ops = ctx.h_memops + ctx.chunks[c].memop_offset;
        for (uint32_t k = 0; k < ctx.chunks[c].n_memops; k++) {
            const MemOp& op = ops[k];
            const uint32_t addr    = op.addr;
            const uint32_t aligned = addr & ZISK_ALIGN_MASK;
            const uint8_t  mode    = op.flags & 0x3F;
            const uint32_t off     = addr & 0x07;
            switch (mode) {
                case MOPS_READ_1:                                 add_pot(aligned, 1, pot, ram); break;
                case MOPS_CWRITE_1: case MOPS_WRITE_1:            add_pot(aligned, 2, pot, ram); break;
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
        if (pot > POTENTIAL_CAP_PER_CHUNK) {
            fprintf(stderr, "ERROR: chunk %u needs %zu potentials > cap %u\n",
                    c, pot, POTENTIAL_CAP_PER_CHUNK);
            std::exit(1);
        }
        ctx.out_offsets[c + 1] = ctx.out_offsets[c] + pot;
        ctx.n_ram_per_chunk[c] = ram;
    }
    ctx.total_bound = ctx.out_offsets[ctx.n_chunks];

    // Allocate everything the chunk loop and the pack pass will need.
    for (int i = 0; i < ZISK_N_STREAMS; i++) alloc_stream_bufs(ctx.sb[i]);

    CUDA_CHECK(cudaMalloc(&ctx.d_histogram,    (size_t)N_ADDR * sizeof(uint32_t)));
    CUDA_CHECK(cudaMemset(ctx.d_histogram, 0,  (size_t)N_ADDR * sizeof(uint32_t)));

    CUDA_CHECK(cudaMalloc(&ctx.d_ops_global,   sizeof(uint32_t) * ctx.total_bound));

    CUDA_CHECK(cudaMalloc(&ctx.d_gappy_offsets,  sizeof(uint32_t) * ctx.n_chunks));
    CUDA_CHECK(cudaMalloc(&ctx.d_chunk_lens,     sizeof(uint32_t) * ctx.n_chunks));
    CUDA_CHECK(cudaMalloc(&ctx.d_packed_chunk_offsets, sizeof(uint32_t) * (ctx.n_chunks + 1)));

    // gappy_offsets is known at setup time — pre-H2D it now.
    std::vector<uint32_t> gappy_u32(ctx.n_chunks);
    for (uint32_t c = 0; c < ctx.n_chunks; c++)
        gappy_u32[c] = (uint32_t)ctx.out_offsets[c];
    CUDA_CHECK(cudaMemcpy(ctx.d_gappy_offsets, gappy_u32.data(),
                          sizeof(uint32_t) * ctx.n_chunks, cudaMemcpyHostToDevice));

    CUDA_CHECK(cudaMalloc(&ctx.d_chunk_counters_scratch, sizeof(ChunkCounters)));
    CUDA_CHECK(cudaMemset(ctx.d_chunk_counters_scratch, 0, sizeof(ChunkCounters)));

    CUDA_CHECK(cudaMalloc(&ctx.d_invalid_mode_flag, sizeof(uint32_t)));
    CUDA_CHECK(cudaMemset(ctx.d_invalid_mode_flag, 0, sizeof(uint32_t)));

    CUDA_CHECK(cudaMalloc(&ctx.d_max_compact, 3 * sizeof(uint32_t)));
    CUDA_CHECK(cudaMemset(ctx.d_max_compact, 0, 3 * sizeof(uint32_t)));

    CUDA_CHECK(cudaMallocHost(&ctx.h_n_emits_all, sizeof(uint32_t) * ctx.n_chunks));

    std::cout << "On-device gappy output bound: "
              << (ctx.total_bound * 4) / (1024*1024) << " MB" << std::endl;
}

static void preprocess_cleanup(PreprocCtx& ctx) {
    cudaFree(ctx.d_chunk_counters_scratch);
    cudaFree(ctx.d_invalid_mode_flag);
    cudaFree(ctx.d_max_compact);
    cudaFree(ctx.d_gappy_offsets);
    cudaFree(ctx.d_chunk_lens);
    cudaFree(ctx.d_packed_chunk_offsets);
    cudaFree(ctx.d_ops_global);
    cudaFree(ctx.d_histogram);
    cudaFreeHost(ctx.h_memops);
    cudaFreeHost(ctx.h_n_emits_all);
    for (int i = 0; i < ZISK_N_STREAMS; i++) free_stream_bufs(ctx.sb[i]);
}

// Critical-path chunk pipeline: enqueue all chunks, then build the packed
// compact d_ops_packed via the device pack kernel. NO cudaMalloc / cudaFree
// inside this function — everything was front-loaded by preprocess_setup
// and lives in `ctx`. Returns PreprocOut with handles into ctx's buffers.
static PreprocOut run_preprocess(PreprocCtx& ctx,
                                 cudaEvent_t e_last_chunk_start) {
    auto t0 = std::chrono::steady_clock::now();
    for (uint32_t c = 0; c < ctx.n_chunks; c++) {
        int s = c % ZISK_N_STREAMS;
        uint32_t n_pot = (uint32_t)(ctx.out_offsets[c + 1] - ctx.out_offsets[c]);
        if (c == ctx.n_chunks - 1) {
            CUDA_CHECK(cudaEventRecord(e_last_chunk_start, ctx.sb[s].stream));
        }
        run_chunk(ctx.sb[s],
                  ctx.h_memops + ctx.chunks[c].memop_offset,
                  ctx.chunks[c].n_memops,
                  n_pot,
                  ctx.n_ram_per_chunk[c],
                  ctx.d_chunk_counters_scratch,
                  ctx.d_invalid_mode_flag,
                  ctx.d_histogram,
                  ctx.d_max_compact,
                  ctx.d_ops_global + ctx.out_offsets[c],
                  &ctx.h_n_emits_all[c]);
    }
    for (int i = 0; i < ZISK_N_STREAMS; i++)
        CUDA_CHECK(cudaStreamSynchronize(ctx.sb[i].stream));
    auto t1 = std::chrono::steady_clock::now();
    double ms = std::chrono::duration<double, std::milli>(t1 - t0).count();
    std::cout << "Preprocess GPU pipeline: " << ms << " ms" << std::endl;

    // No pack pass: chunk_fml_count_gappy_kernel reads d_ops_global directly
    // using gappy_offsets (already on device, pre-H2D'd at setup) plus
    // chunk_lens and packed_chunk_offsets H2D'd via the small async copies
    // below. packed_chunk_offsets is the cumulative valid-length array used
    // by the grid-stride kernel to map a "valid index" → (chunk, offset).
    PreprocOut po;
    po.n_chunks = ctx.n_chunks;

    std::vector<uint32_t> packed_off_h(ctx.n_chunks + 1, 0);
    for (uint32_t c = 0; c < ctx.n_chunks; c++)
        packed_off_h[c + 1] = packed_off_h[c] + ctx.h_n_emits_all[c];
    po.total_valid_ops = packed_off_h[ctx.n_chunks];

    CUDA_CHECK(cudaMemcpyAsync(ctx.d_chunk_lens, ctx.h_n_emits_all,
                               sizeof(uint32_t) * ctx.n_chunks,
                               cudaMemcpyHostToDevice, ctx.sb[0].stream));
    CUDA_CHECK(cudaMemcpyAsync(ctx.d_packed_chunk_offsets, packed_off_h.data(),
                               sizeof(uint32_t) * (ctx.n_chunks + 1),
                               cudaMemcpyHostToDevice, ctx.sb[0].stream));

    po.d_ops_global           = ctx.d_ops_global;
    po.d_chunk_starts         = ctx.d_gappy_offsets;
    po.d_chunk_lens           = ctx.d_chunk_lens;
    po.d_packed_chunk_offsets = ctx.d_packed_chunk_offsets;
    po.d_histogram            = ctx.d_histogram;

    // D2H per-region max populated compact index. Hits at this point because
    // all chunks finished their fused histogram updates above.
    CUDA_CHECK(cudaMemcpy(po.h_max_compact, ctx.d_max_compact,
                          3 * sizeof(uint32_t), cudaMemcpyDeviceToHost));

    std::cout << "Valid ops total: " << po.total_valid_ops
              << "  Max compact: ROM=" << po.h_max_compact[0]
              << " INPUT=" << po.h_max_compact[1]
              << " RAM=" << po.h_max_compact[2] << std::endl;

    return po;
}

// =====================================================================
// PairSortGPU (copied from main_real.cu, sans cpu_fill / reference_sort
// / verify which are not on the metas-only critical path)
// =====================================================================

class PairSortGPU {
    // External state from preprocessing — gappy layout, no copy.
    uint32_t* d_ops;                  // gappy d_ops_global, COMPACT addresses
    uint32_t* d_chunk_starts;         // [num_chunks] gappy start positions
    uint32_t* d_chunk_lens;           // [num_chunks] valid length per chunk
    uint32_t* d_packed_chunk_offsets; // [num_chunks+1] cumulative valid lens
    uint32_t* d_prefix;
    void*     d_temp;
    size_t    d_temp_bytes;
    uint32_t* d_active_ids;
    uint32_t* d_active_first;
    uint32_t* d_active_last;
    uint32_t* d_fml;
    uint32_t* d_result_nops;
    uint32_t* d_meta_scalars;
    uint32_t* d_addr_offsets;
    uint32_t* d_offset_starts;
    // d_chunk_offsets removed: chunk_fml_count_gappy_kernel uses
    // d_chunk_starts and d_chunk_lens (external from preprocessing) instead.

    cudaStream_t d2h_stream;
    cudaStream_t meta_stream;

    uint32_t* h_offsets_buf;
    size_t    h_offsets_buf_size;
    uint32_t* h_result_nops;
    uint32_t* h_meta_scalars;

    uint32_t active_mask[MASK_WORDS];
    uint32_t h_active_local_ids[MAX_ACTIVE];
    std::vector<uint32_t> h_active_first;
    std::vector<uint32_t> h_active_last;
    std::vector<InstanceMeta> metas;

    uint32_t num_ops;
    uint32_t num_chunks;
    uint32_t num_instances;
    uint32_t num_active;

    uint32_t num_inst[3];
    uint32_t num_active_per[3];
    uint32_t active_offset[3];
    uint32_t region_ops_start[3];
    uint32_t region_n_ops[3];

    // Level-1+2 fusion: external pointers from preprocessing.
    uint32_t* d_external_hist = nullptr;
    uint32_t  h_max_compact[3] = {0, 0, 0};   // per-region max populated index

public:
    PairSortGPU();
    ~PairSortGPU();

    // Adopt fully-prepared device state from preprocessing:
    //   d_ops_global     — gappy compact-address ops on device
    //   d_chunk_starts   — [n_chunks] gappy start positions (device)
    //   d_chunk_lens     — [n_chunks] valid length per chunk (device)
    //   n_chunks         — number of chunks
    //   total_valid_ops  — sum of d_chunk_lens
    //   d_histogram      — pre-built compact-address histogram
    //   h_max_compact    — per-region max populated compact index
    void adopt_device_state(uint32_t* d_ops_global,
                            uint32_t* d_chunk_starts,
                            uint32_t* d_chunk_lens,
                            uint32_t* d_packed_chunk_offsets,
                            uint32_t  n_chunks_in,
                            uint32_t  total_valid_ops,
                            uint32_t* d_histogram,
                            const uint32_t h_max_compact[3]);

    // Once-per-block work: prefix sum on the histogram, region/instance
    // counts. Independent of which worker subset we look at — must run
    // exactly once before any gpu_metadata() call.
    void prepare_global();

    // Per-worker work: pick active instances for the current worker mask,
    // compute their boundaries + FML counts, build their metas. Reads the
    // global state set up by prepare_global(). Must be called between
    // set_active_worker() and reading metas_ptr().
    void gpu_metadata();

    const InstanceMeta* metas_ptr()           const { return metas.data(); }
    uint32_t num_active_instances()           const { return num_active; }

    void set_active_worker(uint32_t w) {
        std::memset(active_mask, 0, sizeof(active_mask));
        for (uint32_t i = 0; i < MAX_INSTANCES; i++)
            if (i % N_WORKERS == w)
                active_mask[i / 32] |= (1u << (i % 32));
    }

private:
    void pick_active_instances();
};

PairSortGPU::PairSortGPU()
    : d_ops(nullptr),
      h_active_first(MAX_ACTIVE), h_active_last(MAX_ACTIVE), metas(MAX_ACTIVE),
      num_ops(0), num_chunks(0), num_instances(0), num_active(0),
      num_inst{}, num_active_per{}, active_offset{}, region_ops_start{}, region_n_ops{}
{
    cudaMalloc(&d_prefix,        (size_t)(N_ADDR + 1) * sizeof(uint32_t));

    d_temp = nullptr;
    d_temp_bytes = 0;
    cub::DeviceScan::ExclusiveSum(d_temp, d_temp_bytes,
        (uint32_t*)nullptr, (uint32_t*)nullptr, N_ADDR);
    cudaMalloc(&d_temp, d_temp_bytes);

    cudaMalloc(&d_active_ids,    MAX_ACTIVE * sizeof(uint32_t));
    cudaMalloc(&d_active_first,  MAX_ACTIVE * sizeof(uint32_t));
    cudaMalloc(&d_active_last,   MAX_ACTIVE * sizeof(uint32_t));
    cudaMalloc(&d_fml,           (size_t)MAX_ACTIVE * MAX_CHUNKS * 3 * sizeof(uint32_t));
    cudaMalloc(&d_result_nops,   (size_t)MAX_ACTIVE * MAX_CHUNKS * sizeof(uint32_t));
    cudaMalloc(&d_meta_scalars,  (size_t)MAX_ACTIVE * 4 * sizeof(uint32_t));
    cudaMalloc(&d_addr_offsets,  (size_t)MAX_ACTIVE * (INSTANCE_SIZE_MAX + 1) * sizeof(uint32_t));
    cudaMalloc(&d_offset_starts, MAX_ACTIVE * sizeof(uint32_t));

    cudaStreamCreate(&d2h_stream);
    cudaStreamCreate(&meta_stream);

    h_offsets_buf_size = 1ull << 28;  // 256 MB
    cudaMallocHost(&h_offsets_buf,   h_offsets_buf_size);
    cudaMallocHost(&h_result_nops,   (size_t)MAX_ACTIVE * MAX_CHUNKS * sizeof(uint32_t));
    cudaMallocHost(&h_meta_scalars,  (size_t)MAX_ACTIVE * 4 * sizeof(uint32_t));
}

PairSortGPU::~PairSortGPU() {
    cudaFree(d_prefix); cudaFree(d_temp);
    cudaFree(d_active_ids); cudaFree(d_active_first); cudaFree(d_active_last);
    cudaFree(d_fml); cudaFree(d_result_nops); cudaFree(d_meta_scalars);
    cudaFree(d_addr_offsets); cudaFree(d_offset_starts);

    cudaStreamDestroy(d2h_stream);
    cudaStreamDestroy(meta_stream);

    cudaFreeHost(h_offsets_buf);
    cudaFreeHost(h_result_nops); cudaFreeHost(h_meta_scalars);
}

void PairSortGPU::pick_active_instances() {
    uint32_t pos = 0;
    uint32_t gid_base = 0;
    for (uint8_t r = 0; r < 3; r++) {
        num_active_per[r] = 0;
        active_offset[r] = pos;
        for (uint32_t lid = 0; lid < num_inst[r]; lid++) {
            uint32_t gid = gid_base + lid;
            if (active_mask[gid / 32] & (1u << (gid % 32)))
                h_active_local_ids[pos + num_active_per[r]++] = lid;
        }
        pos += num_active_per[r];
        gid_base += num_inst[r];
    }
    num_active = num_active_per[0] + num_active_per[1] + num_active_per[2];
    cudaMemcpy(d_active_ids, h_active_local_ids,
               num_active * sizeof(uint32_t), cudaMemcpyHostToDevice);
}

void PairSortGPU::adopt_device_state(uint32_t* d_ops_global,
                                     uint32_t* d_chunk_starts_in,
                                     uint32_t* d_chunk_lens_in,
                                     uint32_t* d_packed_chunk_offsets_in,
                                     uint32_t  n_chunks_in,
                                     uint32_t  total_valid_ops,
                                     uint32_t* d_histogram,
                                     const uint32_t hmax[3]) {
    num_chunks = n_chunks_in;
    num_ops    = total_valid_ops;
    if (num_ops == 0) {
        std::cerr << "ERROR: no ops" << std::endl; std::exit(1);
    }
    if (num_ops > MAX_OPS) {
        std::cerr << "ERROR: " << num_ops << " ops > MAX_OPS=" << MAX_OPS << std::endl;
        std::exit(1);
    }
    if (num_chunks > MAX_CHUNKS) {
        std::cerr << "ERROR: " << num_chunks << " chunks > MAX_CHUNKS=" << MAX_CHUNKS << std::endl;
        std::exit(1);
    }
    d_ops                  = d_ops_global;
    d_chunk_starts         = d_chunk_starts_in;
    d_chunk_lens           = d_chunk_lens_in;
    d_packed_chunk_offsets = d_packed_chunk_offsets_in;
    d_external_hist        = d_histogram;
    h_max_compact[0] = hmax[0];
    h_max_compact[1] = hmax[1];
    h_max_compact[2] = hmax[2];
}

void PairSortGPU::prepare_global() {
    // Three scoped prefix sums on the populated [region_start, region_start
    // + max + 2) slices of d_external_hist. INPUT and RAM scans add an
    // inter-region offset so the unified d_prefix layout still reflects
    // cumulative-across-regions values. Total scan work scales with
    // populated cells (~tens of millions) instead of N_ADDR (1.6 G).
    {
        uint32_t n_rom = h_max_compact[REGION_ROM] + 2;
        cub::DeviceScan::ExclusiveSum(d_temp, d_temp_bytes,
            d_external_hist + 0, d_prefix + 0, n_rom);

        uint32_t n_in = h_max_compact[REGION_INPUT] + 2;
        cub::DeviceScan::ExclusiveSum(d_temp, d_temp_bytes,
            d_external_hist + N_ADDR_ROM, d_prefix + N_ADDR_ROM, n_in);
        add_const_kernel<<<(n_in + 255) / 256, 256>>>(
            d_prefix + N_ADDR_ROM,
            d_prefix + h_max_compact[REGION_ROM] + 1,    // total_rom
            n_in);

        uint32_t n_ram = h_max_compact[REGION_RAM] + 2;
        cub::DeviceScan::ExclusiveSum(d_temp, d_temp_bytes,
            d_external_hist + N_ADDR_ROM + N_ADDR_INPUT,
            d_prefix + N_ADDR_ROM + N_ADDR_INPUT, n_ram);
        add_const_kernel<<<(n_ram + 255) / 256, 256>>>(
            d_prefix + N_ADDR_ROM + N_ADDR_INPUT,
            d_prefix + N_ADDR_ROM + h_max_compact[REGION_INPUT] + 1,
            n_ram);
    }

    // Region totals from the per-region scan tails.
    uint32_t h_boundary[3];
    cudaMemcpy(&h_boundary[0],
               d_prefix + h_max_compact[REGION_ROM] + 1,
               sizeof(uint32_t), cudaMemcpyDeviceToHost);
    cudaMemcpy(&h_boundary[1],
               d_prefix + N_ADDR_ROM + h_max_compact[REGION_INPUT] + 1,
               sizeof(uint32_t), cudaMemcpyDeviceToHost);
    h_boundary[2] = num_ops;

    region_n_ops[REGION_ROM]   = h_boundary[0];
    region_n_ops[REGION_INPUT] = h_boundary[1] - h_boundary[0];
    region_n_ops[REGION_RAM]   = h_boundary[2] - h_boundary[1];

    num_instances = 0;
    for (uint8_t r = 0; r < 3; r++) {
        num_inst[r] = region_n_ops[r] ? (region_n_ops[r] + INSTANCE_SIZE[r] - 1) / INSTANCE_SIZE[r] : 0;
        if (num_inst[r] > MAX_INST[r]) {
            std::cerr << "ERROR: too many instances in " << REGION_NAME[r]
                      << " (" << num_inst[r] << " > " << MAX_INST[r] << ")" << std::endl;
            std::exit(1);
        }
        num_instances += num_inst[r];
    }

    region_ops_start[REGION_ROM]   = 0;
    region_ops_start[REGION_INPUT] = h_boundary[0];
    region_ops_start[REGION_RAM]   = h_boundary[1];
}

void PairSortGPU::gpu_metadata() {
    // Per-worker only. prepare_global() must have run already.
    pick_active_instances();

    // Per-worker d_fml accumulator: one slot per (active, chunk, first/middle/last).
    cudaMemset(d_fml, 0, (size_t)num_active * num_chunks * 3 * sizeof(uint32_t));

    // Per-region instance boundaries (binary searches in d_prefix). Cheap.
    for (uint8_t r = 0; r < 3; r++) {
        if (num_active_per[r] == 0) continue;
        uint32_t na  = num_active_per[r];
        uint32_t off = active_offset[r];
        instance_boundaries_kernel<<<1, na>>>(
            d_prefix, REGION_ADDR_START[r], h_max_compact[r] + 1,
            region_n_ops[r], INSTANCE_SIZE[r],
            d_active_ids + off, d_active_first + off, d_active_last + off,
            d_offset_starts + off, na);
    }

    // Single grid-stride pass over the gappy d_ops_global counting per
    // (active instance, chunk) first/middle/last hits. cudaOccupancyMax-
    // PotentialBlockSize picks block + grid for max SM occupancy — same
    // pattern as main_real.cu's chunk_fml_count_kernel.
    int fml_block, fml_grid;
    cudaOccupancyMaxPotentialBlockSize(&fml_grid, &fml_block,
        chunk_fml_count_gappy_kernel, 0, 0);
    chunk_fml_count_gappy_kernel<<<fml_grid, fml_block>>>(
        d_ops, d_chunk_starts, d_packed_chunk_offsets,
        d_active_first, d_active_last,
        d_fml, num_active, num_chunks, num_ops);

    // host_active_first/last are needed below to compute h_offset_starts
    // (cumulative addr_offsets-array slot per instance) before launching
    // build_metas / compute_addr_offsets.
    cudaDeviceSynchronize();
    cudaMemcpy(h_active_first.data(), d_active_first, num_active * sizeof(uint32_t), cudaMemcpyDeviceToHost);
    cudaMemcpy(h_active_last.data(),  d_active_last,  num_active * sizeof(uint32_t), cudaMemcpyDeviceToHost);

    std::vector<uint32_t> h_offset_starts(num_active);
    uint32_t total_addrs = 0;
    for (uint32_t i = 0; i < num_active; i++) {
        h_offset_starts[i] = total_addrs;
        total_addrs += h_active_last[i] - h_active_first[i] + 1;
    }

    // build_metas (meta_stream) and compute_addr_offsets (d2h_stream)
    // overlap because they touch disjoint device buffers.
    for (uint8_t r = 0; r < 3; r++) {
        if (num_active_per[r] == 0) continue;
        uint32_t na  = num_active_per[r];
        uint32_t off = active_offset[r];

        build_metas_kernel<<<na, 256, 0, meta_stream>>>(
            d_fml + (size_t)off * num_chunks * 3, d_prefix,
            REGION_ADDR_START[r], region_n_ops[r], INSTANCE_SIZE[r],
            d_active_ids + off, d_active_first + off, d_active_last + off,
            d_result_nops + (size_t)off * num_chunks,
            d_meta_scalars + off * 4, na, num_chunks);

        compute_addr_offsets_kernel<<<na, 1024, 0, d2h_stream>>>(
            d_prefix, REGION_ADDR_START[r], region_n_ops[r], INSTANCE_SIZE[r],
            d_active_ids + off, d_active_first + off, d_active_last + off,
            d_addr_offsets, d_offset_starts + off, na);
    }

    cudaMemcpyAsync(h_meta_scalars, d_meta_scalars,
                    num_active * 4 * sizeof(uint32_t), cudaMemcpyDeviceToHost, meta_stream);
    cudaMemcpyAsync(h_result_nops, d_result_nops,
                    (size_t)num_active * num_chunks * sizeof(uint32_t), cudaMemcpyDeviceToHost, meta_stream);
    cudaMemcpyAsync(h_offsets_buf, d_addr_offsets,
                    (size_t)total_addrs * sizeof(uint32_t), cudaMemcpyDeviceToHost, d2h_stream);

    cudaStreamSynchronize(meta_stream);
    cudaStreamSynchronize(d2h_stream);

    // Populate InstanceMeta structs
    uint32_t ai = 0;
    for (uint8_t r = 0; r < 3; r++) {
        for (uint32_t j = 0; j < num_active_per[r]; j++, ai++) {
            uint32_t* scalars      = h_meta_scalars + ai * 4;
            metas[ai].inst_id      = h_active_local_ids[active_offset[r] + j];
            metas[ai].type         = r;
            metas[ai].first_addr   = expand_addr(h_active_first[ai]);
            metas[ai].last_addr    = expand_addr(h_active_last[ai]);
            metas[ai].first_addr_chunk  = scalars[0];
            metas[ai].first_addr_skip   = scalars[1];
            metas[ai].last_addr_chunk   = scalars[2];
            metas[ai].last_addr_include = scalars[3];
            uint32_t num_addrs = h_active_last[ai] - h_active_first[ai] + 1;
            metas[ai].count_per_chunk = {h_result_nops + (size_t)ai * num_chunks, num_chunks};
            metas[ai].addr_offsets   = {h_offsets_buf + h_offset_starts[ai], num_addrs};
        }
    }
}

// =====================================================================
// main
// =====================================================================

int main(int argc, char** argv) {
    if (argc < 2) {
        fprintf(stderr,
                "Usage: %s <block_number> [--save-metas <path>] [--verify-metas <path>]\n",
                argv[0]);
        return 1;
    }
    const std::string block = argv[1];
    std::string save_path, verify_path;
    for (int i = 2; i < argc; i++) {
        if (strcmp(argv[i], "--save-metas") == 0 && i + 1 < argc) {
            save_path = argv[++i];
        } else if (strcmp(argv[i], "--verify-metas") == 0 && i + 1 < argc) {
            verify_path = argv[++i];
        } else {
            fprintf(stderr, "unknown arg: %s\n", argv[i]);
            return 1;
        }
    }

    // ─── Setup phase (all heavy allocations) — OFF the critical path ───
    // Pre-load reference if verifying.
    std::vector<RefMeta> ref;
    if (!verify_path.empty()) {
        ref = load_ref_metas(verify_path);
        std::cout << "Loaded reference metas: " << ref.size()
                  << " entries from " << verify_path << std::endl;
    }
    FILE* save_f = nullptr;
    if (!save_path.empty()) save_f = save_metas_begin(save_path);

    // PairSortGPU constructor allocates ~6.5 GB of device memory (d_prefix,
    // d_temp, etc). Construct BEFORE the timing events so this stays out of
    // the last-chunk-to-final window.
    PairSortGPU app;

    // Allocate ALL preprocessing state up-front — chunk metadata loaded,
    // pinned h_memops + h_n_emits_all, StreamBufs ×4, d_histogram (~6.5 GB),
    // d_ops_global (~1.4 GB), d_ops_packed (~1.4 GB), pack-metadata arrays,
    // d_invalid_mode_flag, d_chunk_counters_scratch.
    PreprocCtx ctx;
    preprocess_setup(block, ctx);

    // Last-chunk-to-final: e_last_chunk_start fires the moment the LAST
    // raw chunk's H2D enters its stream (set by run_preprocess); e_metas_ready
    // fires after the final worker pass's metas have landed in pinned host.
    // Critical-path instrumentation: events at each handoff boundary so we
    // can attribute the last-chunk-to-final time to specific stages.
    cudaEvent_t e_last_chunk_start, e_after_preproc, e_after_prepare, e_metas_ready;
    CUDA_CHECK(cudaEventCreate(&e_last_chunk_start));
    CUDA_CHECK(cudaEventCreate(&e_after_preproc));
    CUDA_CHECK(cudaEventCreate(&e_after_prepare));
    CUDA_CHECK(cudaEventCreate(&e_metas_ready));

    // ─── Critical-path phase ───
    PreprocOut po = run_preprocess(ctx, e_last_chunk_start);
    CUDA_CHECK(cudaEventRecord(e_after_preproc, 0));

    app.adopt_device_state(po.d_ops_global, po.d_chunk_starts, po.d_chunk_lens,
                           po.d_packed_chunk_offsets,
                           po.n_chunks, po.total_valid_ops,
                           po.d_histogram, po.h_max_compact);
    app.prepare_global();        // prefix sum + region counts (once)
    CUDA_CHECK(cudaEventRecord(e_after_prepare, 0));

    // Production performance is ONE worker. Only when the user asks for
    // --save-metas or --verify-metas do we run all N_WORKERS — that's
    // verification overhead, not the real critical path.
    const bool need_all_workers = save_f != nullptr || !verify_path.empty();
    const uint32_t n_passes = need_all_workers ? N_WORKERS : 1u;

    auto t0 = std::chrono::steady_clock::now();
    uint32_t total_metas = 0;
    uint32_t ref_idx     = 0;
    bool any_fail = false;
    for (uint32_t w = 0; w < n_passes; w++) {
        app.set_active_worker(w);
        app.gpu_metadata();
        // Record e_metas_ready right after the FIRST worker completes —
        // that's the real production "last-chunk-to-final" metric. Any
        // subsequent worker passes are verification/save overhead.
        if (w == 0) {
            CUDA_CHECK(cudaEventRecord(e_metas_ready, 0));
            CUDA_CHECK(cudaEventSynchronize(e_metas_ready));
        }

        const InstanceMeta* mp = app.metas_ptr();
        const uint32_t      n  = app.num_active_instances();

        if (save_f) {
            for (uint32_t i = 0; i < n; i++) save_metas_append(save_f, mp[i]);
        }
        if (!verify_path.empty()) {
            for (uint32_t i = 0; i < n; i++) {
                if (ref_idx >= ref.size()) {
                    fprintf(stderr,
                            "VERIFY: more metas produced than reference has at worker %u idx %u\n",
                            w, i);
                    any_fail = true; break;
                }
                std::string err;
                if (!compare_meta(mp[i], ref[ref_idx], err)) {
                    fprintf(stderr,
                            "VERIFY MISMATCH worker=%u meta=%u (inst_id=%u type=%u): %s\n",
                            w, i, mp[i].inst_id, mp[i].type, err.c_str());
                    any_fail = true;
                    goto verify_done;
                }
                ref_idx++;
            }
        }
        total_metas += n;
    }
verify_done:
    auto t1 = std::chrono::steady_clock::now();
    double pipeline_ms = std::chrono::duration<double, std::milli>(t1 - t0).count();
    std::cout << "PairSortGPU " << n_passes
              << "-worker pipeline: " << pipeline_ms << " ms" << std::endl;

    // Headline metric: GPU wall time between the LAST raw chunk's H2D
    // entering its preprocessing stream and worker 0's metas landing in
    // pinned host memory. Three sub-intervals show where the time goes.
    float lcr_ms = 0.f, t_preproc = 0.f, t_prepare = 0.f, t_worker = 0.f;
    CUDA_CHECK(cudaEventElapsedTime(&lcr_ms,    e_last_chunk_start, e_metas_ready));
    CUDA_CHECK(cudaEventElapsedTime(&t_preproc, e_last_chunk_start, e_after_preproc));
    CUDA_CHECK(cudaEventElapsedTime(&t_prepare, e_after_preproc,    e_after_prepare));
    CUDA_CHECK(cudaEventElapsedTime(&t_worker,  e_after_prepare,    e_metas_ready));
    std::cout << "Last-chunk-to-final (1 worker): " << lcr_ms << " ms"
              << "  [preproc-tail " << t_preproc
              << " | prepare_global " << t_prepare
              << " | worker " << t_worker << "]" << std::endl;

    if (save_f) {
        save_metas_end(save_f, total_metas);
        std::cout << "Saved " << total_metas << " InstanceMetas → " << save_path
                  << " (across " << N_WORKERS << " worker passes)" << std::endl;
    }

    if (!verify_path.empty()) {
        if (any_fail) {
            std::cout << "VERIFY (metas): FAIL" << std::endl;
            return 1;
        }
        if (ref_idx != ref.size()) {
            std::cout << "VERIFY (metas): FAIL — produced " << ref_idx
                      << " metas, reference has " << ref.size() << std::endl;
            return 1;
        }
        std::cout << "VERIFY (metas): OK " << ref_idx
                  << " instances match " << verify_path << std::endl;
    }

    // ─── Cleanup (off the critical path, after timing) ───
    preprocess_cleanup(ctx);
    cudaEventDestroy(e_last_chunk_start);
    cudaEventDestroy(e_after_preproc);
    cudaEventDestroy(e_after_prepare);
    cudaEventDestroy(e_metas_ready);
    return 0;
}
