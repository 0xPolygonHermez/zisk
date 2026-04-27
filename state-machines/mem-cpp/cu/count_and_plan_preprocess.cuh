#ifndef MEM_PREPROCESS_CUH
#define MEM_PREPROCESS_CUH

#include <cstdint>
#include <cuda_runtime.h>

// =====================================================================
// Constants mirrored from zisk/mem_config.hpp
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

// Sizing & address-space constants (constexpr, not macros, to avoid
// polluting any other headers — notably CUB — that include us.)
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
constexpr uint32_t ZISK_MAX_CHUNKS_HOST        = 4096;

// =====================================================================
// Device-side structs
// =====================================================================

struct __align__(8) MemOp {
    uint32_t addr;
    uint32_t flags;
};

// One potential emission. We carry the raw aligned address (which has bits
// 0-2 = 0) and a small flags word. is_ram lets us skip non-RAM in the sort/
// scan; kind drives the state-machine.
struct __align__(8) PotentialEmit {
    uint32_t aligned_addr;  // 8-byte aligned raw address (or 0 if unused slot)
    uint32_t flags;         // bit 0 = is_ram, bit 1 = kind (0=R, 1=W), bit 2 = used
};

#define POT_FLAG_IS_RAM   0x1u
#define POT_FLAG_KIND_W   0x2u
#define POT_FLAG_USED     0x4u

struct BlockOpSpill {
    uint32_t memop_idx;
    uint32_t aligned_base;
    uint32_t count;
    uint32_t potential_base;  // filled in after prefix sum
    uint32_t kind_w;          // 1 if write, 0 if read
};

struct ChunkCounters {
    uint32_t full_5;
    uint32_t full_3;
    uint32_t full_2;
    uint32_t read_byte;
    uint32_t write_byte;
};

// =====================================================================
// Helpers
// =====================================================================

// True iff `addr` lies inside the RAM region [0xA0000000, 0xA0000000+512MB).
// Used to gate which potential emissions go through the duality state machine
// (only RAM addresses do; ROM/INPUT always emit).
//
// Input:  addr — raw hardware address
// Returns:     — true if RAM, false otherwise
__host__ __device__ __forceinline__
bool is_ram_addr(uint32_t addr) {
    return (addr >= ZISK_RAM_ADDR_BASE) && (addr < ZISK_RAM_ADDR_END);
}

// Map an 8-byte-aligned RAM address to a compact 0..(RAM_SIZE/8 − 1) index.
// Used as the sort key so RAM events group by address after CUB SortPairs.
//
// Input:  aligned_addr — RAM address with bits 0..2 == 0
// Returns:             — compact RAM slot index (≤ 26 bits)
__host__ __device__ __forceinline__
uint32_t ram_compact(uint32_t aligned_addr) {
    return (aligned_addr - ZISK_RAM_ADDR_BASE) >> 3;
}

#define CUDA_CHECK(call) do {                                                  \
    cudaError_t _err = (call);                                                 \
    if (_err != cudaSuccess) {                                                 \
        fprintf(stderr, "CUDA error %s at %s:%d: %s\n",                        \
                cudaGetErrorString(_err), __FILE__, __LINE__, #call);          \
        exit(1);                                                               \
    }                                                                          \
} while (0)

// =====================================================================
// Decode: per-memop potential count
//
// Mirrors the switch in zisk/mem_counter_single.cpp::execute. Always returns
// the full number of potential emissions — block ops return their `count`
// regardless of size. The BLOCKOP_SPILL_THRESH_VAL threshold is used later,
// only to decide which kernel fills the reserved slots in d_potentials:
// small blocks are inlined in decode_emit_kernel, large ones in
// blockop_emit_kernel.
//
// Input:  op       — single memop record (addr + flags)
// Returns:         — potential emission count for this memop:
//                    1 for READ_*, ALIGNED_READ, ALIGNED_WRITE, READ_8 aligned;
//                    2 for WRITE_1/CWRITE_1, WRITE_2/4 not straddling, READ_2/4
//                      straddling, READ_8 straddling, WRITE_8 perfectly aligned;
//                    4 for WRITE_2/4 straddling, WRITE_8 misaligned;
//                    `flags >> 4` for BLOCK_READ / BLOCK_WRITE (and ALIGNED_*);
//                    0 for unknown modes.
// =====================================================================

