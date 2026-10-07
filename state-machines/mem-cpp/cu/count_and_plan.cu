// =====================================================================
// CountAndPlan — implementation
// =====================================================================

#include "count_and_plan.cuh"
#include "../cpp/mops_format.hpp"

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

// Decodes the record starting at word `start`. `flags` loses the tag and the relocated top bit;
// the payload gets the value's bit 63 back (a no-op for step-field payloads).
__host__ __device__ __forceinline__ MemOp load_record(const uint64_t* words, uint32_t start) {
    const uint64_t hdr = words[start];
    MemOp op;
    op.addr  = (uint32_t)hdr;
    op.flags = (uint32_t)(hdr >> 32) & 0x3FFFFFFFu;
    op.payload = mops_record_len(hdr) == 1 ? 0 : (words[start + 1] | ((hdr >> 62) & 1ull) << 63);
    return op;
}

// Device fault word, checked once per block before the prefix scan: bits in word 0, the compact
// address of a counter overflow in word 1.
constexpr uint32_t INVALID_MODE     = 1u;   // unrecognised record mode
constexpr uint32_t INVALID_ADDRESS  = 2u;   // access outside the memory map
constexpr uint32_t COUNTER_OVERFLOW = 4u;   // more than 2^32 - 1 rows at one address
constexpr uint32_t LIGHT_RECORD     = 8u;   // a record of the light form (no value or step): no device witness

__device__ __forceinline__ void hist_add(uint32_t* counter, uint32_t n, uint32_t compact,
                                         uint32_t* d_fault) {
    const uint32_t old = atomicAdd(counter, n);
    if (old + n < old) {
        atomicOr(d_fault, COUNTER_OVERFLOW);
        atomicExch(d_fault + 1, compact);
    }
}

// Per-chunk capacity for block-op spill entries. Must hold every memop that
// could be a block-op spill candidate, which in the worst case is every memop
// in the piece, so this tracks the words per piece.
constexpr uint32_t MAX_BLOCKOP_SPILL_PER_CHUNK = MAX_WORDS_PER_PIECE;   // records <= words
static_assert(MAX_BLOCKOP_SPILL_PER_CHUNK >= MAX_WORDS_PER_PIECE,
              "MAX_BLOCKOP_SPILL_PER_CHUNK must accommodate the worst case "
              "(every memop in the chunk being a block-op spill candidate)");
constexpr uint32_t BLOCKOP_SPILL_THRESH_VAL    = 64u;

// =====================================================================
// Full definitions for the types forward-declared in the header.
// =====================================================================

// One aligned access a record expands to. `meta` packs what the RAM witness needs about it:
//   bits 0-1   kind: 0 read, 1 full write, 2 partial write (bytes [off, off+width)), 3 block write
//              whose value the stream does not carry
//   bits 2-4   byte offset of a partial write
//   bits 5-8   byte width of a partial write
//   bits 9-28  (step_in_chunk << 2) | slot
// `value` is the written word for a full write, or the write's bytes already shifted into place for
// a partial one.
struct __align__(8) PotentialEmit {
    uint32_t aligned_addr_packed;
    uint32_t meta;
    uint64_t value;
};
constexpr uint32_t POT_KIND_READ    = 0;
constexpr uint32_t POT_KIND_WRITE   = 1;
constexpr uint32_t POT_KIND_PARTIAL = 2;
constexpr uint32_t POT_KIND_UNKNOWN = 3;
constexpr uint32_t POT_META_OFF_SHIFT   = 2;
constexpr uint32_t POT_META_WIDTH_SHIFT = 5;
constexpr uint32_t POT_META_STEP_SHIFT  = 9;
constexpr uint32_t MOPS_STEP_FIELD_MASK = (1u << 20) - 1;
// Step field of a record: bits 38-57 of the header for non-block modes, of the payload for blocks.
__host__ __device__ __forceinline__ uint32_t mops_step_field(const MemOp& op, bool block) {
    return block ? (uint32_t)((op.payload >> 38) & MOPS_STEP_FIELD_MASK)
                 : (uint32_t)((op.flags >> 6) & MOPS_STEP_FIELD_MASK);
}
__host__ __device__ __forceinline__ uint32_t pot_meta(uint32_t kind, uint32_t off, uint32_t width, uint32_t step) {
    return kind | (off << POT_META_OFF_SHIFT) | (width << POT_META_WIDTH_SHIFT) | (step << POT_META_STEP_SHIFT);
}

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
    uint64_t payload;   // the record's payload: its step field
};


// 64-bit RAM sort-key bit layout (see kernels below).
//   bit 0        : kind_w
//   bits 1..23   : orig_pos (potential index + 1; 0 marks a carry entry from the previous piece,
//                  whose kind bit holds the pairing state)
//   bits 24..49  : compact_ram (RAM word index, < 2^26)
// orig_pos must span the full potential range of a piece; too few bits would let the top index
// bit bleed into compact_ram. The static_asserts below pin orig_pos width and the sort end-bit
// to MAX_POT_PER_PIECE and RAM size.
#define KIND_W_BIT          0u
#define ORIG_POS_SHIFT      1u
#define ORIG_POS_BITS       23u
#define ORIG_POS_MASK       ((1u << ORIG_POS_BITS) - 1u)
#define COMPACT_ADDR_SHIFT  (ORIG_POS_SHIFT + ORIG_POS_BITS)   // 24
#define RAM_KEY_END_BIT     50
#define CARRY_POS           0x7FFFFFFFu   // packed orig_pos of a carry entry

// orig_pos must represent every potential index in a chunk without truncation.
static_assert((1ull << ORIG_POS_BITS) > (uint64_t)MAX_POT_PER_PIECE,
              "ORIG_POS_BITS too small: orig_pos would overflow into compact_ram");
// the radix sort (bits [0, RAM_KEY_END_BIT)) must cover the whole compact_ram
// field sitting above orig_pos.
static_assert((((uint64_t)(ZISK_RAM_SIZE_BYTES >> 3) - 1) << COMPACT_ADDR_SHIFT)
                  < (1ull << RAM_KEY_END_BIT),
              "RAM_KEY_END_BIT too small to cover compact_ram above orig_pos");

#define CUDA_CHECK(call) do {                                                  \
    cudaError_t _err = (call);                                                 \
    if (_err != cudaSuccess) {                                                 \
        fprintf(stderr, "CUDA error %s at %s:%d: %s\n",                        \
                cudaGetErrorString(_err), __FILE__, __LINE__, #call);          \
        exit(1);                                                               \
    }                                                                          \
} while (0)

// Check for a kernel-launch error at the launch site (host-side, no device sync).
// Catches a bad launch config immediately instead of letting it surface,
// misattributed, at a later synchronizing CUDA call.
#define CUDA_CHECK_LAUNCH() CUDA_CHECK(cudaGetLastError())

// Binds `dev` (the GPU that owns our buffers) for the calling thread and
// leaves it bound; negative = keep the current device. Deliberately no
// save/restore of the caller's device: cudaGetDevice on a thread that never
// touched CUDA reports the default device 0, and re-binding that on exit
// would create a ~0.5-1 GB context on a GPU owned by another MPI rank
// (CUDA >= 12 cudaSetDevice initializes the device eagerly) — OOM once that
// rank fills its GPU. Contract (same as proofman): every entry point may
// leave the thread on gpu_device_; callers running their own CUDA work bind
// their device explicitly, never relying on inherited thread state.
namespace {
inline void bind_device(int dev) {
    if (dev >= 0) CUDA_CHECK(cudaSetDevice(dev));
}
}  // namespace

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

        case MOPS_READ_8_VALUE:
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
        case MOPS_BLOCK_VALUES + 0x00: case MOPS_BLOCK_VALUES + 0x10:
        case MOPS_BLOCK_VALUES + 0x20: case MOPS_BLOCK_VALUES + 0x30:
            *count_out = (op.flags >> MOPS_BLOCK_COUNT_SBITS) & 63u; return true;

        default:
            atomicOr(d_invalid_mode_flag, INVALID_MODE);
            *count_out = 0;
            return false;
    }
}

__device__ __forceinline__
void emit_one_r(uint32_t aligned, PotentialEmit* out, uint32_t step, uint64_t tag = 0) {
    const uint32_t ram_bit = is_ram_addr(aligned) ? POT_FLAG_IS_RAM : 0u;
    out[0].aligned_addr_packed = aligned | ram_bit;
    out[0].meta  = pot_meta(POT_KIND_READ, 0, 0, step);
    out[0].value = tag;   // a read's value field: the old-word slot of its MemAlign access, or 0
}

__device__ __forceinline__
void emit_one_w(uint32_t aligned, PotentialEmit* out, uint32_t step, uint32_t kind, uint64_t value) {
    const uint32_t ram_bit = is_ram_addr(aligned) ? POT_FLAG_IS_RAM : 0u;
    out[0].aligned_addr_packed = aligned | ram_bit | POT_FLAG_KIND_W;
    out[0].meta  = pot_meta(kind, 0, 0, step);
    out[0].value = value;
}

// A partial write of `width` bytes at byte `off` of the word: the read of the old word, then the
// write, whose value carries the bytes already shifted to `off`.
__device__ __forceinline__
void emit_pair_rw(uint32_t aligned, PotentialEmit* out, uint32_t step, uint32_t off, uint32_t width,
                  uint64_t value_shifted, uint32_t kind_w, uint64_t tag) {
    const uint32_t ram_bit = is_ram_addr(aligned) ? POT_FLAG_IS_RAM : 0u;
    out[0].aligned_addr_packed = aligned | ram_bit;                       // R
    out[0].meta  = pot_meta(POT_KIND_READ, 0, 0, step);
    out[0].value = tag;
    // MemAlign places the aligned write one mem step after its read (`get_write_step`); the step
    // field's low bits are the slot, so `+ 1` is that step.
    out[1].aligned_addr_packed = aligned | ram_bit | POT_FLAG_KIND_W;     // W
    out[1].meta  = pot_meta(kind_w, off, width, step + 1);
    out[1].value = value_shifted;
}

// A store of `width` bytes at `addr`: the first word takes bytes [off, 8), a second word the rest.
__device__ __forceinline__
void emit_store(uint32_t addr, uint32_t width, PotentialEmit* out, uint32_t step, uint64_t v, uint32_t kind_w,
                uint64_t tag0 = 0, uint64_t tag1 = 0) {
    const uint32_t aligned = addr & ZISK_ALIGN_MASK;
    const uint32_t off     = addr & 0x07u;
    const uint32_t first_w = (8u - off) < width ? (8u - off) : width;
    emit_pair_rw(aligned, out, step, off, first_w, v << (8u * off), kind_w, tag0);
    if (first_w < width) {
        emit_pair_rw(aligned + 8, out + 2, step, 0, width - first_w, v >> (8u * (8u - off)), kind_w, tag1);
    }
}

// The MemAlign record of an access, and the slot tags of its reads: tag = (2 * index + word) + 1.
__device__ __forceinline__
void write_align_record(AlignRecord* rec, uint32_t addr, uint32_t chunk, uint32_t chunk_bits, uint32_t step_field,
                        uint32_t kind, uint32_t width, bool wr, uint64_t value) {
    const uint64_t main_step = ((uint64_t)chunk << chunk_bits) + (step_field >> 2);
    const uint64_t mem_step  = 1ull + (main_step << 2) + (step_field & 3u);
    rec->addr  = addr;
    rec->chunk = chunk;
    rec->info  = mem_step | ((uint64_t)kind << ALIGN_KIND_SHIFT) | ((uint64_t)width << ALIGN_WIDTH_SHIFT)
               | ((uint64_t)(wr ? 1 : 0) << ALIGN_WR_SHIFT);
    rec->value = wr ? value : 0;
    rec->old[0] = 0; rec->old[1] = 0;
}