__device__ __forceinline__
uint32_t decode_potential_count(MemOp op) {
    const uint32_t addr        = op.addr;
    const uint32_t aligned     = addr & ZISK_ALIGN_MASK;
    const uint8_t  mode        = op.flags & 0x3F;
    const uint32_t off_in_word = addr & 0x07;

    switch (mode) {
        case MOPS_READ_1:    return 1;
        case MOPS_CWRITE_1:  return 2;
        case MOPS_WRITE_1:   return 2;

        case MOPS_READ_2:    return (off_in_word > 6) ? 2 : 1;
        case MOPS_WRITE_2:   return (off_in_word > 6) ? 4 : 2;

        case MOPS_READ_4:    return (off_in_word > 4) ? 2 : 1;
        case MOPS_WRITE_4:   return (off_in_word > 4) ? 4 : 2;

        case MOPS_READ_8:    return (off_in_word > 0) ? 2 : 1;
        case MOPS_WRITE_8:   return (addr == aligned) ? 1 : 4;

        case MOPS_ALIGNED_READ  + 0x00: case MOPS_ALIGNED_READ  + 0x10:
        case MOPS_ALIGNED_READ  + 0x20: case MOPS_ALIGNED_READ  + 0x30:
            return 1;

        case MOPS_ALIGNED_WRITE + 0x00: case MOPS_ALIGNED_WRITE + 0x10:
        case MOPS_ALIGNED_WRITE + 0x20: case MOPS_ALIGNED_WRITE + 0x30:
            return 1;

        case MOPS_BLOCK_READ        + 0x00: case MOPS_BLOCK_READ        + 0x10:
        case MOPS_BLOCK_READ        + 0x20: case MOPS_BLOCK_READ        + 0x30:
        case MOPS_ALIGNED_BLOCK_READ+ 0x00: case MOPS_ALIGNED_BLOCK_READ+ 0x10:
        case MOPS_ALIGNED_BLOCK_READ+ 0x20: case MOPS_ALIGNED_BLOCK_READ+ 0x30:
        case MOPS_BLOCK_WRITE        + 0x00: case MOPS_BLOCK_WRITE        + 0x10:
        case MOPS_BLOCK_WRITE        + 0x20: case MOPS_BLOCK_WRITE        + 0x30:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x00: case MOPS_ALIGNED_BLOCK_WRITE+ 0x10:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x20: case MOPS_ALIGNED_BLOCK_WRITE+ 0x30:
            return op.flags >> MOPS_BLOCK_COUNT_SBITS;

        default:
            // Invalid mode — should not happen on real data. Returning 0 makes
            // the chunk visibly degenerate so verify catches it.
            return 0;
    }
}

// =====================================================================
// Decode: counter deltas (full_5, full_3, full_2, read_byte, write_byte)
// Mirrors the side-effects of the same switch in mem_counter_single.cpp.
//
// Input:  op      — single memop record
// Returns:        — ChunkCounters with at most one field set to 1, the rest 0.
//                   READ_1 → read_byte; CWRITE_1 → write_byte; WRITE_1,
//                   {READ,WRITE}_2/4/8 fall into full_5/3/2 by alignment.
//                   ALIGNED_* and BLOCK_* don't touch counters.
// =====================================================================

__device__ __forceinline__
ChunkCounters decode_counter_deltas(MemOp op) {
    const uint32_t addr        = op.addr;
    const uint32_t aligned     = addr & ZISK_ALIGN_MASK;
    const uint8_t  mode        = op.flags & 0x3F;
    const uint32_t off_in_word = addr & 0x07;
    ChunkCounters c{0,0,0,0,0};

    switch (mode) {
        case MOPS_READ_1:    c.read_byte  = 1; break;
        case MOPS_CWRITE_1:  c.write_byte = 1; break;
        case MOPS_WRITE_1:   c.full_3     = 1; break;

        case MOPS_READ_2:    if (off_in_word > 6) c.full_3 = 1; else c.full_2 = 1; break;
        case MOPS_WRITE_2:   if (off_in_word > 6) c.full_5 = 1; else c.full_3 = 1; break;
        case MOPS_READ_4:    if (off_in_word > 4) c.full_3 = 1; else c.full_2 = 1; break;
        case MOPS_WRITE_4:   if (off_in_word > 4) c.full_5 = 1; else c.full_3 = 1; break;
        case MOPS_READ_8:    if (off_in_word > 0) c.full_3 = 1;                      break;
        case MOPS_WRITE_8:   if (addr != aligned) c.full_5 = 1;                      break;

        // ALIGNED_*, BLOCK_* don't touch counters.
        default: break;
    }
    return c;
}

// =====================================================================
// Decode: emit potentials (writes 1..4 PotentialEmit slots starting at out;
// or `count` slots for block ops handled inline).
//
// Block-op dispatch is decided by the caller via the `skip_block` parameter
// of decode_emit_inline:
//   - Small blocks (count <= BLOCKOP_SPILL_THRESH_VAL): never spilled by
//     decode_count_kernel, so caller passes skip_block=false → handled inline.
//   - Large blocks (count > BLOCKOP_SPILL_THRESH_VAL) that fit in the spill
//     table: caller passes skip_block=true → we write nothing, blockop_emit
//     kernel fills the slots cooperatively.
//   - Large blocks that overflow the spill table (rare): caller passes
//     skip_block=false → fallback path, one thread loops `count` times.
// =====================================================================

// emit_pair_rw / emit_one_r / emit_one_w — write a single PotentialEmit pair
// or singleton at *out. Each PotentialEmit carries:
//   aligned_addr — 8-byte aligned raw address
//   flags        — POT_FLAG_USED | (POT_FLAG_IS_RAM if RAM) | (POT_FLAG_KIND_W for W)
//
// In:     aligned — aligned 8-byte address to record (assumed valid)
// Out:    out[]   — 1 slot for one_r/one_w, 2 slots (R then W) for pair_rw

__device__ __forceinline__
void emit_pair_rw(uint32_t aligned, PotentialEmit* out) {
    const uint32_t base = (POT_FLAG_USED) | (is_ram_addr(aligned) ? POT_FLAG_IS_RAM : 0u);
    out[0].aligned_addr = aligned; out[0].flags = base;                       // R
    out[1].aligned_addr = aligned; out[1].flags = base | POT_FLAG_KIND_W;     // W
}

__device__ __forceinline__
void emit_one_r(uint32_t aligned, PotentialEmit* out) {
    const uint32_t base = (POT_FLAG_USED) | (is_ram_addr(aligned) ? POT_FLAG_IS_RAM : 0u);
    out[0].aligned_addr = aligned; out[0].flags = base;
}

__device__ __forceinline__
void emit_one_w(uint32_t aligned, PotentialEmit* out) {
    const uint32_t base = (POT_FLAG_USED) | (is_ram_addr(aligned) ? POT_FLAG_IS_RAM : 0u);
    out[0].aligned_addr = aligned; out[0].flags = base | POT_FLAG_KIND_W;
}

// Decode one memop into 1..4 PotentialEmit records (or 0..count for blocks).
// Mirrors zisk/mem_counter_single.cpp::execute one-for-one.
//
// Big-block dispatch is decided by the caller via `skip_block`:
//   - skip_block == true  → block op was claimed by blockop_emit_kernel; we
//                           write nothing here for block-op modes.
//   - skip_block == false → caller wants this memop's slots filled inline.
//                           For block ops with large `count` this means one
//                           thread loops `count` times — slow but correct.
//                           Used as a CORRECTNESS FALLBACK when the spill
//                           table overflowed (see decode_count_kernel and
//                           the comment on MAX_BLOCKOP_SPILL_PER_CHUNK).
//
// Non-block modes are always handled inline regardless of skip_block.
//
// Input:  op          — memop to decode
// Input:  skip_block  — true iff this memop is a block op handled by spill kernel
// Output: out[0..N)   — N = decode_potential_count(op) PotentialEmit slots
__device__ __forceinline__
void decode_emit_inline(MemOp op, PotentialEmit* out, bool skip_block) {
    const uint32_t addr        = op.addr;
    const uint32_t aligned     = addr & ZISK_ALIGN_MASK;
    const uint8_t  mode        = op.flags & 0x3F;
    const uint32_t off_in_word = addr & 0x07;

    switch (mode) {
        case MOPS_READ_1:
            emit_one_r(aligned, out); break;
        case MOPS_CWRITE_1:
        case MOPS_WRITE_1:
            emit_pair_rw(aligned, out); break;

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
            if (skip_block) break;     // claimed by blockop_emit_kernel
            const uint32_t count = op.flags >> MOPS_BLOCK_COUNT_SBITS;
            for (uint32_t i = 0; i < count; i++) emit_one_r(addr + i * 8, out + i);
            break;
        }
        case MOPS_BLOCK_WRITE        + 0x00: case MOPS_BLOCK_WRITE        + 0x10:
        case MOPS_BLOCK_WRITE        + 0x20: case MOPS_BLOCK_WRITE        + 0x30:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x00: case MOPS_ALIGNED_BLOCK_WRITE+ 0x10:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x20: case MOPS_ALIGNED_BLOCK_WRITE+ 0x30: {
            if (skip_block) break;     // claimed by blockop_emit_kernel
            const uint32_t count = op.flags >> MOPS_BLOCK_COUNT_SBITS;
            for (uint32_t i = 0; i < count; i++) emit_one_w(addr + i * 8, out + i);
            break;
        }
        default: break;
    }
}