// `align_rec` is the MemAlign record of this access when it is one (`align_kind` < ALIGN_KINDS)
// and the record region holds it; `align_idx` its index, which tags its reads.
__device__ __forceinline__
void decode_emit_inline(MemOp op, PotentialEmit* out, bool skip_block, const uint64_t* rec,
                        AlignRecord* align_rec, uint32_t align_idx, uint32_t align_kind,
                        uint32_t chunk, uint32_t chunk_bits) {
    const uint32_t addr        = op.addr;
    const uint32_t aligned     = addr & ZISK_ALIGN_MASK;
    const uint8_t  mode        = op.flags & 0x3Fu;
    const uint32_t off_in_word = addr & 0x07u;
    const uint32_t step        = mops_step_field(op, false);
    // A single write without its value (header bit MOPS_NO_VALUE_BIT) is a write of unknown value.
    const bool     novalue     = ((op.flags >> (MOPS_NO_VALUE_BIT - 32)) & 1u) != 0;
    const uint32_t kind_full   = novalue ? POT_KIND_UNKNOWN : POT_KIND_WRITE;
    const uint32_t kind_part   = novalue ? POT_KIND_UNKNOWN : POT_KIND_PARTIAL;
    const uint64_t tag0 = align_rec ? 2ull * align_idx + 1 : 0;
    const uint64_t tag1 = align_rec ? 2ull * align_idx + 2 : 0;
    if (align_rec) {
        const uint32_t width = mode & 0x0Fu;   // the single-access modes carry their width in the low nibble
        write_align_record(align_rec, addr, chunk, chunk_bits, step, align_kind, width,
                           (mode & MOPS_WRITE_FLAG) != 0, op.payload);
    }
    switch (mode) {
        case MOPS_READ_1:                                         emit_one_r(aligned, out, step, tag0); break;
        case MOPS_CWRITE_1: case MOPS_WRITE_1:                    emit_store(addr, 1, out, step, op.payload, kind_part, tag0, tag1); break;
        case MOPS_READ_2:
            emit_one_r(aligned, out, step, tag0);
            if (off_in_word > 6) emit_one_r(aligned + 8, out + 1, step, tag1);
            break;
        case MOPS_WRITE_2:  emit_store(addr, 2, out, step, op.payload, kind_part, tag0, tag1); break;
        case MOPS_READ_4:
            emit_one_r(aligned, out, step, tag0);
            if (off_in_word > 4) emit_one_r(aligned + 8, out + 1, step, tag1);
            break;
        case MOPS_WRITE_4:  emit_store(addr, 4, out, step, op.payload, kind_part, tag0, tag1); break;
        case MOPS_READ_8:
            emit_one_r(aligned, out, step, tag0);
            if (off_in_word > 0) emit_one_r(aligned + 8, out + 1, step, tag1);
            break;
        case MOPS_WRITE_8:
            if (addr == aligned) {
                emit_one_w(aligned, out, step, kind_full, op.payload);
            } else {
                emit_store(addr, 8, out, step, op.payload, kind_part, tag0, tag1);
            }
            break;
        case MOPS_ALIGNED_READ  + 0x00: case MOPS_ALIGNED_READ  + 0x10:
        case MOPS_ALIGNED_READ  + 0x20: case MOPS_ALIGNED_READ  + 0x30:
            emit_one_r(addr, out, step); break;
        case MOPS_ALIGNED_WRITE + 0x00: case MOPS_ALIGNED_WRITE + 0x10:
        case MOPS_ALIGNED_WRITE + 0x20: case MOPS_ALIGNED_WRITE + 0x30:
            emit_one_w(addr, out, step, kind_full, op.payload); break;
        // A read that carries its value (the free-input word): kept as a value-carrying access so
        // the InputData fill takes the value from the record; the word never belongs to RAM.
        case MOPS_READ_8_VALUE:
            emit_one_w(addr, out, step, kind_full, op.payload); break;
        case MOPS_BLOCK_READ        + 0x00: case MOPS_BLOCK_READ        + 0x10:
        case MOPS_BLOCK_READ        + 0x20: case MOPS_BLOCK_READ        + 0x30:
        case MOPS_ALIGNED_BLOCK_READ+ 0x00: case MOPS_ALIGNED_BLOCK_READ+ 0x10:
        case MOPS_ALIGNED_BLOCK_READ+ 0x20: case MOPS_ALIGNED_BLOCK_READ+ 0x30: {
            if (skip_block) break;
            const uint32_t count = op.flags >> MOPS_BLOCK_COUNT_SBITS;
            const uint32_t bstep = mops_step_field(op, true);
            for (uint32_t i = 0; i < count; i++) emit_one_r(addr + i * 8, out + i, bstep);
            break;
        }
        case MOPS_BLOCK_WRITE        + 0x00: case MOPS_BLOCK_WRITE        + 0x10:
        case MOPS_BLOCK_WRITE        + 0x20: case MOPS_BLOCK_WRITE        + 0x30:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x00: case MOPS_ALIGNED_BLOCK_WRITE+ 0x10:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x20: case MOPS_ALIGNED_BLOCK_WRITE+ 0x30: {
            if (skip_block) break;
            const uint32_t count = op.flags >> MOPS_BLOCK_COUNT_SBITS;
            const uint32_t bstep = mops_step_field(op, true);
            for (uint32_t i = 0; i < count; i++) emit_one_w(addr + i * 8, out + i, bstep, POT_KIND_UNKNOWN, 0);
            break;
        }
        case MOPS_BLOCK_VALUES + 0x00: case MOPS_BLOCK_VALUES + 0x10:
        case MOPS_BLOCK_VALUES + 0x20: case MOPS_BLOCK_VALUES + 0x30: {
            const uint32_t count = (op.flags >> MOPS_BLOCK_COUNT_SBITS) & 63u;
            const uint32_t vstep = (op.flags >> (MOPS_VALUES_STEP_SHIFT - 32)) & MOPS_STEP_FIELD_MASK;
            const uint64_t tops  = rec[1];
            for (uint32_t i = 0; i < count; i++) {
                const uint64_t v = rec[2 + i] | (((tops >> i) & 1ull) << 63);
                emit_one_w(addr + i * 8, out + i, vstep, POT_KIND_WRITE, v);
            }
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
void tag_flag_kernel(const uint64_t* __restrict__ words, uint32_t n_words,
                     uint32_t* __restrict__ flags) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n_words) flags[i] = (uint32_t)(words[i] >> 63);
}
// `rec_index` is the exclusive scan of the flags over n_words + 1 entries.
__global__
void rec_start_kernel(const uint64_t* __restrict__ words, uint32_t n_words,
                      const uint32_t* __restrict__ rec_index,
                      uint32_t* __restrict__ rec_start, uint32_t* __restrict__ n_records) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i == 0) *n_records = rec_index[n_words];
    if (i < n_words && (words[i] >> 63)) rec_start[rec_index[i]] = i;
}
__global__
void decode_count_kernel(const uint64_t* __restrict__ words,
                         const uint32_t* __restrict__ rec_start,
                         const uint32_t* __restrict__ n_records,
                         uint32_t n_memops,
                         uint32_t* __restrict__ d_counts,
                         uint8_t* __restrict__ d_spill_status,
                         ChunkCounters* __restrict__ d_chunk_counters_entry,
                         BlockOpSpill* __restrict__ d_spill,
                         uint32_t* __restrict__ d_spill_count,
                         uint32_t* __restrict__ d_invalid_mode_flag,
                         uint8_t* __restrict__ d_align_kind,
                         uint32_t* __restrict__ d_align_flag) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    ChunkCounters my{0,0,0,0,0};
    if (i < n_memops && i >= *n_records) { d_counts[i] = 0; d_align_flag[i] = 0; }   // beyond the records: the scans see zeros
    if (i < *n_records) {
        MemOp op = load_record(words, rec_start[i]);
        if (mops_record_is_light(words[rec_start[i]])) atomicOr(d_invalid_mode_flag, LIGHT_RECORD);
        const bool ok = decode(op, &d_counts[i], my, d_invalid_mode_flag);
        // The MemAlign kind of a single access, from the counters the decode set (one at most).
        const uint32_t ak = my.full_5 ? 0u : my.full_3 ? 1u : my.full_2 ? 2u : my.read_byte ? 3u : my.write_byte ? 4u : ALIGN_KIND_NONE;
        d_align_kind[i] = (uint8_t)ak;
        d_align_flag[i] = ak < ALIGN_KINDS ? 1u : 0u;
        if (ok) {
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
                        s.payload      = op.payload;
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
void decode_emit_kernel(const uint64_t* __restrict__ words,
                        const uint32_t* __restrict__ rec_start,
                        const uint32_t* __restrict__ n_records,
                        const uint32_t* __restrict__ d_potential_offsets,
                        const uint8_t* __restrict__ d_spill_status,
                        PotentialEmit* __restrict__ d_potentials,
                        const uint8_t* __restrict__ d_align_kind,
                        const uint32_t* __restrict__ d_align_rank,
                        const AlignRun* __restrict__ align_run,
                        AlignRecord* __restrict__ d_align,
                        uint32_t chunk, uint32_t chunk_bits) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= *n_records) return;
    const uint32_t start = rec_start[i];
    MemOp op = load_record(words, start);
    PotentialEmit* out_ptr = d_potentials + d_potential_offsets[i];
    const uint32_t ak = d_align_kind[i];
    AlignRecord* arec = nullptr;
    uint32_t aidx = 0;
    if (ak < ALIGN_KINDS && align_run != nullptr && align_run->base != ALIGN_RUN_NONE) {
        aidx = align_run->base + d_align_rank[i];
        arec = d_align + aidx;
    }
    decode_emit_inline(op, out_ptr, /*skip_block=*/d_spill_status[i] != 0, words + start, arec, aidx, ak, chunk, chunk_bits);
}

// Reserves the piece's MemAlign records: `rank[n_records]` of them, at the device cursor.
__global__
void align_reserve_kernel(const uint32_t* __restrict__ d_align_rank, const uint32_t* __restrict__ n_records,
                          uint32_t* __restrict__ d_cursor, uint32_t cap, AlignRun* __restrict__ run,
                          uint32_t* __restrict__ d_overflow) {
    const uint32_t n = d_align_rank[*n_records];
    const uint32_t base = atomicAdd(d_cursor, n);
    if ((uint64_t)base + n > cap) { *run = AlignRun{ALIGN_RUN_NONE, 0}; atomicOr(d_overflow, 1u); }
    else                          { *run = AlignRun{base, n}; }
}


constexpr uint32_t BLOCKOP_EMIT_GRID = 2048;

__global__
void blockop_emit_kernel(const BlockOpSpill* __restrict__ d_spill,
                         const uint32_t* __restrict__ d_spill_count,
                         const uint32_t* __restrict__ d_potential_offsets,
                         PotentialEmit* __restrict__ d_potentials) {
    const uint32_t cap = min(*d_spill_count, MAX_BLOCKOP_SPILL_PER_CHUNK);
    for (uint32_t b = blockIdx.x; b < cap; b += gridDim.x) {
        const BlockOpSpill s = d_spill[b];
        const uint32_t base_addr   = s.aligned_base;
        const uint32_t count       = s.count;
        const uint32_t base_offset = d_potential_offsets[s.memop_idx];
        PotentialEmit* base = d_potentials + base_offset;
        const uint32_t step = (uint32_t)((s.payload >> 38) & MOPS_STEP_FIELD_MASK);
        for (uint32_t i = threadIdx.x; i < count; i += blockDim.x) {
            const uint32_t a = base_addr + i * 8u;
            if (s.kind_w) emit_one_w(a, base + i, step, POT_KIND_UNKNOWN, 0);
            else          emit_one_r(a, base + i, step);
        }
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
    d_sorted_packed[i] = (kind_w_bit << 31) | (orig_pos == 0 ? CARRY_POS : orig_pos - 1);
}

// =====================================================================
// PairSortGPU constants (those NOT in count_and_plan.cuh)
// =====================================================================



constexpr uint8_t REGION_ROM            = 0;
constexpr uint8_t REGION_INPUT          = 1;
constexpr uint8_t REGION_RAM            = 2;
constexpr const char* REGION_NAME[3]    = {"ROM", "INPUT", "RAM"};

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

// Real entries among the sorted ones (carry entries are not accesses).
__global__ void real_flags_kernel(const uint32_t* __restrict__ d_sorted_packed, uint32_t n,
                                  uint32_t* __restrict__ d_flags) {
    const uint32_t t = blockIdx.x * blockDim.x + threadIdx.x;
    if (t >= n) return;
    d_flags[t] = (d_sorted_packed[t] & 0x7FFFFFFFu) == CARRY_POS ? 0u : 1u;
}

// The piece's RAM accesses as records, in the (address, arrival) order of its sorted keys; with
// carry entries in the sort, `d_rank` gives each position its rank among the real ones.
__global__ void retain_ram_kernel(const PotentialEmit* __restrict__ d_potentials,
                                  const uint32_t* __restrict__ d_sorted_packed, uint32_t n_sorted,
                                  const uint32_t* __restrict__ d_rank,
                                  size_t base, uint32_t chunk, uint32_t chunk_size_bits, RamRecords rec,
                                  unsigned long long* __restrict__ d_nwrites) {
    const uint32_t t = blockIdx.x * blockDim.x + threadIdx.x;
    if (t >= n_sorted) return;
    const uint32_t pos = d_sorted_packed[t] & 0x7FFFFFFFu;
    if (pos == CARRY_POS) return;
    const PotentialEmit p = d_potentials[pos];
    const size_t k = base + (d_rank ? d_rank[t] : t);
    const uint32_t field = p.meta >> POT_META_STEP_SHIFT;             // (step_in_chunk << 2) | slot
    const uint64_t main_step = ((uint64_t)chunk << chunk_size_bits) + (field >> 2);
    const uint64_t mem_step  = 1ull + (main_step << 2) + (field & 3u);
    const uint64_t kind  = p.meta & 3u;
    const uint64_t off   = (p.meta >> POT_META_OFF_SHIFT) & 7u;
    const uint64_t width = (p.meta >> POT_META_WIDTH_SHIFT) & 15u;
    rec.store(k, ram_compact(emit_aligned_addr(p)),
              mem_step | (kind << RAM_META_KIND_SHIFT) | (off << RAM_META_OFF_SHIFT) | (width << RAM_META_WIDTH_SHIFT),
              p.value);
    // Write fraction, one atomic per warp.
    const unsigned writers = __ballot_sync(__activemask(), kind != POT_KIND_READ);
    if ((threadIdx.x & 31u) == (unsigned)(__ffs(__activemask()) - 1)) atomicAdd(d_nwrites, (unsigned long long)__popc(writers));
}

// Retains the piece's ROM and input accesses in arrival order, as records of the same stack as
// the RAM ones (compact address of the whole map, mem step, kind, value). `d_flag` is the emit
// bit the histogram kernel set: 1 for every access outside RAM within the memory map.
__global__ void retain_other_kernel(const PotentialEmit* __restrict__ d_potentials, uint32_t n,
                                    const uint32_t* __restrict__ d_flag, const uint32_t* __restrict__ d_rank,
                                    size_t base, uint32_t chunk, uint32_t chunk_size_bits, RamRecords rec) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n || d_flag[i] == 0) return;
    const PotentialEmit p = d_potentials[i];
    const uint32_t field = p.meta >> POT_META_STEP_SHIFT;
    const uint64_t main_step = ((uint64_t)chunk << chunk_size_bits) + (field >> 2);
    const uint64_t mem_step  = 1ull + (main_step << 2) + (field & 3u);
    const uint64_t kind  = p.meta & 3u;
    const uint64_t off   = (p.meta >> POT_META_OFF_SHIFT) & 7u;
    const uint64_t width = (p.meta >> POT_META_WIDTH_SHIFT) & 15u;
    rec.store(base + d_rank[i], compact_addr_dev(emit_aligned_addr(p)),
              mem_step | (kind << RAM_META_KIND_SHIFT) | (off << RAM_META_OFF_SHIFT) | (width << RAM_META_WIDTH_SHIFT),
              p.value);
}