// =====================================================================
// Kernels
//
// Pipeline order (per chunk, all on one stream — see main_preprocess.cu):
//
//   [1]  H2D memops + zero d_ram_count, d_spill_count
//   [2]  decode_count_kernel       counts/chunk_counters/spill enqueue
//   [3]  CUB ExclusiveSum(d_counts)        → d_potential_offsets
//   [4]  fill_spill_bases_kernel   patches potential_base for spilled blocks
//   [5]  decode_emit_kernel        writes PotentialEmit slots (non-spilled ops)
//   [6]  blockop_emit_kernel       writes PotentialEmit slots for big blocks
//   [7]  gather_ram_events_kernel  splits RAM (sort/scan) vs non-RAM (emit=1)
//   [8]  CUB SortPairs             sort RAM events by (addr, orig_pos)
//   [9]  extract_sorted_addr_kernel  → dense compact_addr column for RLE
//   [10] CUB RunLengthEncode + ExclusiveSum  → d_run_offsets, d_num_unique
//   [11] state_machine_by_run_kernel  per-segment state machine, scatters bits
//   [12] CUB ExclusiveSum(d_emit_bits)        → d_final_offsets
//   [13] compact_kernel            scatter surviving aligned addresses to d_out
//   [14] D2H d_out
// =====================================================================

// Per-block shared-memory reducer for ChunkCounters. Each thread contributes
// its `my` to a block-private staging copy in shared memory; thread 0 then
// flushes that copy to *g_dst with one atomicAdd per field.
//
// Input:  my     — this thread's per-memop counter delta (mostly zero fields)
// In/Out: g_dst  — global ChunkCounters slot for this chunk; atomicAdded into
__device__ __forceinline__
void block_reduce_counters(const ChunkCounters& my, ChunkCounters* g_dst) {
    typedef unsigned long long ull;
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

// Helper: returns true iff `mode` (low 6 bits of MemOp.flags) is one of the
// recognised opcodes. We check this in decode_count_kernel so the host can
// fail loud on corrupted input — mem_counter_single.cpp throws in this case;
// the GPU silently produced 0 emissions before this check was added.
__host__ __device__ __forceinline__
bool is_known_mode(uint8_t mode) {
    switch (mode) {
        case MOPS_READ_1:    case MOPS_CWRITE_1:  case MOPS_WRITE_1:
        case MOPS_READ_2:    case MOPS_WRITE_2:
        case MOPS_READ_4:    case MOPS_WRITE_4:
        case MOPS_READ_8:    case MOPS_WRITE_8:
        case MOPS_ALIGNED_READ  + 0x00: case MOPS_ALIGNED_READ  + 0x10:
        case MOPS_ALIGNED_READ  + 0x20: case MOPS_ALIGNED_READ  + 0x30:
        case MOPS_ALIGNED_WRITE + 0x00: case MOPS_ALIGNED_WRITE + 0x10:
        case MOPS_ALIGNED_WRITE + 0x20: case MOPS_ALIGNED_WRITE + 0x30:
        case MOPS_BLOCK_READ        + 0x00: case MOPS_BLOCK_READ        + 0x10:
        case MOPS_BLOCK_READ        + 0x20: case MOPS_BLOCK_READ        + 0x30:
        case MOPS_ALIGNED_BLOCK_READ+ 0x00: case MOPS_ALIGNED_BLOCK_READ+ 0x10:
        case MOPS_ALIGNED_BLOCK_READ+ 0x20: case MOPS_ALIGNED_BLOCK_READ+ 0x30:
        case MOPS_BLOCK_WRITE        + 0x00: case MOPS_BLOCK_WRITE        + 0x10:
        case MOPS_BLOCK_WRITE        + 0x20: case MOPS_BLOCK_WRITE        + 0x30:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x00: case MOPS_ALIGNED_BLOCK_WRITE+ 0x10:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x20: case MOPS_ALIGNED_BLOCK_WRITE+ 0x30:
            return true;
        default:
            return false;
    }
}

// [Step 1] One thread per memop. Writes its potential-emission count to
// d_counts[i] (consumed by the upcoming exclusive-sum), accumulates the
// per-chunk counter deltas via block_reduce_counters, and queues big block ops
// (count > BLOCKOP_SPILL_THRESH_VAL) into d_spill for later cooperative
// expansion in blockop_emit_kernel.
//
// Per-memop flag d_spill_status[i] is set to 1 iff the memop was successfully
// claimed by the spill table. decode_emit_kernel uses this flag to decide
// whether to skip the memop (spill kernel will fill its slots) or to fill
// them inline (correctness fallback for the rare case where the spill table
// overflowed — i.e. more than MAX_BLOCKOP_SPILL_PER_CHUNK big blocks in a
// single chunk).
//
// d_invalid_mode_flag is atomicOr'd with 1 if any thread sees an unrecognised
// mode byte. The host inspects this after the chunk and aborts — silent wrong
// answers on corrupted input would otherwise be possible because every other
// switch in this file falls through to default: break for unknown modes.
//
// Caller must zero *d_spill_count, d_spill_status[0..n_memops), and
// *d_chunk_counters_entry before processing the first chunk; the host owns
// resetting *d_invalid_mode_flag as well.
//
// Input:  memops[n_memops]                  — per-chunk MemOp records (H2D'd)
// Input:  n_memops                          — count
// Output: d_counts[n_memops]                — potential emissions per memop
// Output: d_spill_status[n_memops]          — 1 = spilled (skip in decode_emit),
//                                             0 = handle inline in decode_emit
// In/Out: d_chunk_counters_entry            — atomicAdd-style accumulation
// In/Out: d_spill[*d_spill_count..]         — appended spill records for big blocks
// In/Out: d_spill_count                     — atomically incremented per spill;
//                                             may temporarily exceed
//                                             MAX_BLOCKOP_SPILL_PER_CHUNK on
//                                             overflow (consumers cap to MAX
//                                             via the fixed grid)
// In/Out: d_invalid_mode_flag               — set to 1 on first unknown mode
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
        const uint8_t mode = op.flags & 0x3F;

        if (!is_known_mode(mode)) {
            // Bad input. Flag for host inspection. Still set d_counts[i] = 0
            // (matches decode_potential_count's default) so the prefix sum
            // doesn't read garbage.
            atomicOr(d_invalid_mode_flag, 1u);
            d_counts[i] = 0;
        } else {
            d_counts[i] = decode_potential_count(op);
            my = decode_counter_deltas(op);

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
                        s.memop_idx       = i;
                        s.aligned_base    = op.addr;     // already aligned for blocks
                        s.count           = count;
                        s.potential_base  = 0;            // filled later
                        s.kind_w          = is_block_write ? 1u : 0u;
                        d_spill[slot] = s;
                        d_spill_status[i] = 1;            // claimed by spill kernel
                    }
                    // Else: spill table is full. d_spill_status[i] stays 0, so
                    // decode_emit_kernel will handle this big block inline.
                    // *d_spill_count is left incremented; consumers iterate at
                    // most MAX_BLOCKOP_SPILL_PER_CHUNK (the grid bound) and only
                    // read d_spill[0..MAX), all of which are valid.
                }
            }
        }
    }
    block_reduce_counters(my, d_chunk_counters_entry);
}