__global__
void gather_ram_events_with_hist_kernel(const PotentialEmit* __restrict__ d_potentials,
                                        uint32_t n_potentials,
                                        uint64_t* __restrict__ d_ram_keys,
                                        uint32_t* __restrict__ d_ram_count,
                                        uint32_t* __restrict__ d_emit_bits,
                                        uint32_t* __restrict__ d_histogram,
                                        uint32_t* __restrict__ d_max_compact,
                                        uint32_t* __restrict__ d_invalid_flag) {
    uint32_t local_max_rom   = 0;
    uint32_t local_max_input = 0;

    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i < n_potentials) {
        PotentialEmit p = d_potentials[i];
        if (emit_is_ram(p)) {
            const uint32_t compact_ram = ram_compact(emit_aligned_addr(p));
            const uint64_t key = ((uint64_t)compact_ram << COMPACT_ADDR_SHIFT)
                               | ((uint64_t)(i + 1)     << ORIG_POS_SHIFT)
                               | (emit_kind_w(p) ? 1ull : 0ull);
            const uint32_t slot = atomicAdd(d_ram_count, 1u);
            d_ram_keys[slot] = key;
            d_emit_bits[i] = 0;
        } else if (const uint32_t raw = emit_aligned_addr(p);
                   raw >= ZISK_ROM_ADDR_BASE ? raw >= ZISK_ROM_ADDR_END
                                             : (raw < ZISK_INPUT_ADDR_BASE || raw >= ZISK_INPUT_ADDR_END)) {
            d_emit_bits[i] = 0;
            atomicOr(d_invalid_flag, INVALID_ADDRESS);
        } else {
            d_emit_bits[i] = 1;
            const uint32_t compact = compact_addr_dev(raw);
            hist_add(&d_histogram[compact], 1u, compact, d_invalid_flag);
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
                                           uint32_t* __restrict__ d_max_compact,
                                           uint32_t* __restrict__ d_invalid_flag,
                                           uint64_t* __restrict__ d_carry) {
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
            if (orig_pos == CARRY_POS) { state = kind_w; continue; }   // previous piece's state
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
        const uint32_t compact_ram = d_sorted_addr[start];
        if (n_emit > 0) {
            const uint32_t compact = compact_ram + N_ADDR_ROM + N_ADDR_INPUT;
            hist_add(&d_histogram[compact], n_emit, compact, d_invalid_flag);
            local_max_ram = compact_ram;
        }
        if (d_carry) d_carry[t] = ((uint64_t)compact_ram << COMPACT_ADDR_SHIFT) | (state ? 1ull : 0ull);
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

// Dense addr-offsets value of slot j, computed on the fly from the global
// prefix array:
//   slot 0 → 1 iff the instance is its region's first (no halo), else 0;
//   slot j → prefix[fa + j] - (base_pos - 1).
__device__ __forceinline__ uint32_t addr_offset_at(
    const uint32_t* __restrict__ prefix,
    uint32_t fa, uint32_t base_pos, uint32_t inst_first, uint32_t j)
{
    return (j == 0) ? inst_first : prefix[fa + j] - (base_pos - 1);
}

// Paged compaction of the per-instance addr offsets. One block per
// (instance, page); one thread per slot. Precondition: present_counters[ai]
// must be zero on entry.
__global__ void compact_paged_kernel(
    const uint32_t* __restrict__ prefix,
    const uint32_t* active_ids,           // [num_active] — region-local ids
    const uint32_t* inst_base_pos,        // [num_active]
    const uint32_t* active_first,         // [num_active]
    const uint32_t* active_last,          // [num_active]
    const uint32_t* page_meta_starts,     // [num_active] — prefix sum of num_pages
    const uint32_t* pages_dense_starts,   // [num_active] — prefix sum of num_pages (worst-case dense reservation, in pages)
    uint32_t        num_active,
    uint32_t*       present_counters,     // [num_active], atomic
    uint32_t*       page_starts,
    uint32_t*       page_single_value,
    uint32_t*       pages_dense)
{
    const uint32_t ai = blockIdx.y;
    if (ai >= num_active) return;
    const uint32_t page = blockIdx.x;

    const uint32_t fa = active_first[ai];
    const uint32_t la = active_last[ai];
    const uint32_t num_addrs = la - fa + 1;
    const uint32_t num_pages = (num_addrs + MEM_OFFSETS_PAGE_SIZE - 1) / MEM_OFFSETS_PAGE_SIZE;
    if (page >= num_pages) return;

    const uint32_t  base_pos     = inst_base_pos[ai];
    const uint32_t  inst_first   = (active_ids[ai] == 0) ? 1u : 0u;
    const uint32_t  page_start   = page * MEM_OFFSETS_PAGE_SIZE;
    const uint32_t  page_end_live = (page_start + MEM_OFFSETS_PAGE_SIZE < num_addrs)
                                        ? page_start + MEM_OFFSETS_PAGE_SIZE
                                        : num_addrs;

    const uint32_t single_value = addr_offset_at(prefix, fa, base_pos, inst_first, page_start);

    const uint32_t tid = threadIdx.x;
    // Each thread checks one slot (slot = page_start + tid). Skip slot 0 (== single_value by definition).
    bool local_diff = false;
    if (tid > 0) {
        const uint32_t slot = page_start + tid;
        if (slot < page_end_live &&
            addr_offset_at(prefix, fa, base_pos, inst_first, slot) != single_value) {
            local_diff = true;
        }
    }

    // Block-wide vote.
    __shared__ uint32_t s_has_diff;
    if (tid == 0) s_has_diff = 0;
    __syncthreads();
    if (local_diff) atomicOr(&s_has_diff, 1u);
    __syncthreads();
    const bool present = s_has_diff != 0;

    const uint32_t page_meta_idx = page_meta_starts[ai] + page;
    __shared__ uint32_t s_local_idx;
    if (tid == 0) {
        page_single_value[page_meta_idx] = single_value;
        if (present) {
            s_local_idx = atomicAdd(&present_counters[ai], 1u);
            page_starts[page_meta_idx] = s_local_idx;
        } else {
            page_starts[page_meta_idx] = MEM_OFFSETS_PAGE_ABSENT;
        }
    }
    __syncthreads();

    if (!present) return;

    // Each thread writes one slot into the compact output. Slots past
    // num_addrs are padded with single_value so any read past the live
    // range returns a well-defined value.
    const uint32_t dense_dst_page = pages_dense_starts[ai] + s_local_idx;
    uint32_t* out = pages_dense + (size_t)dense_dst_page * MEM_OFFSETS_PAGE_SIZE;
    const uint32_t src_slot = page_start + tid;
    out[tid] = (src_slot < num_addrs)
                   ? addr_offset_at(prefix, fa, base_pos, inst_first, src_slot)
                   : single_value;
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
    uint32_t np  = m.offsets.num_pages;
    uint32_t pc  = m.offsets.present_count;
    uint32_t ars = m.offsets.addr_range_slots;
    wr(&m.inst_id,            sizeof(uint32_t));
    wr(&m.kind,               sizeof(uint32_t));
    wr(&m.first_addr,         sizeof(uint32_t));
    wr(&m.last_addr,          sizeof(uint32_t));
    wr(&m.first_addr_chunk,   sizeof(uint32_t));
    wr(&m.first_addr_skip,    sizeof(uint32_t));
    wr(&m.last_addr_chunk,    sizeof(uint32_t));
    wr(&m.last_addr_include,  sizeof(uint32_t));
    wr(&cps, sizeof(uint32_t));
    wr(&np,  sizeof(uint32_t));
    wr(&pc,  sizeof(uint32_t));
    wr(&ars, sizeof(uint32_t));
    wr(m.count_per_chunk,           cps * sizeof(uint32_t));
    wr(m.offsets.page_starts,       np  * sizeof(uint32_t));
    wr(m.offsets.page_single_value, np  * sizeof(uint32_t));
    wr(m.offsets.pages_dense,
       static_cast<size_t>(pc) * MEM_OFFSETS_PAGE_SIZE * sizeof(uint32_t));
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

// Free on the device the resources were created on: the calling thread may be
// bound to another GPU. Skip the bind when nothing was allocated (CUDA may be
// absent, and bind_device's cudaSetDevice would be fatal there).
void CountAndPlan::free_all_bound_() {
    if (arena_ || streams_[0] || d2h_stream_ || meta_stream_) {
        bind_device(gpu_device_);
        free_all_();
    } else {
        free_all_();
    }
}

CountAndPlan::~CountAndPlan() { free_all_bound_(); }

bool CountAndPlan::setup(void* d_buf, size_t bytes,
                         uint32_t n_workers, uint32_t worker_id,
                         int gpu_id, const uint32_t instance_rows[3], bool retain_rows) {
    free_all_bound_();
    retain_rows_ = retain_rows;

    if (n_workers == 0 || worker_id >= n_workers) {
        fprintf(stderr,
                "CountAndPlan::setup ERROR: invalid n_workers=%u worker_id=%u\n",
                n_workers, worker_id);
        return false;
    }

    if (instance_rows == nullptr) {
        fprintf(stderr, "CountAndPlan::setup ERROR: instance_rows is null\n");
        return false;
    }
    for (int r = 0; r < 3; r++) {
        const uint32_t rows = instance_rows[r];
        if (rows == 0 || (rows & (rows - 1)) != 0) {
            fprintf(stderr,
                    "CountAndPlan::setup ERROR: instance_rows[%d]=%u is not a "
                    "non-zero power of two\n", r, rows);
            return false;
        }
        instance_rows_[r] = rows;
    }

    // gpu_id = proofman's my_gpu_ids[0], the GPU our buffer/kernels must use
    // (NOT always device 0 — NUMA can reorder). Captured into gpu_device_ for
    // all worker threads. A negative gpu_id is only valid on the self-allocated
    // path (d_buf == nullptr), where we allocate on the current device; with a
    // borrowed buffer it means the caller failed to resolve the device — abort
    // rather than silently allocate on the wrong GPU.
    if (gpu_id >= 0) {
        gpu_device_ = gpu_id;
    } else if (d_buf == nullptr) {
        CUDA_CHECK(cudaGetDevice(&gpu_device_));
    } else {
        fprintf(stderr,
                "CountAndPlan::setup FATAL: borrowed buffer %p but gpu_id=%d "
                "(< 0) — device unresolved\n", d_buf, gpu_id);
        std::abort();
    }
    bind_device(gpu_device_);
    // Eagerly create the primary context on our GPU so no later implicit
    // initialization can land anywhere else.
    CUDA_CHECK(cudaFree(0));

    n_workers_  = n_workers;
    worker_id_  = worker_id;
    max_active_ = (MAX_INSTANCES + n_workers - 1) / n_workers;

    size_t scan_counts_b, scan_emit_b, scan_runs_b, sort_b, rle_b, hist_scan_b;
    query_cub_sizes_(scan_counts_b, scan_emit_b, scan_runs_b, sort_b, rle_b, hist_scan_b);
    cub_temp_bytes_    = std::max({scan_counts_b, scan_emit_b, scan_runs_b, sort_b, rle_b});
    d_temp_hist_bytes_ = hist_scan_b;

    // Worst-case total page-meta entries across all active instances.
    const size_t max_total_pages =
        ((size_t)N_ADDR + MEM_OFFSETS_PAGE_SIZE - 1) / MEM_OFFSETS_PAGE_SIZE + max_active_;

    auto fixed_bytes = [&]() -> size_t {
        size_t cur = 0;
        auto take = [&](size_t b) { cur = (cur + 255) & ~(size_t)255; cur += b; };
        take(((size_t)N_ADDR + 1) * 4);              // d_histogram_, scanned in place into the prefix
        take(d_temp_hist_bytes_);
        take((size_t)max_active_ * 4);
        take((size_t)max_active_ * 4);
        take((size_t)max_active_ * 4);
        take((size_t)max_active_ * MAX_CHUNKS * 3 * 4);
        take((size_t)max_active_ * MAX_CHUNKS * 4);
        take((size_t)max_active_ * 4 * 4);
        take((size_t)max_active_ * 4);               // d_inst_base_pos_
        take(max_total_pages * 4);                   // d_page_starts_
        take(max_total_pages * 4);                   // d_page_single_
        take((size_t)max_active_ * 4);               // d_present_counters_
        take((size_t)max_active_ * 4);               // d_page_meta_starts_
        take((size_t)max_active_ * 4);               // d_pages_dense_starts_
        take(3 * 4);
        take(8);                                      // fault word
        take((size_t)MAX_CHUNKS * sizeof(ChunkCounters));
        take((size_t)MAX_CHUNKS * 4);
        take((size_t)MAX_CHUNKS * 4);
        take(((size_t)MAX_CHUNKS + 1) * 4);
        for (int s = 0; s < N_STREAMS; s++) {
            take((size_t)MAX_POT_PER_PIECE    * sizeof(PotentialEmit));
            take((size_t)MAX_POT_PER_PIECE    * 4);
            take(((size_t)MAX_POT_PER_PIECE + 1) * 4);
            take((size_t)MAX_WORDS_PER_PIECE * 8);          // d_words_
            take(((size_t)MAX_WORDS_PER_PIECE + 1) * 4);    // d_tag_flags_
            take((size_t)MAX_WORDS_PER_PIECE * 4);          // d_rec_start_
            take(4);                                         // d_n_records_
            take(((size_t)MAX_WORDS_PER_PIECE + 1) * 4);    // d_counts_
            take(((size_t)MAX_WORDS_PER_PIECE + 1) * 4);    // d_potential_offsets_
            take((size_t)MAX_BLOCKOP_SPILL_PER_CHUNK * sizeof(BlockOpSpill));
            take(4);
            take((size_t)MAX_WORDS_PER_PIECE);              // d_spill_status_
            take((size_t)MAX_SORT_PER_PIECE * 8);
            take((size_t)MAX_SORT_PER_PIECE * 8);
            take((size_t)MAX_SORT_PER_PIECE * 4);
            take((size_t)MAX_SORT_PER_PIECE * 4);
            take((size_t)MAX_SORT_PER_PIECE * 4);
            take(((size_t)MAX_SORT_PER_PIECE + 1) * 4);
            take((size_t)MAX_POT_PER_PIECE * 8);            // d_carry_
            take((size_t)MAX_SORT_PER_PIECE * 4);           // d_carry_rank_
            take(4);
            take(4);
            take(cub_temp_bytes_);
        }
        take(8);                                          // d_ram_nwrites_
        cur = (cur + 255) & ~(size_t)255;       // mirror the final round-up below
        return cur;
    }();

    if (d_buf != nullptr && bytes > 0) {
        if (bytes < fixed_bytes) {
            fprintf(stderr,
                    "CountAndPlan::setup ERROR: caller buffer is %zu bytes, need at least "
                    "%zu for fixed regions (MAX_CHUNKS=%u, MAX_POT_PER_PIECE=%u)\n",
                    bytes, fixed_bytes, MAX_CHUNKS, MAX_POT_PER_PIECE);
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

    // The prefix scan runs in place: after prepare_global_() the histogram holds the prefix.
    d_histogram_              = (uint32_t*)take(((size_t)N_ADDR + 1) * 4);
    d_prefix_                 = d_histogram_;
    d_temp_hist_              = (void*)    take(d_temp_hist_bytes_);
    d_active_ids_             = (uint32_t*)take((size_t)max_active_ * 4);
    d_active_first_           = (uint32_t*)take((size_t)max_active_ * 4);
    d_active_last_            = (uint32_t*)take((size_t)max_active_ * 4);
    d_fml_                    = (uint32_t*)take((size_t)max_active_ * MAX_CHUNKS * 3 * 4);
    d_result_nops_            = (uint32_t*)take((size_t)max_active_ * MAX_CHUNKS * 4);
    d_meta_scalars_           = (uint32_t*)take((size_t)max_active_ * 4 * 4);
    d_inst_base_pos_          = (uint32_t*)take((size_t)max_active_ * 4);
    d_page_starts_            = (uint32_t*)take(max_total_pages * 4);
    d_page_single_            = (uint32_t*)take(max_total_pages * 4);
    // d_pages_dense_ is carved from the dynamic region after the plan (process_worker_).
    d_pages_dense_            = nullptr;
    d_present_counters_       = (uint32_t*)take((size_t)max_active_ * 4);
    d_page_meta_starts_       = (uint32_t*)take((size_t)max_active_ * 4);
    d_pages_dense_starts_     = (uint32_t*)take((size_t)max_active_ * 4);
    d_max_compact_            = (uint32_t*)take(3 * 4);
    d_invalid_mode_flag_        = (uint32_t*)take(8);
    d_chunk_counters_per_chunk_ = (ChunkCounters*)take((size_t)MAX_CHUNKS * sizeof(ChunkCounters));
    d_gappy_offsets_          = (uint32_t*)take((size_t)MAX_CHUNKS * 4);
    d_chunk_lens_             = (uint32_t*)take((size_t)MAX_CHUNKS * 4);
    d_packed_chunk_offsets_   = (uint32_t*)take(((size_t)MAX_CHUNKS + 1) * 4);

    for (int s = 0; s < N_STREAMS; s++) {
        d_potentials_[s]        = (PotentialEmit*)take((size_t)MAX_POT_PER_PIECE    * sizeof(PotentialEmit));
        d_emit_bits_[s]         = (uint32_t*)     take((size_t)MAX_POT_PER_PIECE    * 4);
        d_other_rank_[s]        = (uint32_t*)     take(((size_t)MAX_POT_PER_PIECE + 1) * 4);
        d_align_kind_[s]        = (uint8_t*)      take((size_t)MAX_WORDS_PER_PIECE);
        d_align_flag_[s]        = (uint32_t*)     take(((size_t)MAX_WORDS_PER_PIECE + 1) * 4);
        d_align_rank_[s]        = (uint32_t*)     take(((size_t)MAX_WORDS_PER_PIECE + 1) * 4);
        d_final_offsets_[s]     = (uint32_t*)     take(((size_t)MAX_POT_PER_PIECE + 1) * 4);
        d_words_[s]             = (uint64_t*)     take((size_t)MAX_WORDS_PER_PIECE * 8);
        d_tag_flags_[s]         = (uint32_t*)     take(((size_t)MAX_WORDS_PER_PIECE + 1) * 4);
        d_rec_start_[s]         = (uint32_t*)     take((size_t)MAX_WORDS_PER_PIECE * 4);
        d_n_records_[s]         = (uint32_t*)     take(4);
        d_counts_[s]            = (uint32_t*)     take(((size_t)MAX_WORDS_PER_PIECE + 1) * 4);
        d_potential_offsets_[s] = (uint32_t*)     take(((size_t)MAX_WORDS_PER_PIECE + 1) * 4);
        d_spill_[s]             = (BlockOpSpill*) take((size_t)MAX_BLOCKOP_SPILL_PER_CHUNK * sizeof(BlockOpSpill));
        d_spill_count_[s]       = (uint32_t*)     take(4);
        d_spill_status_[s]      = (uint8_t*)      take((size_t)MAX_WORDS_PER_PIECE);
        d_ram_keys_[s]          = (uint64_t*)     take((size_t)MAX_SORT_PER_PIECE * 8);
        d_ram_keys_sorted_[s]   = (uint64_t*)     take((size_t)MAX_SORT_PER_PIECE * 8);
        d_sorted_addr_[s]       = (uint32_t*)     take((size_t)MAX_SORT_PER_PIECE * 4);
        d_ram_vals_sorted_[s]   = (uint32_t*)     take((size_t)MAX_SORT_PER_PIECE * 4);
        d_run_lengths_[s]       = (uint32_t*)     take((size_t)MAX_SORT_PER_PIECE * 4);
        d_run_offsets_[s]       = (uint32_t*)     take(((size_t)MAX_SORT_PER_PIECE + 1) * 4);
        d_carry_[s]             = (uint64_t*)     take((size_t)MAX_POT_PER_PIECE * 8);
        d_carry_rank_[s]        = (uint32_t*)     take((size_t)MAX_SORT_PER_PIECE * 4);
        d_num_unique_[s]        = (uint32_t*)     take(4);
        d_ram_count_[s]         = (uint32_t*)     take(4);
        d_cub_temp_[s]          = (void*)         take(cub_temp_bytes_);
    }
    d_ram_nwrites_ = (unsigned long long*)take(8);
    d_align_cursor_   = (uint32_t*)take(4);
    d_align_overflow_ = (uint32_t*)take(4);
    d_align_runs_     = (AlignRun*)take((size_t)MAX_ALIGN_RUNS * sizeof(AlignRun));

    cursor_ = (cursor_ + 255) & ~(size_t)255;
    if (cursor_ > arena_bytes_) {
        fprintf(stderr, "CountAndPlan::setup INTERNAL ERROR: cursor %zu > arena %zu\n",
                cursor_, arena_bytes_);
        return false;
    }
    // Dynamic region: the ops pool grows up from the fixed regions, the retained RAM accesses grow
    // down from the arena top; the reservations keep the two from crossing.
    top_bytes_           = arena_bytes_ & ~(size_t)255;
    // The MemAlign records take a fixed region at the very top (ZISK_MEM_ALIGN_MB, 2560): the
    // reservations are made on the device, so they cannot share the host-reserved record stack.
    {
        size_t align_mb = 2560;
        if (const char* e = std::getenv("ZISK_MEM_ALIGN_MB")) {
            const long v = std::atol(e);
            if (v >= 0) align_mb = (size_t)v;
        }
        const size_t align_bytes = (align_mb << 20) & ~(size_t)255;
        if (!retain_rows_) {
            // CPU witness: no records are kept, the whole dynamic region is the ops pool.
        } else if (align_bytes > 0 && top_bytes_ > cursor_ + align_bytes + ((size_t)1 << 30)) {
            top_bytes_ -= align_bytes;
            d_align_    = (AlignRecord*)(arena_ + top_bytes_);
            align_cap_  = align_bytes / sizeof(AlignRecord);
        } else if (align_bytes > 0) {
            fprintf(stderr, "CountAndPlan: no room for a %zu MB MemAlign region; its witness stays on the CPU\n", align_mb);
        }
    }
    ram_records_.top     = (uint32_t*)(arena_ + top_bytes_);
    d_ops_pool_          = (uint32_t*)(arena_ + cursor_);
    d_ops_pool_cap_u32_  = (top_bytes_ - cursor_) / 4;
    d_ops_pool_used_u32_ = 0;
    ram_retention_enabled_.store(retain_rows_ && top_bytes_ > cursor_, std::memory_order_relaxed);
    piece_potentials_ = MAX_POT_PER_PIECE;
    if (const char* e = std::getenv("ZISK_MOPS_PIECE_POTENTIALS")) {   // test knob: force small pieces
        const long v = std::atol(e);
        if (v > 0 && (uint32_t)v < MAX_POT_PER_PIECE) piece_potentials_ = (uint32_t)v;
    }
    fprintf(stderr, "[mops] arena %zu MB: fixed %zu MB, %zu MB shared by the ops pool and the retained RAM accesses\n",
            arena_bytes_ >> 20, cursor_ >> 20, (top_bytes_ - cursor_) >> 20);

    // Highest device priority: the count/plan pipeline is the executor's critical
    // path, and proofman's streaming-commit slots (priority 0) may run concurrently
    // on this GPU.
    int leastPrio = 0, greatestPrio = 0;
    CUDA_CHECK(cudaDeviceGetStreamPriorityRange(&leastPrio, &greatestPrio));
    for (int s = 0; s < N_STREAMS; s++)
        CUDA_CHECK(cudaStreamCreateWithPriority(&streams_[s], cudaStreamDefault, greatestPrio));
    CUDA_CHECK(cudaStreamCreateWithPriority(&d2h_stream_, cudaStreamDefault, greatestPrio));
    CUDA_CHECK(cudaStreamCreateWithPriority(&meta_stream_, cudaStreamDefault, greatestPrio));
    CUDA_CHECK(cudaEventCreate(&e_after_preproc_));
    CUDA_CHECK(cudaEventCreate(&e_after_prepare_));
    CUDA_CHECK(cudaEventCreate(&e_metas_ready_));

    CUDA_CHECK(cudaMallocHost(&h_n_emits_all_, (size_t)MAX_CHUNKS * 4));
    CUDA_CHECK(cudaMallocHost(&h_result_nops_, (size_t)max_active_ * MAX_CHUNKS * 4));
    CUDA_CHECK(cudaMallocHost(&h_meta_scalars_, (size_t)max_active_ * 4 * 4));
    CUDA_CHECK(cudaMallocHost(&h_chunk_counters_per_chunk_,
        (size_t)MAX_CHUNKS * sizeof(ChunkCounters)));
    // Pinned destinations for the compacted paged-offsets output. Each
    // instance contributes ceil(addr_range_slots / MEM_OFFSETS_PAGE_SIZE)
    // entries to h_page_starts_buf_ / h_page_single_buf_, and at most
    // addr_range_slots entries to h_pages_dense_buf_ (worst case: every
    // page present). Seeded at 1 GiB; grown on demand in process_worker_().
    const size_t initial_offsets_bytes = 1ull << 30;
    h_page_meta_buf_size_   = initial_offsets_bytes / MEM_OFFSETS_PAGE_SIZE + 4096;
    h_pages_dense_buf_size_ = initial_offsets_bytes;
    CUDA_CHECK(cudaMallocHost(&h_page_starts_buf_,  h_page_meta_buf_size_));
    CUDA_CHECK(cudaMallocHost(&h_page_single_buf_,   h_page_meta_buf_size_));
    CUDA_CHECK(cudaMallocHost(&h_pages_dense_buf_,  h_pages_dense_buf_size_));
    CUDA_CHECK(cudaMallocHost(&h_present_counters_, (size_t)max_active_ * sizeof(uint32_t)));
    for (int s = 0; s < N_STREAMS; s++) {
        CUDA_CHECK(cudaMallocHost(&h_n_emits_[s], sizeof(uint32_t)));
        CUDA_CHECK(cudaMallocHost(&h_n_carry_[s], sizeof(uint32_t)));
    }

    pool_enabled_ = (ZISK_MOPS_POOL != 0);
    if (pool_enabled_) {
        fprintf(stderr, "[mops-pool] enabled (%d stream workers, device %d)\n",
                N_STREAMS, gpu_device_);
    }

    // The pinned buffers the RAM and MemAlign fills pack rows into: pinning gigabytes takes
    // hundreds of milliseconds, so the default capacities (ZISK_MEM_GPU_ROWS_MB 4096,
    // ZISK_MEM_ALIGN_ROWS_MB 2560) are allocated now, off the block path; a block that needs more
    // grows them.
    {
        auto mb_of = [](const char* name, size_t dflt) {
            if (const char* e = std::getenv(name)) {
                const long v = std::atol(e);
                if (v >= 0) return (size_t)v;
            }
            return dflt;
        };
        const size_t rows_mb = mb_of("ZISK_MEM_GPU_ROWS_MB", 4096);
        const size_t align_mb = mb_of("ZISK_MEM_ALIGN_ROWS_MB", 2560);
        if (rows_mb > 0 || align_mb > 0) {
            const int dev = gpu_device_;
            h_ram_rows_prealloc_ = std::thread([this, dev, rows_mb, align_mb] {
                cudaSetDevice(dev);
                void* p = nullptr;
                if (rows_mb > 0 && cudaMallocHost(&p, rows_mb << 20) == cudaSuccess) {
                    h_ram_rows_ = (uint64_t*)p;
                    h_ram_rows_cap_ = (rows_mb << 20) / 8;
                }
                p = nullptr;
                if (align_mb > 0 && cudaMallocHost(&p, align_mb << 20) == cudaSuccess) {
                    h_align_rows_ = (uint64_t*)p;
                    h_align_rows_cap_ = (align_mb << 20) / 8;
                }
            });
        }
    }

    reset();
    return true;
}

bool CountAndPlan::add_chunk(const uint64_t* words, uint32_t n) {
    if (n_chunks_ >= MAX_CHUNKS) {
        fprintf(stderr,
                "CountAndPlan::add_chunk ERROR: MAX_CHUNKS=%u exceeded\n", MAX_CHUNKS);
        std::abort();
    }
    const uint32_t c = n_chunks_++;
    if (pool_enabled_) {
        const int s = c % N_STREAMS;
        {
            std::lock_guard<std::mutex> lk(pool_mtx_[s]);
            pool_q_[s].push_back(ChunkJob{words, n, c});
        }
        pool_cv_[s].notify_one();
        return true;
    }
    bind_device(gpu_device_);  // pool workers bind once at thread start instead
    return add_chunk_core_(words, n, c);
}

// The chunk rewritten in the light form, one word per record: steps zeroed, writes without
// values, block records without payload, value blocks as block writes. Test support
// (ZISK_MOPS_TEST_LIGHT_STREAM) for the form the light emulator emits.
static std::vector<uint64_t> strip_values(const uint64_t* words, uint32_t n) {
    std::vector<uint64_t> out;
    out.reserve(n);
    const uint64_t step_bits = 0xFFFFFull << 38;
    for (uint32_t k = 0; k < n;) {
        const uint64_t hdr = words[k];
        const uint32_t len = mops_record_len(hdr);
        const uint32_t mode = (uint32_t)(hdr >> 32) & 0x3Fu, low = mode & 0x0Fu;
        const bool single = (low == 1 || low == 2 || low == 4 || low == 8);
        if (low == MOPS_BLOCK_VALUES) {
            const uint64_t count = (hdr >> 36) & 63u;
            out.push_back((hdr & 0xFFFFFFFFull) | ((uint64_t)MOPS_ALIGNED_BLOCK_WRITE << 32) | (count << 36)
                          | (1ull << MOPS_NO_PAYLOAD_BIT) | (1ull << 63));
        } else if (low == MOPS_ALIGNED_READ || (mode != MOPS_READ_8_VALUE && single && (mode & MOPS_WRITE_FLAG) == 0)) {
            out.push_back(hdr & ~step_bits);                                                      // read
        } else if (single || low == MOPS_ALIGNED_WRITE) {
            out.push_back(((hdr | (1ull << MOPS_NO_VALUE_BIT)) & ~(1ull << 62)) & ~step_bits);    // write
        } else {
            out.push_back(hdr | (1ull << MOPS_NO_PAYLOAD_BIT));                                  // block read or write
        }
        k += len;
    }
    return out;
}

bool CountAndPlan::add_chunk_core_(const uint64_t* words, uint32_t n, uint32_t c) {
    const int s = c % N_STREAMS;
    static const bool light_stream = std::getenv("ZISK_MOPS_TEST_LIGHT_STREAM") != nullptr;
    std::vector<uint64_t> light;
    if (light_stream) {
        light = strip_values(words, n);
        words = light.data();
        n = (uint32_t)light.size();
    }

    // Host walk: potentials and RAM accesses per record, and the cuts into pieces of whole records.
    struct Piece { uint32_t w0, nw, pot, ram; };
    std::vector<Piece> pieces;
    Piece cur{0, 0, 0, 0};
    size_t   pot_total = 0;
    uint32_t ram_total = 0;
    for (uint32_t k = 0; k < n;) {
        const uint32_t len = mops_record_len(words[k]);
        const MemOp op = load_record(words, k);
        const uint32_t addr    = op.addr;
        const uint32_t aligned = addr & ZISK_ALIGN_MASK;
        const uint8_t  mode    = op.flags & 0x3Fu;
        const uint32_t off     = addr & 0x07u;
        uint32_t dp = 0, dr = 0;
        auto add_pot = [&](uint32_t a, uint32_t count) { dp += count; if (is_ram_addr(a)) dr += count; };
        switch (mode) {
            case MOPS_READ_1:                                add_pot(aligned, 1); break;
            case MOPS_CWRITE_1: case MOPS_WRITE_1:           add_pot(aligned, 2); break;
            case MOPS_READ_2:   add_pot(aligned, 1); if (off > 6) add_pot(aligned + 8, 1); break;
            case MOPS_WRITE_2:  add_pot(aligned, 2); if (off > 6) add_pot(aligned + 8, 2); break;
            case MOPS_READ_4:   add_pot(aligned, 1); if (off > 4) add_pot(aligned + 8, 1); break;
            case MOPS_WRITE_4:  add_pot(aligned, 2); if (off > 4) add_pot(aligned + 8, 2); break;
            case MOPS_READ_8:   add_pot(aligned, 1); if (off > 0) add_pot(aligned + 8, 1); break;
            case MOPS_READ_8_VALUE: add_pot(addr, 1); break;
            case MOPS_WRITE_8:  if (addr == aligned) add_pot(aligned, 1);
                                else { add_pot(aligned, 2); add_pot(aligned + 8, 2); } break;
            case MOPS_ALIGNED_READ  + 0x00: case MOPS_ALIGNED_READ  + 0x10:
            case MOPS_ALIGNED_READ  + 0x20: case MOPS_ALIGNED_READ  + 0x30:
            case MOPS_ALIGNED_WRITE + 0x00: case MOPS_ALIGNED_WRITE + 0x10:
            case MOPS_ALIGNED_WRITE + 0x20: case MOPS_ALIGNED_WRITE + 0x30:
                add_pot(addr, 1); break;
            case MOPS_BLOCK_VALUES + 0x00: case MOPS_BLOCK_VALUES + 0x10:
            case MOPS_BLOCK_VALUES + 0x20: case MOPS_BLOCK_VALUES + 0x30:
                add_pot(addr, (op.flags >> MOPS_BLOCK_COUNT_SBITS) & 63u); break;
            default: add_pot(addr, op.flags >> MOPS_BLOCK_COUNT_SBITS); break;
        }
        if (cur.nw > 0 && (cur.pot + dp > piece_potentials_ || cur.nw + len > MAX_WORDS_PER_PIECE)) {
            pieces.push_back(cur);
            cur = Piece{k, 0, 0, 0};
        }
        cur.nw += len; cur.pot += dp; cur.ram += dr;
        pot_total += dp; ram_total += dr;
        k += len;
    }
    if (cur.nw > 0) pieces.push_back(cur);
    {
        uint32_t np = (uint32_t)pieces.size(), seen = max_pieces_.load(std::memory_order_relaxed);
        while (np > seen && !max_pieces_.compare_exchange_weak(seen, np, std::memory_order_relaxed)) {}
    }

    // Reserve this chunk's compacted-output region in the device ops pool. Each stack bumps its
    // own cursor before reading the other's, so two concurrent reservations cannot both miss the
    // crossing. Meeting the retained accesses drops the device witness first; only a pool that
    // does not fit the region on its own is an error.
    const size_t base = pool_cursor_u32_.fetch_add(pot_total, std::memory_order_seq_cst);
    if (pool_end_bytes(base + pot_total) > ram_low_edge_bytes(ram_cursor_.load(std::memory_order_seq_cst))
        && ram_retention_enabled_.exchange(false)) {
        fprintf(stderr, "CountAndPlan: the ops pool meets the retained RAM accesses at chunk %u; "
                        "no device RAM witness for this block\n", c);
    }
    if (pool_end_bytes(base + pot_total) > top_bytes_) {
        fprintf(stderr,
                "CountAndPlan::add_chunk ERROR: ops pool exhausted at chunk %u "
                "(base %zu + need %zu > capacity %zu u32 entries). "
                "Increase the buffer size passed to setup().\n",
                c, base, pot_total, d_ops_pool_cap_u32_);
        add_error_.store(true, std::memory_order_relaxed);
        return false;
    }
    out_offsets_[c]            = base;
    n_potentials_per_chunk_[c] = (uint32_t)pot_total;
    n_words_per_chunk_[c]      = n;
    n_ram_per_chunk_[c]        = ram_total;

    if (n == 0) {
        *(h_n_emits_[s]) = 0;
        h_n_emits_all_[c] = 0;
        return true;
    }
    uint32_t* d_chunk_out = d_ops_pool_ + base;
    if (pieces.size() == 1) {
        const bool ok = add_piece_(words, n, c, s, (uint32_t)pot_total, ram_total, d_chunk_out, 0, false, &h_n_emits_all_[c]);
        if (light_stream) CUDA_CHECK(cudaStreamSynchronize(streams_[s]));   // the rewritten words die here
        return ok;
    }

    // Pieces run in order on the chunk's stream; each hands its emit count and the pairing state
    // of the addresses it touched to the next.
    uint32_t emits = 0, n_carry = 0;
    for (size_t p = 0; p < pieces.size(); ++p) {
        const Piece& pc = pieces[p];
        const bool last = p + 1 == pieces.size();
        if (!add_piece_(words + pc.w0, pc.nw, c, s, pc.pot, pc.ram, d_chunk_out + emits, n_carry, !last, h_n_emits_[s]))
            return false;
        CUDA_CHECK(cudaStreamSynchronize(streams_[s]));
        emits += *h_n_emits_[s];
        n_carry = (!last && pc.ram + n_carry > 0) ? *h_n_carry_[s] : 0;
    }
    h_n_emits_all_[c] = emits;
    return true;
}

// One piece of a chunk through the device pipeline: decode, expand, count, pair, retain, compact.
bool CountAndPlan::add_piece_(const uint64_t* words, uint32_t n, uint32_t c, int s, uint32_t pot, uint32_t ram,
                              uint32_t* d_out, uint32_t n_carry, bool carry_out, uint32_t* h_emits) {
    cudaStream_t st = streams_[s];
    constexpr int BLOCK = 256;
    const int g_memops = (n + BLOCK - 1) / BLOCK;
    const int g_pot    = (pot + BLOCK - 1) / BLOCK;
    const uint32_t n_sort = ram + n_carry;
    const int g_sort   = n_sort == 0 ? 0 : (int)((n_sort + BLOCK - 1) / BLOCK);

    CUDA_CHECK(cudaMemcpyAsync(d_words_[s], words, 8ull * n, cudaMemcpyHostToDevice, st));
    CUDA_CHECK(cudaMemsetAsync(d_ram_count_[s],    0, 4, st));
    CUDA_CHECK(cudaMemsetAsync(d_spill_count_[s],  0, 4, st));
    CUDA_CHECK(cudaMemsetAsync(d_spill_status_[s], 0, n, st));
    // Record boundaries: header words are tagged; their exclusive scan indexes the records.
    tag_flag_kernel<<<g_memops, BLOCK, 0, st>>>(d_words_[s], n, d_tag_flags_[s]);
    CUDA_CHECK_LAUNCH();
    {
        size_t bytes = cub_temp_bytes_;
        CUDA_CHECK(cub::DeviceScan::ExclusiveSum(d_cub_temp_[s], bytes,
            d_tag_flags_[s], d_tag_flags_[s], n + 1, st));
    }
    rec_start_kernel<<<g_memops, BLOCK, 0, st>>>(d_words_[s], n, d_tag_flags_[s], d_rec_start_[s], d_n_records_[s]);
    CUDA_CHECK_LAUNCH();

    decode_count_kernel<<<g_memops, BLOCK, 0, st>>>(
        d_words_[s], d_rec_start_[s], d_n_records_[s], n, d_counts_[s], d_spill_status_[s],
        &d_chunk_counters_per_chunk_[c],
        d_spill_[s], d_spill_count_[s],
        d_invalid_mode_flag_, d_align_kind_[s], d_align_flag_[s]);
    CUDA_CHECK_LAUNCH();

    {
        size_t bytes = cub_temp_bytes_;
        CUDA_CHECK(cub::DeviceScan::ExclusiveSum(d_cub_temp_[s], bytes,
            d_counts_[s], d_potential_offsets_[s], n + 1, st));
    }

    // The piece's MemAlign accesses: ranked, reserved at the device cursor, recorded by the emit.
    const AlignRun* align_run = nullptr;
    if (align_enabled_.load(std::memory_order_relaxed)) {
        const uint32_t slot = align_slot_.fetch_add(1, std::memory_order_relaxed);
        if (slot < MAX_ALIGN_RUNS) {
            size_t bytes = cub_temp_bytes_;
            CUDA_CHECK(cub::DeviceScan::ExclusiveSum(d_cub_temp_[s], bytes,
                d_align_flag_[s], d_align_rank_[s], n + 1, st));
            align_reserve_kernel<<<1, 1, 0, st>>>(d_align_rank_[s], d_n_records_[s], d_align_cursor_,
                                                  (uint32_t)align_cap_, d_align_runs_ + slot, d_align_overflow_);
            CUDA_CHECK_LAUNCH();
            align_run = d_align_runs_ + slot;
            std::lock_guard<std::mutex> lk(ram_runs_mtx_);
            align_runs_.push_back(AlignPieceRun{c, slot});
        } else if (align_enabled_.exchange(false)) {
            fprintf(stderr, "CountAndPlan: more than %u pieces; no device MemAlign witness for this block\n", MAX_ALIGN_RUNS);
        }
    }

    decode_emit_kernel<<<g_memops, BLOCK, 0, st>>>(
        d_words_[s], d_rec_start_[s], d_n_records_[s], d_potential_offsets_[s], d_spill_status_[s], d_potentials_[s],
        d_align_kind_[s], d_align_rank_[s], align_run, d_align_, c, chunk_size_bits_);
    CUDA_CHECK_LAUNCH();

    blockop_emit_kernel<<<BLOCKOP_EMIT_GRID, 256, 0, st>>>(
        d_spill_[s], d_spill_count_[s], d_potential_offsets_[s], d_potentials_[s]);
    CUDA_CHECK_LAUNCH();

    gather_ram_events_with_hist_kernel<<<g_pot, BLOCK, 0, st>>>(
        d_potentials_[s], pot,
        d_ram_keys_[s], d_ram_count_[s], d_emit_bits_[s],
        d_histogram_, d_max_compact_, d_invalid_mode_flag_);
    CUDA_CHECK_LAUNCH();

    // Retain the piece's ROM and input accesses, in arrival order.
    if (pot > ram && ram_retention_enabled_.load(std::memory_order_relaxed)) {
        const uint32_t n_other = pot - ram;
        const size_t obase = ram_cursor_.fetch_add(n_other, std::memory_order_seq_cst);
        if (ram_low_edge_bytes(obase + n_other) < pool_end_bytes(pool_cursor_u32_.load(std::memory_order_seq_cst))) {
            if (ram_retention_enabled_.exchange(false)) {
                fprintf(stderr, "CountAndPlan: the retained accesses meet the ops pool at chunk %u "
                                "(%zu accesses so far); no device memory witness for this block\n", c, obase + n_other);
            }
        } else {
            {
                std::lock_guard<std::mutex> lk(ram_runs_mtx_);
                other_runs_.push_back(RamRun{(uint32_t)obase, n_other, c});
            }
            size_t bytes_rank = cub_temp_bytes_;
            CUDA_CHECK(cub::DeviceScan::ExclusiveSum(d_cub_temp_[s], bytes_rank,
                d_emit_bits_[s], d_other_rank_[s], pot, st));
            retain_other_kernel<<<g_pot, BLOCK, 0, st>>>(d_potentials_[s], pot, d_emit_bits_[s], d_other_rank_[s],
                obase, c, chunk_size_bits_, ram_records_);
            CUDA_CHECK_LAUNCH();
        }
    }

    if (n_sort > 0) {
        // The previous piece's pairing state enters the sort as one entry per address.
        if (n_carry > 0)
            CUDA_CHECK(cudaMemcpyAsync(d_ram_keys_[s] + ram, d_carry_[s], 8ull * n_carry, cudaMemcpyDeviceToDevice, st));
        size_t bytes_sort = cub_temp_bytes_;
        CUDA_CHECK(cub::DeviceRadixSort::SortKeys(d_cub_temp_[s], bytes_sort,
            d_ram_keys_[s], d_ram_keys_sorted_[s], n_sort, 0, RAM_KEY_END_BIT, st));

        extract_sorted_addr_kernel<<<g_sort, BLOCK, 0, st>>>(
            d_ram_keys_sorted_[s], n_sort, d_sorted_addr_[s]);
        CUDA_CHECK_LAUNCH();

        extract_sorted_packed_kernel<<<g_sort, BLOCK, 0, st>>>(
            d_ram_keys_sorted_[s], n_sort, d_ram_vals_sorted_[s]);
        CUDA_CHECK_LAUNCH();

        // Retain the piece's RAM accesses for the witness, in the sorted order.
        if (ram > 0 && ram_retention_enabled_.load(std::memory_order_relaxed)) {
            const size_t rbase = ram_cursor_.fetch_add(ram, std::memory_order_seq_cst);
            if (ram_low_edge_bytes(rbase + ram) < pool_end_bytes(pool_cursor_u32_.load(std::memory_order_seq_cst))) {
                if (ram_retention_enabled_.exchange(false)) {
                    fprintf(stderr, "CountAndPlan: the retained RAM accesses meet the ops pool at chunk %u "
                                    "(%zu accesses so far); no device RAM witness for this block\n", c, rbase + ram);
                }
            } else {
                {
                    std::lock_guard<std::mutex> lk(ram_runs_mtx_);
                    ram_runs_.push_back(RamRun{(uint32_t)rbase, ram, c});
                }
                const uint32_t* d_rank = nullptr;
                if (n_carry > 0) {
                    real_flags_kernel<<<g_sort, BLOCK, 0, st>>>(d_ram_vals_sorted_[s], n_sort, d_carry_rank_[s]);
                    CUDA_CHECK_LAUNCH();
                    size_t bytes_rank = cub_temp_bytes_;
                    CUDA_CHECK(cub::DeviceScan::ExclusiveSum(d_cub_temp_[s], bytes_rank,
                        d_carry_rank_[s], d_carry_rank_[s], n_sort, st));
                    d_rank = d_carry_rank_[s];
                }
                retain_ram_kernel<<<g_sort, BLOCK, 0, st>>>(d_potentials_[s], d_ram_vals_sorted_[s], n_sort, d_rank,
                    rbase, c, chunk_size_bits_, ram_records_, d_ram_nwrites_);
                CUDA_CHECK_LAUNCH();
            }
        }

        size_t bytes_rle = cub_temp_bytes_;
        CUDA_CHECK(cub::DeviceRunLengthEncode::Encode(d_cub_temp_[s], bytes_rle,
            d_sorted_addr_[s], thrust::discard_iterator<>{},
            d_run_lengths_[s], d_num_unique_[s], n_sort, st));

        size_t bytes_sr = cub_temp_bytes_;
        CUDA_CHECK(cub::DeviceScan::ExclusiveSum(d_cub_temp_[s], bytes_sr,
            d_run_lengths_[s], d_run_offsets_[s], n_sort + 1, st));

        state_machine_by_run_with_hist_kernel<<<g_sort, BLOCK, 0, st>>>(
            d_run_offsets_[s], d_num_unique_[s], d_ram_vals_sorted_[s],
            d_sorted_addr_[s], d_emit_bits_[s], d_histogram_, d_max_compact_, d_invalid_mode_flag_,
            carry_out ? d_carry_[s] : nullptr);
        CUDA_CHECK_LAUNCH();
        if (carry_out)
            CUDA_CHECK(cudaMemcpyAsync(h_n_carry_[s], d_num_unique_[s], 4, cudaMemcpyDeviceToHost, st));
    }

    {
        size_t bytes = cub_temp_bytes_;
        CUDA_CHECK(cub::DeviceScan::ExclusiveSum(d_cub_temp_[s], bytes,
            d_emit_bits_[s], d_final_offsets_[s], pot + 1, st));
    }

    CUDA_CHECK(cudaMemcpyAsync(h_emits, d_final_offsets_[s] + pot, 4, cudaMemcpyDeviceToHost, st));

    compact_kernel_with_shift<<<g_pot, BLOCK, 0, st>>>(
        d_potentials_[s], d_emit_bits_[s], d_final_offsets_[s],
        pot, d_out);
    CUDA_CHECK_LAUNCH();

    return true;
}


void CountAndPlan::pool_thread_loop_(int s) {
    CUDA_CHECK(cudaSetDevice(gpu_device_));
    for (;;) {
        ChunkJob job;
        {
            std::unique_lock<std::mutex> lk(pool_mtx_[s]);
            pool_cv_[s].wait(lk, [&] { return pool_should_stop_ || !pool_q_[s].empty(); });
            if (pool_q_[s].empty()) return;
            job = pool_q_[s].front();
            pool_q_[s].pop_front();
        }
        add_chunk_core_(job.words, job.n, job.c);
    }
}

void CountAndPlan::pool_start_() {
    pool_should_stop_ = false;
    for (int s = 0; s < N_STREAMS; s++)
        pool_threads_[s] = std::thread(&CountAndPlan::pool_thread_loop_, this, s);
}


void CountAndPlan::pool_stop_() {
    for (int s = 0; s < N_STREAMS; s++) {
        std::lock_guard<std::mutex> lk(pool_mtx_[s]);
        pool_should_stop_ = true;
    }
    for (int s = 0; s < N_STREAMS; s++) pool_cv_[s].notify_all();
    for (int s = 0; s < N_STREAMS; s++)
        if (pool_threads_[s].joinable()) pool_threads_[s].join();
}

bool CountAndPlan::run(InstanceMeta** metas_out, uint32_t& n_metas) {
    bind_device(gpu_device_);

    if (pool_enabled_) pool_stop_();
    if (n_chunks_ == 0) {
        fprintf(stderr, "CountAndPlan::run ERROR: no chunks added\n");
        return false;
    }

    if (add_error_.load(std::memory_order_relaxed)) {
        fprintf(stderr, "CountAndPlan::run FATAL: add_chunk reported an error "
                        "(potentials or ops-pool capacity exceeded) — aborting\n");
        std::exit(1);
    }

    if (!preprocessed_) {
        for (int s = 0; s < N_STREAMS; s++)
            CUDA_CHECK(cudaStreamSynchronize(streams_[s]));
        CUDA_CHECK(cudaEventRecord(e_after_preproc_, 0));

        // Pull per-chunk mem-align counters back to host (only the touched
        // range; max MAX_CHUNKS * sizeof(ChunkCounters), well under 1 MB).
        // Streams are already synced above, so a plain synchronous memcpy is
        // fine.
        if (h_chunk_counters_per_chunk_ && d_chunk_counters_per_chunk_) {
            CUDA_CHECK(cudaMemcpy(
                h_chunk_counters_per_chunk_,
                d_chunk_counters_per_chunk_,
                (size_t)n_chunks_ * sizeof(ChunkCounters),
                cudaMemcpyDeviceToHost));
        }

        // The prefix, the pool offsets and the instance arithmetic are u32: a block with 2^32 rows
        // or more cannot be planned, and the counters below would wrap silently.
        packed_chunk_offsets_h_.assign(n_chunks_ + 1, 0);
        uint64_t total_rows = 0;
        for (uint32_t c = 0; c < n_chunks_; c++) {
            total_rows += h_n_emits_all_[c];
            if (total_rows > 0xFFFFFFFFull) {
                fprintf(stderr, "CountAndPlan::run FATAL: the block has more than 2^32 - 1 memory rows "
                                "(reached at chunk %u of %u)\n", c, n_chunks_);
                std::exit(1);
            }
            packed_chunk_offsets_h_[c + 1] = (uint32_t)total_rows;
        }
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

        uint32_t h_fault[2] = {0, 0};
        CUDA_CHECK(cudaMemcpy(h_fault, d_invalid_mode_flag_, 8, cudaMemcpyDeviceToHost));
        if (h_fault[0] & LIGHT_RECORD) {
            // The stream is in the light form (no values, no steps): the plan stands, the device
            // witness does not.
            if (ram_retention_enabled_.exchange(false))
                fprintf(stderr, "CountAndPlan: the stream is in the light form (no values or steps); no device "
                                "RAM witness for this block\n");
            h_fault[0] &= ~LIGHT_RECORD;
        }
        if (h_fault[0] & COUNTER_OVERFLOW) {
            fprintf(stderr, "CountAndPlan::run FATAL: more than 2^32 - 1 rows at address 0x%08x\n",
                    expand_addr(h_fault[1]));
            std::exit(1);
        }
        if (h_fault[0] != 0) {
            fprintf(stderr, "CountAndPlan::run FATAL: %s in the memory-ops stream\n",
                    (h_fault[0] & INVALID_ADDRESS) ? "access outside the memory map" : "unrecognised record mode");
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
    bind_device(gpu_device_);

    n_chunks_              = 0;
    num_ops_               = 0;
    d_ops_pool_used_u32_   = 0;
    out_offsets_.assign(MAX_CHUNKS, 0);
    n_potentials_per_chunk_.assign(MAX_CHUNKS, 0);
    n_ram_per_chunk_.assign(MAX_CHUNKS, 0);
    n_words_per_chunk_.assign(MAX_CHUNKS, 0);
    { std::lock_guard<std::mutex> lk(ram_runs_mtx_); ram_runs_.clear(); }
    max_pieces_.store(0, std::memory_order_relaxed);
    packed_chunk_offsets_h_.clear();
    pool_cursor_u32_.store(0, std::memory_order_relaxed);
    add_error_.store(false, std::memory_order_relaxed);
    metas_.assign(MAX_INSTANCES, InstanceMeta{});
    metas_ready_recorded_   = false;
    preprocessed_           = false;
    prepared_               = false;
    ram_cursor_.store(0, std::memory_order_relaxed);
    ram_retention_enabled_.store(retain_rows_ && top_bytes_ > cursor_, std::memory_order_relaxed);
    ram_prepared_           = false;
    ram_tables_ready_       = false;
    ram_results_.clear();
    ram_n_lanes_            = 0;
    { std::lock_guard<std::mutex> lk(ram_runs_mtx_); other_runs_.clear(); }
    rom_prepared_           = false;
    rom_results_.clear();
    input_prepared_         = false;
    input_results_.clear();
    align_prepared_         = false;
    align_results_.clear();
    slot_prepared_          = false;
    resolve_all_            = false;
    slot_scratch_           = nullptr;
    slot_quiesce();
    staged_.clear();
    stage_low_              = nullptr;
    stage_try_              = false;
    input_scratch_          = nullptr;
    d_image_                = nullptr;
    image_words_            = 0;
    slot_align_plans_.clear();
    slot_align_entries_.clear();
    align_total_            = 0;
    d_align_order_          = nullptr;
    align_runs_.clear();
    align_slot_.store(0, std::memory_order_relaxed);
    align_enabled_.store(retain_rows_ && d_align_ != nullptr && top_bytes_ > cursor_, std::memory_order_relaxed);
    if (d_align_cursor_)             CUDA_CHECK(cudaMemset(d_align_cursor_, 0, 4));
    if (d_align_overflow_)           CUDA_CHECK(cudaMemset(d_align_overflow_, 0, 4));
    other_total_            = 0;
    d_other_idx_            = nullptr;
    d_other_addr_           = nullptr;

    if (d_histogram_)                CUDA_CHECK(cudaMemset(d_histogram_, 0, ((size_t)N_ADDR + 1) * 4));
    if (d_max_compact_)              CUDA_CHECK(cudaMemset(d_max_compact_, 0, 3 * 4));
    if (d_ram_nwrites_)              CUDA_CHECK(cudaMemset(d_ram_nwrites_, 0, 8));
    if (d_invalid_mode_flag_)        CUDA_CHECK(cudaMemset(d_invalid_mode_flag_, 0, 8));
    if (d_chunk_counters_per_chunk_) CUDA_CHECK(cudaMemset(d_chunk_counters_per_chunk_, 0, (size_t)MAX_CHUNKS * sizeof(ChunkCounters)));

    if (pool_enabled_) {
        pool_stop_();
        pool_start_();
    }
}

bool CountAndPlan::register_input_pinned(void* ptr, size_t bytes) {
    if (!ptr || bytes == 0) return false;
    bind_device(gpu_device_);
    cudaError_t e = cudaHostRegister(ptr, bytes, cudaHostRegisterDefault);
    if (e != cudaSuccess) {
        cudaGetLastError();
        fprintf(stderr,
                "[mops-pinned] cudaHostRegister(%p, %zu, Default) failed: %s "
                "— H2D will use the synchronous pageable fallback\n",
                ptr, bytes, cudaGetErrorString(e));
        return false;
    }
    return true;
}

void CountAndPlan::unregister_input_pinned(void* ptr) {
    if (!ptr) return;
    bind_device(gpu_device_);
    cudaHostUnregister(ptr);
    cudaGetLastError();  // ignore "not registered"
}

void CountAndPlan::free_pinned_() {
    join_rows_prealloc_();
    if (h_ram_rows_)                 { cudaFreeHost(h_ram_rows_);                 h_ram_rows_                 = nullptr; h_ram_rows_cap_ = 0; }
    if (h_rom_rows_)                 { cudaFreeHost(h_rom_rows_);                 h_rom_rows_                 = nullptr; h_rom_rows_cap_ = 0; }
    if (h_input_rows_)               { cudaFreeHost(h_input_rows_);               h_input_rows_               = nullptr; h_input_rows_cap_ = 0; }
    if (h_align_rows_)               { cudaFreeHost(h_align_rows_);               h_align_rows_               = nullptr; h_align_rows_cap_ = 0; }
    if (h_n_emits_all_)              { cudaFreeHost(h_n_emits_all_);              h_n_emits_all_              = nullptr; }
    if (h_page_starts_buf_)          { cudaFreeHost(h_page_starts_buf_);          h_page_starts_buf_          = nullptr; }
    if (h_page_single_buf_)           { cudaFreeHost(h_page_single_buf_);           h_page_single_buf_           = nullptr; }
    if (h_pages_dense_buf_)          { cudaFreeHost(h_pages_dense_buf_);          h_pages_dense_buf_          = nullptr; }
    if (h_present_counters_)         { cudaFreeHost(h_present_counters_);         h_present_counters_         = nullptr; }
    if (h_result_nops_)              { cudaFreeHost(h_result_nops_);              h_result_nops_              = nullptr; }
    if (h_meta_scalars_)             { cudaFreeHost(h_meta_scalars_);             h_meta_scalars_             = nullptr; }
    if (h_chunk_counters_per_chunk_) { cudaFreeHost(h_chunk_counters_per_chunk_); h_chunk_counters_per_chunk_ = nullptr; }
    for (int s = 0; s < N_STREAMS; s++) {
        if (h_n_emits_[s]) { cudaFreeHost(h_n_emits_[s]); h_n_emits_[s] = nullptr; }
        if (h_n_carry_[s]) { cudaFreeHost(h_n_carry_[s]); h_n_carry_[s] = nullptr; }
    }
}

void CountAndPlan::free_all_() {
    pool_stop_();
    for (int s = 0; s < N_STREAMS; s++)
        if (streams_[s]) { cudaStreamDestroy(streams_[s]); streams_[s] = nullptr; }
    if (d2h_stream_)         { cudaStreamDestroy(d2h_stream_);  d2h_stream_  = nullptr; }
    if (meta_stream_)        { cudaStreamDestroy(meta_stream_); meta_stream_ = nullptr; }
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
    const uint32_t MAX_POT = MAX_SORT_PER_PIECE;
    scan_counts_b = scan_emit_b = scan_runs_b = sort_b = rle_b = hist_scan_b = 0;
    cub::DeviceScan::ExclusiveSum(nullptr, scan_counts_b,
        (uint32_t*)nullptr, (uint32_t*)nullptr, MAX_WORDS_PER_PIECE + 1);
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
        CUDA_CHECK(cub::DeviceScan::ExclusiveSum(d_temp_hist_, d_temp_hist_bytes_,
            d_histogram_ + 0, d_prefix_ + 0, n_rom));

        uint32_t n_in = h_max_compact_[REGION_INPUT] + 2;
        CUDA_CHECK(cub::DeviceScan::ExclusiveSum(d_temp_hist_, d_temp_hist_bytes_,
            d_histogram_ + N_ADDR_ROM, d_prefix_ + N_ADDR_ROM, n_in));
        add_const_kernel<<<(n_in + 255) / 256, 256>>>(
            d_prefix_ + N_ADDR_ROM, d_prefix_ + h_max_compact_[REGION_ROM] + 1, n_in);
        CUDA_CHECK_LAUNCH();

        uint32_t n_ram = h_max_compact_[REGION_RAM] + 2;
        CUDA_CHECK(cub::DeviceScan::ExclusiveSum(d_temp_hist_, d_temp_hist_bytes_,
            d_histogram_ + N_ADDR_ROM + N_ADDR_INPUT,
            d_prefix_ + N_ADDR_ROM + N_ADDR_INPUT, n_ram));
        add_const_kernel<<<(n_ram + 255) / 256, 256>>>(
            d_prefix_ + N_ADDR_ROM + N_ADDR_INPUT,
            d_prefix_ + N_ADDR_ROM + h_max_compact_[REGION_INPUT] + 1, n_ram);
        CUDA_CHECK_LAUNCH();
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
        num_inst_[r] = region_n_ops_[r] ? (region_n_ops_[r] + instance_rows_[r] - 1) / instance_rows_[r] : 0;
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
            region_n_ops_[r], instance_rows_[r],
            d_active_ids_ + off, d_active_first_ + off, d_active_last_ + off,
            na);
        CUDA_CHECK_LAUNCH();
    }

    int fml_block, fml_grid;
    cudaOccupancyMaxPotentialBlockSize(&fml_grid, &fml_block,
        chunk_fml_count_gappy_kernel, 0, 0);
    chunk_fml_count_gappy_kernel<<<fml_grid, fml_block>>>(
        d_ops_pool_, d_gappy_offsets_, d_packed_chunk_offsets_,
        d_active_first_, d_active_last_,
        d_fml_, num_active_, n_chunks_, num_ops_);
    CUDA_CHECK_LAUNCH();

    // Everything process_worker_ launched sits on the legacy default stream, and
    // the sync memcpys below already order against it (and against the blocking
    // count streams). A device-wide sync would additionally wait for FOREIGN
    // non-blocking streams -- e.g. proofman's streaming-commit slots -- adopting
    // their whole backlog into this critical path. Sync only our own stream.
    CUDA_CHECK(cudaStreamSynchronize(nullptr));
    CUDA_CHECK(cudaMemcpy(h_active_first_.data(), d_active_first_, num_active_ * 4, cudaMemcpyDeviceToHost));
    CUDA_CHECK(cudaMemcpy(h_active_last_.data(),  d_active_last_,  num_active_ * 4, cudaMemcpyDeviceToHost));

    std::vector<uint32_t> h_offset_starts(num_active_);
    uint32_t total_addrs = 0;
    for (uint32_t i = 0; i < num_active_; i++) {
        h_offset_starts[i] = total_addrs;
        total_addrs += h_active_last_[i] - h_active_first_[i] + 1;
    }

    // Sizing tracker for the paged buffers below (in bytes, equivalent to the
    // dense addr_offsets footprint — one u32 per addr).
    size_t needed_offsets_bytes = (size_t)total_addrs * sizeof(uint32_t);
    // Paged-output buffers grow on demand:
    //   page-meta arrays: bounded by ceil(total_addrs / MEM_OFFSETS_PAGE_SIZE) + 1 entry
    //                     per instance (small).
    //   pages_dense:      bounded by total_addrs (every page present).
    size_t needed_page_meta_bytes =
        ((size_t)total_addrs / MEM_OFFSETS_PAGE_SIZE + num_active_ + 1) * sizeof(uint32_t);
    if (needed_page_meta_bytes > h_page_meta_buf_size_) {
        if (h_page_starts_buf_) CUDA_CHECK(cudaFreeHost(h_page_starts_buf_));
        if (h_page_single_buf_)  CUDA_CHECK(cudaFreeHost(h_page_single_buf_));
        h_page_meta_buf_size_ = needed_page_meta_bytes + (needed_page_meta_bytes / 4);
        CUDA_CHECK(cudaMallocHost(&h_page_starts_buf_, h_page_meta_buf_size_));
        CUDA_CHECK(cudaMallocHost(&h_page_single_buf_,  h_page_meta_buf_size_));
    }
    if (needed_offsets_bytes > h_pages_dense_buf_size_) {
        if (h_pages_dense_buf_) CUDA_CHECK(cudaFreeHost(h_pages_dense_buf_));
        h_pages_dense_buf_size_ = needed_offsets_bytes + (needed_offsets_bytes / 4);
        CUDA_CHECK(cudaMallocHost(&h_pages_dense_buf_, h_pages_dense_buf_size_));
    }

    // Per-instance region base positions for the fused addr-offsets
    // computation inside compact_paged_kernel.
    std::vector<uint32_t> h_inst_base_pos(num_active_);
    {
        uint32_t ai = 0;
        for (uint8_t r = 0; r < 3; r++)
            for (uint32_t j = 0; j < num_active_per_[r]; j++, ai++)
                h_inst_base_pos[ai] = region_ops_start_[r]
                    + h_active_local_ids_[active_offset_[r] + j] * instance_rows_[r];
    }
    CUDA_CHECK(cudaMemcpyAsync(d_inst_base_pos_, h_inst_base_pos.data(),
                               num_active_ * 4, cudaMemcpyHostToDevice, d2h_stream_));

    for (uint8_t r = 0; r < 3; r++) {
        if (num_active_per_[r] == 0) continue;
        uint32_t na  = num_active_per_[r];
        uint32_t off = active_offset_[r];

        build_metas_kernel<<<na, 256, 0, meta_stream_>>>(
            d_fml_ + (size_t)off * n_chunks_ * 3, d_prefix_,
            REGION_ADDR_START[r], region_n_ops_[r], instance_rows_[r],
            d_active_ids_ + off, d_active_first_ + off, d_active_last_ + off,
            d_result_nops_ + (size_t)off * n_chunks_,
            d_meta_scalars_ + off * 4, na, n_chunks_);
        CUDA_CHECK_LAUNCH();
    }

    CUDA_CHECK(cudaMemcpyAsync(h_meta_scalars_, d_meta_scalars_,
                               num_active_ * 4 * 4, cudaMemcpyDeviceToHost, meta_stream_));
    CUDA_CHECK(cudaMemcpyAsync(h_result_nops_, d_result_nops_,
                               (size_t)num_active_ * n_chunks_ * 4, cudaMemcpyDeviceToHost, meta_stream_));

    // Dense → paged compaction.
    // Compute per-instance num_pages and prefix sums (host-side over a
    // handful of values).
    std::vector<uint32_t> h_num_pages(num_active_);
    std::vector<uint32_t> h_page_meta_prefix(num_active_);
    std::vector<uint32_t> h_pages_dense_dev_prefix(num_active_);
    uint32_t total_pages         = 0;
    uint32_t max_pages_per_inst  = 0;
    for (uint32_t i = 0; i < num_active_; i++) {
        const uint32_t num_addrs = h_active_last_[i] - h_active_first_[i] + 1;
        const uint32_t np = (num_addrs + MEM_OFFSETS_PAGE_SIZE - 1) / MEM_OFFSETS_PAGE_SIZE;
        h_num_pages[i] = np;
        h_page_meta_prefix[i] = total_pages;
        h_pages_dense_dev_prefix[i] = total_pages;  // worst-case device reservation: np pages per instance
        total_pages += np;
        if (np > max_pages_per_inst) max_pages_per_inst = np;
    }

    // The dense pages take the dynamic region above the ops pool, which the counting kernel
    // above was the last to read, up to the retained accesses.
    {
        const size_t pages_bytes = (size_t)total_pages * MEM_OFFSETS_PAGE_SIZE * 4;
        const size_t pages_off =
            (pool_end_bytes(pool_cursor_u32_.load(std::memory_order_relaxed)) + 255) & ~(size_t)255;
        if (pages_off + pages_bytes > ram_low_edge_bytes(ram_cursor_.load(std::memory_order_relaxed))) {
            fprintf(stderr, "CountAndPlan FATAL: no room for %zu MB of offset pages between the ops pool "
                            "and the retained RAM accesses\n", pages_bytes >> 20);
            std::exit(1);
        }
        d_pages_dense_ = (uint32_t*)(arena_ + pages_off);
        uint32_t max_words = 0, max_pot = 0;
        for (uint32_t ch = 0; ch < n_chunks_; ++ch) {
            max_words = std::max(max_words, n_words_per_chunk_[ch]);
            max_pot = std::max(max_pot, n_potentials_per_chunk_[ch]);
        }
        fprintf(stderr, "[mops] block: %u chunks, largest %u words and %u potentials, up to %u pieces per chunk "
                        "(%u potentials each), ops pool %zu MB, retained %zu accesses (%zu MB), offset pages %zu MB\n",
                n_chunks_, max_words, max_pot, max_pieces_.load(std::memory_order_relaxed), piece_potentials_,
                pool_end_bytes(pool_cursor_u32_.load(std::memory_order_relaxed)) - cursor_ >> 20,
                ram_cursor_.load(std::memory_order_relaxed),
                ram_cursor_.load(std::memory_order_relaxed) * RAM_RECORD_WORDS * 4 >> 20, pages_bytes >> 20);
    }

    // Push prefix sums to device, zero the per-instance present counters.
    CUDA_CHECK(cudaMemcpyAsync(d_page_meta_starts_, h_page_meta_prefix.data(),
                               num_active_ * 4, cudaMemcpyHostToDevice, d2h_stream_));
    CUDA_CHECK(cudaMemcpyAsync(d_pages_dense_starts_, h_pages_dense_dev_prefix.data(),
                               num_active_ * 4, cudaMemcpyHostToDevice, d2h_stream_));
    CUDA_CHECK(cudaMemsetAsync(d_present_counters_, 0,
                               num_active_ * 4, d2h_stream_));

    // One block per (instance, page); one thread per slot.
    dim3 compact_grid(max_pages_per_inst, num_active_);
    compact_paged_kernel<<<compact_grid, MEM_OFFSETS_PAGE_SIZE, 0, d2h_stream_>>>(
        d_prefix_, d_active_ids_, d_inst_base_pos_,
        d_active_first_, d_active_last_,
        d_page_meta_starts_, d_pages_dense_starts_,
        num_active_,
        d_present_counters_, d_page_starts_, d_page_single_, d_pages_dense_);
    CUDA_CHECK_LAUNCH();

    // Pull per-instance present counts so we know the per-instance D2H sizes.
    CUDA_CHECK(cudaMemcpyAsync(h_present_counters_, d_present_counters_,
                               num_active_ * 4, cudaMemcpyDeviceToHost, d2h_stream_));

    CUDA_CHECK(cudaStreamSynchronize(meta_stream_));
    CUDA_CHECK(cudaStreamSynchronize(d2h_stream_));

    // Per-instance prefix sum of present pages — gives the host-side dense
    // slice base for each instance's compacted output.
    std::vector<uint32_t> h_pages_dense_host_prefix(num_active_);
    uint32_t total_present_pages = 0;
    for (uint32_t i = 0; i < num_active_; i++) {
        h_pages_dense_host_prefix[i] = total_present_pages;
        total_present_pages += h_present_counters_[i];
    }

    // D2H the paged metadata (page_starts + page_single_value — total_pages
    // entries each) and the per-instance compact dense slices.
    CUDA_CHECK(cudaMemcpyAsync(h_page_starts_buf_, d_page_starts_,
                               (size_t)total_pages * 4, cudaMemcpyDeviceToHost, d2h_stream_));
    CUDA_CHECK(cudaMemcpyAsync(h_page_single_buf_, d_page_single_,
                               (size_t)total_pages * 4, cudaMemcpyDeviceToHost, d2h_stream_));
    for (uint32_t i = 0; i < num_active_; i++) {
        const uint32_t pc = h_present_counters_[i];
        if (pc == 0) continue;
        uint32_t*       dst = h_pages_dense_buf_
                              + (size_t)h_pages_dense_host_prefix[i] * MEM_OFFSETS_PAGE_SIZE;
        const uint32_t* src = d_pages_dense_
                              + (size_t)h_pages_dense_dev_prefix[i]  * MEM_OFFSETS_PAGE_SIZE;
        CUDA_CHECK(cudaMemcpyAsync(dst, src,
                                   (size_t)pc * MEM_OFFSETS_PAGE_SIZE * 4,
                                   cudaMemcpyDeviceToHost, d2h_stream_));
    }
    CUDA_CHECK(cudaStreamSynchronize(d2h_stream_));

    // Wire metas: pointers into the pinned host buffers.
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
            const uint32_t num_addrs   = h_active_last_[ai] - h_active_first_[ai] + 1;
            metas_[ai].count_per_chunk = h_result_nops_ + (size_t)ai * n_chunks_;
            metas_[ai].n_chunks        = n_chunks_;
            metas_[ai].offsets.num_pages        = h_num_pages[ai];
            metas_[ai].offsets.present_count    = h_present_counters_[ai];
            metas_[ai].offsets.addr_range_slots = num_addrs;
            metas_[ai].offsets.page_starts       = h_page_starts_buf_ + h_page_meta_prefix[ai];
            metas_[ai].offsets.page_single_value = h_page_single_buf_  + h_page_meta_prefix[ai];
            metas_[ai].offsets.pages_dense       = h_pages_dense_buf_
                                           + (size_t)h_pages_dense_host_prefix[ai] * MEM_OFFSETS_PAGE_SIZE;
        }
    }
}