// [Step 4] Patch potential_base on each spilled BlockOpSpill record now that
// the prefix sum (Step 3, CUB ExclusiveSum) has filled d_potential_offsets.
//
// Grid is fixed at MAX_BLOCKOP_SPILL_PER_CHUNK / BLOCK; threads past the
// actual count early-exit via *d_spill_count (avoiding a host sync).
//
// Input:  d_spill_count                          — number of valid spill entries
// In/Out: d_spill[0..*d_spill_count)             — .potential_base set per entry
// Input:  d_potential_offsets[memop_idx]         — slot base for that memop
__global__
void fill_spill_bases_kernel(BlockOpSpill* __restrict__ d_spill,
                             const uint32_t* __restrict__ d_spill_count,
                             const uint32_t* __restrict__ d_potential_offsets) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    // *d_spill_count may exceed MAX_BLOCKOP_SPILL_PER_CHUNK on overflow
    // (decode_count_kernel keeps the atomic counter monotonic but only
    // writes records for slot < MAX). Cap at MAX so we never read an
    // uninitialised d_spill slot.
    const uint32_t cap = min(*d_spill_count, MAX_BLOCKOP_SPILL_PER_CHUNK);
    if (i >= cap) return;
    d_spill[i].potential_base = d_potential_offsets[d_spill[i].memop_idx];
}

// [Step 5] One thread per memop. Writes the memop's potential emissions at
// the slot range [d_potential_offsets[i] .. d_potential_offsets[i+1]) in
// d_potentials.
//
// d_spill_status[i] tells us per memop whether to skip (spill kernel handles)
// or inline (we handle here):
//   - 1 → big block claimed by blockop_emit_kernel; we skip.
//   - 0 → handle inline. For non-block memops this is the fast path. For big
//         block memops where the spill table overflowed, this is the
//         correctness fallback — one thread loops `count` times.
//
// Input:  memops[n_memops]                  — same memop array as Step 1
// Input:  n_memops                          — count
// Input:  d_potential_offsets[n_memops + 1] — exclusive prefix sum from Step 3
// Input:  d_spill_status[n_memops]          — 1 = spilled (skip), 0 = inline
// Output: d_potentials[0..total_potentials) — populated for all non-spilled slots
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

// [Step 6] One CTA per spilled big block. The CTA's threads stride through
// the block's `count` aligned addresses and write a PotentialEmit per slot.
//
// Grid is fixed at MAX_BLOCKOP_SPILL_PER_CHUNK; CTAs with blockIdx.x >=
// *d_spill_count early-exit (avoiding a host sync). decode_emit_kernel
// already skipped these slots, so this step completes coverage of
// d_potentials[0..total_potentials).
//
// Input:  d_spill[0..*d_spill_count)             — spill records (with potential_base)
// Input:  d_spill_count                          — number of valid spill entries
// Output: d_potentials[s.potential_base..+count) — populated for each spill
__global__
void blockop_emit_kernel(const BlockOpSpill* __restrict__ d_spill,
                         const uint32_t* __restrict__ d_spill_count,
                         PotentialEmit* __restrict__ d_potentials) {
    // Defensive cap: same reasoning as in fill_spill_bases_kernel. Grid is
    // launched at MAX_BLOCKOP_SPILL_PER_CHUNK so blockIdx.x < MAX already,
    // but we cap here too for symmetry / future-proofing.
    const uint32_t cap = min(*d_spill_count, MAX_BLOCKOP_SPILL_PER_CHUNK);
    if (blockIdx.x >= cap) return;
    const BlockOpSpill s = d_spill[blockIdx.x];
    const uint32_t base_addr = s.aligned_base;
    const uint32_t count     = s.count;
    PotentialEmit* base = d_potentials + s.potential_base;
    const uint32_t flags_base = POT_FLAG_USED | (s.kind_w ? POT_FLAG_KIND_W : 0u);
    for (uint32_t i = threadIdx.x; i < count; i += blockDim.x) {
        const uint32_t a = base_addr + i * 8u;
        base[i].aligned_addr = a;
        base[i].flags        = flags_base | (is_ram_addr(a) ? POT_FLAG_IS_RAM : 0u);
    }
}

// Sort-key layout (tight, 47-bit pack):
//   bits [0..20]  = orig_pos      (≤ 21 bits, fits POTENTIAL_CAP_PER_CHUNK=2M)
//   bits [21..46] = compact_addr  (26 bits, fits 64M RAM slots)
// Sorting by this key groups events by address and orders by chunk-local
// orig_pos within each group. RAM_KEY_END_BIT is passed to CUB SortPairs.
//
// Sort value: (kind_w << 31) | orig_pos — state_machine_by_run_kernel uses
// orig_pos to scatter the emit bit.
#define ORIG_POS_BITS 21u
#define ORIG_POS_MASK ((1u << ORIG_POS_BITS) - 1u)
#define RAM_KEY_END_BIT 47

// [Step 7] One thread per potential slot. Splits the d_potentials stream:
//   - non-RAM (or unused) slots: emit_bit set directly here (always 1 for
//     used non-RAM, 0 for the unused tail), no further processing.
//   - RAM slots: the slot is atomically compacted into d_ram_keys/d_ram_vals
//     and emit_bit defaults to 0 (will be overwritten by Step 11).
//
// Input:  d_potentials[0..n_potentials)          — populated by Steps 5 + 6
// Input:  n_potentials                           — exact upper bound (CPU pre-computed)
// Output: d_ram_keys[0..*d_ram_count)            — packed (compact_addr, orig_pos)
// Output: d_ram_vals[0..*d_ram_count)            — packed (kind_w, orig_pos)
// In/Out: d_ram_count                            — atomic compaction counter
// Output: d_emit_bits[0..n_potentials)           — 1 for non-RAM emits, 0 for RAM/unused
__global__
void gather_ram_events_kernel(const PotentialEmit* __restrict__ d_potentials,
                              uint32_t n_potentials,
                              uint64_t* __restrict__ d_ram_keys,
                              uint32_t* __restrict__ d_ram_vals,
                              uint32_t* __restrict__ d_ram_count,
                              uint32_t* __restrict__ d_emit_bits) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n_potentials) return;
    PotentialEmit p = d_potentials[i];
    const bool used   = (p.flags & POT_FLAG_USED)   != 0;
    const bool is_ram = (p.flags & POT_FLAG_IS_RAM) != 0;
    const bool kind_w = (p.flags & POT_FLAG_KIND_W) != 0;

    if (!used) { d_emit_bits[i] = 0; return; }

    if (is_ram) {
        const uint32_t compact = ram_compact(p.aligned_addr);
        const uint64_t key = ((uint64_t)compact << ORIG_POS_BITS) | (uint64_t)i;
        const uint32_t val = ((uint32_t)(kind_w ? 1u : 0u) << 31) | i;
        const uint32_t slot = atomicAdd(d_ram_count, 1u);
        d_ram_keys[slot] = key;
        d_ram_vals[slot] = val;
        d_emit_bits[i] = 0;  // will be overwritten by state_machine_scan_kernel
    } else {
        d_emit_bits[i] = 1;  // non-RAM always emits
    }
}

// [Step 9] Extract the compact_addr high-bits from each sorted key into a
// dense uint32 array, so cub::DeviceRunLengthEncode can group events by
// address (RLE compares whole values, and the low 21 bits of the sort key
// hold orig_pos which differs within a group — we strip them here).
//
// Input:  d_sorted_keys[0..n_events)             — output of CUB SortPairs (Step 8)
// Input:  n_events                               — n_ram for this chunk
// Output: d_sorted_addr[0..n_events)             — high bits, ready for RLE
__global__
void extract_sorted_addr_kernel(const uint64_t* __restrict__ d_sorted_keys,
                                uint32_t n_events,
                                uint32_t* __restrict__ d_sorted_addr) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n_events) return;
    d_sorted_addr[i] = (uint32_t)(d_sorted_keys[i] >> ORIG_POS_BITS);
}

// [Step 11] Per-segment R/W state machine. One thread per unique RAM address
// (RLE output, typically n_unique ≪ n_ram). Each thread reads its segment's
// [start, end) from d_run_offsets, walks the events in chunk-order, applies
// the duality rule, and scatters the resulting emit bits to d_emit_bits at
// each event's original slot index.
//
// State machine (state starts false per chunk):
//   R: if state==true → emit=0, state=false
//      else            → emit=1, state=true
//   W: emit=1, state=true (always)
//
// Grid is conservative (g_ram blocks); threads with t >= *d_num_unique
// early-exit so no host sync is needed for the kernel launch.
//
// Input:  d_run_offsets[0..n_unique]             — RLE prefix sum (start indices)
// Input:  d_num_unique                           — n_unique on device
// Input:  d_sorted_vals[0..n_ram)                — sorted Step 8 values
// In/Out: d_emit_bits[orig_pos]                  — written for each event in segment
__global__
void state_machine_by_run_kernel(const uint32_t* __restrict__ d_run_offsets,
                                 const uint32_t* __restrict__ d_num_unique,
                                 const uint32_t* __restrict__ d_sorted_vals,
                                 uint32_t* __restrict__ d_emit_bits) {
    const uint32_t t = blockIdx.x * blockDim.x + threadIdx.x;
    if (t >= *d_num_unique) return;
    const uint32_t start = d_run_offsets[t];
    const uint32_t end   = d_run_offsets[t + 1];

    bool state = false;
    for (uint32_t j = start; j < end; j++) {
        const uint32_t v = d_sorted_vals[j];
        const bool kind_w    = (v >> 31) & 1u;
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
    }
}

// [DEAD CODE — kept for comparison] Earlier per-event implementation of the
// state machine. Launched one thread per event, did a per-thread "am I a
// segment start?" check via reading d_sorted_keys[i-1], and walked the
// segment serially when so. Replaced by state_machine_by_run_kernel which
// uses RLE to launch only n_unique threads (5–10× fewer); see [§6 of
// preprocess_walkthrough.md](preprocess_walkthrough.md) for the measurement.
//
// Safe to delete once you no longer need to A/B compare against this version.
__global__
void state_machine_scan_kernel(const uint64_t* __restrict__ d_sorted_keys,
                               const uint32_t* __restrict__ d_sorted_vals,
                               uint32_t n_events,
                               uint32_t* __restrict__ d_emit_bits) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n_events) return;
    const uint64_t k = d_sorted_keys[i];
    const uint32_t compact_addr = (uint32_t)(k >> ORIG_POS_BITS);

    bool is_segment_start;
    if (i == 0) {
        is_segment_start = true;
    } else {
        const uint32_t prev_addr = (uint32_t)(d_sorted_keys[i - 1] >> ORIG_POS_BITS);
        is_segment_start = (prev_addr != compact_addr);
    }
    if (!is_segment_start) return;

    bool state = false;
    uint32_t j = i;
    while (j < n_events) {
        const uint64_t kj = d_sorted_keys[j];
        const uint32_t addr_j = (uint32_t)(kj >> ORIG_POS_BITS);
        if (addr_j != compact_addr) break;
        const uint32_t v = d_sorted_vals[j];
        const bool kind_w = (v >> 31) & 1u;
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
        j++;
    }
}

// [Step 13] Compaction: one thread per potential slot. For each slot that
// the state machine kept alive (d_emit_bits[i] == 1), scatter the aligned
// address into d_out at the position given by d_final_offsets[i] (output of
// Step 12's exclusive sum).
//
// Input:  d_potentials[0..n_potentials)          — populated by Steps 5+6
// Input:  d_emit_bits[0..n_potentials)           — 0/1 per slot (Steps 7+11)
// Input:  d_final_offsets[0..n_potentials]       — exclusive sum of d_emit_bits
// Input:  n_potentials                           — bound on slots to scan
// Output: d_out[d_final_offsets[i]]              — written when emit_bit[i] == 1
__global__
void compact_kernel(const PotentialEmit* __restrict__ d_potentials,
                    const uint32_t* __restrict__ d_emit_bits,
                    const uint32_t* __restrict__ d_final_offsets,
                    uint32_t n_potentials,
                    uint32_t* __restrict__ d_out) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n_potentials) return;
    if (d_emit_bits[i] == 0) return;
    d_out[d_final_offsets[i]] = d_potentials[i].aligned_addr;
}

#endif  // MEM_PREPROCESS_CUH
