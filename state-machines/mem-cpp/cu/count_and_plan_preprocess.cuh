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

// One potential emission, packed into a single uint32_t.
//
// Layout:
//   bit  0     POT_FLAG_IS_RAM   1 if address falls inside the RAM region
//   bit  1     POT_FLAG_KIND_W   1 if this is a Write event (else Read)
//   bit  2     reserved          (was POT_FLAG_USED; removed)
//   bits 3..31 aligned_addr      the 8-byte-aligned address itself
//
// Aligned addresses naturally have bits 0..2 == 0, so the flag bits live in
// the address word at no cost. This halves d_potentials memory and the
// gather_ram_events_kernel read traffic vs. the previous (addr, flags) pair.
//
// Use the emit_* accessors below to extract fields — never mask by hand at
// the call site.
//
// Every slot in d_potentials[0..n_potentials) is written by Phase B
// (decode_emit or blockop_emit), so there's no need for a "is this slot
// populated?" marker: gather_ram_events_kernel only reads that range.
struct __align__(4) PotentialEmit {
    uint32_t aligned_addr_packed;
};

#define POT_FLAG_IS_RAM   0x1u
#define POT_FLAG_KIND_W   0x2u
#define POT_FLAG_MASK     0x7u   // covers all three reserved bits

// Accessors for PotentialEmit's packed layout. Always use these; never mask
// the raw word at a call site.
__host__ __device__ __forceinline__
uint32_t emit_aligned_addr(PotentialEmit p) { return p.aligned_addr_packed & ~POT_FLAG_MASK; }

__host__ __device__ __forceinline__
bool emit_is_ram(PotentialEmit p) { return (p.aligned_addr_packed & POT_FLAG_IS_RAM) != 0; }

__host__ __device__ __forceinline__
bool emit_kind_w(PotentialEmit p) { return (p.aligned_addr_packed & POT_FLAG_KIND_W) != 0; }

struct BlockOpSpill {
    uint32_t memop_idx;       // index into the chunk's MemOp array; used by
                              // blockop_emit_kernel to look up the slot base
                              // via d_potential_offsets[memop_idx] on the fly
    uint32_t aligned_base;
    uint32_t count;
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
// Decode: fused per-memop validity check, potential-emission count, and
// per-memop counter deltas. 
//
// Behaviour:
//   - Known mode → writes *count_out (potential emissions) and counters_out
//                  (at most one field set to 1, rest zero), returns true.
//   - Unknown mode → atomicOr's *d_invalid_mode_flag (sticky; host inspects
//                    after the chunk loop and aborts), writes *count_out = 0,
//                    leaves counters_out zero, returns false. The caller
//                    should skip the per-memop block-op spill logic.
//
// Counter rules (matches mem_counter_single.cpp's accumulators):
//   READ_1            → read_byte
//   CWRITE_1          → write_byte
//   WRITE_1           → full_3
//   READ_2/4/8 strad. → full_3   (else full_2 for READ_2/4, none for READ_8 aligned)
//   WRITE_2/4 strad.  → full_5   (else full_3)
//   WRITE_8 misalign. → full_5   (aligned WRITE_8 leaves all counters zero)
//   ALIGNED_*, BLOCK_* → no counter contribution
// =====================================================================

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
            *count_out = 1;
            counters_out.read_byte = 1;
            return true;
        case MOPS_CWRITE_1:
            *count_out = 2;
            counters_out.write_byte = 1;
            return true;
        case MOPS_WRITE_1:
            *count_out = 2;
            counters_out.full_3 = 1;
            return true;

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
            *count_out = 1;
            return true;

        case MOPS_BLOCK_READ        + 0x00: case MOPS_BLOCK_READ        + 0x10:
        case MOPS_BLOCK_READ        + 0x20: case MOPS_BLOCK_READ        + 0x30:
        case MOPS_ALIGNED_BLOCK_READ+ 0x00: case MOPS_ALIGNED_BLOCK_READ+ 0x10:
        case MOPS_ALIGNED_BLOCK_READ+ 0x20: case MOPS_ALIGNED_BLOCK_READ+ 0x30:
        case MOPS_BLOCK_WRITE        + 0x00: case MOPS_BLOCK_WRITE        + 0x10:
        case MOPS_BLOCK_WRITE        + 0x20: case MOPS_BLOCK_WRITE        + 0x30:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x00: case MOPS_ALIGNED_BLOCK_WRITE+ 0x10:
        case MOPS_ALIGNED_BLOCK_WRITE+ 0x20: case MOPS_ALIGNED_BLOCK_WRITE+ 0x30:
            *count_out = op.flags >> MOPS_BLOCK_COUNT_SBITS;
            return true;

        default:
            atomicOr(d_invalid_mode_flag, 1u);
            *count_out = 0;
            return false;
    }
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
//   flags        — (POT_FLAG_IS_RAM if RAM) | (POT_FLAG_KIND_W for W)
//
// In:     aligned — aligned 8-byte address to record (assumed valid)
// Out:    out[]   — 1 slot for one_r/one_w, 2 slots (R then W) for pair_rw

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
//   [1]  H2D memops + zero d_ram_count, d_spill_count, d_spill_status
//   [2]  decode_count_kernel       counts/chunk_counters/spill enqueue
//   [3]  CUB ExclusiveSum(d_counts)        → d_potential_offsets
//   [4]  decode_emit_kernel        writes PotentialEmit slots (non-spilled ops)
//   [5]  blockop_emit_kernel       writes PotentialEmit slots for big blocks
//                                  (reads slot bases from d_potential_offsets
//                                  on the fly — no separate fill pass needed)
//   [6]  gather_ram_events_kernel  splits RAM (sort/scan) vs non-RAM (emit=1)
//   [7]  CUB SortPairs             sort RAM events by (addr, orig_pos)
//   [8]  extract_sorted_addr_kernel  → dense compact_addr column for RLE
//   [9]  CUB RunLengthEncode + ExclusiveSum  → d_run_offsets, d_num_unique
//   [10] state_machine_by_run_kernel  per-segment state machine, scatters bits
//   [11] CUB ExclusiveSum(d_emit_bits)        → d_final_offsets
//   [12] compact_kernel            scatter surviving aligned addresses to d_out
//   [13] D2H d_out
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
// Caller-owned resets:
//   - *d_spill_count                — zero BEFORE EVERY CHUNK (per-stream buffer
//                                     is reused; new chunk needs a clean count)
//   - d_spill_status[0..n_memops)   — zero BEFORE EVERY CHUNK (same reason; we
//                                     memset only the active range each chunk)
//   - *d_chunk_counters_entry       — zero ONCE at startup (each chunk writes
//                                     to its own slot in the global array; we
//                                     never reuse a slot)
//   - *d_invalid_mode_flag          — zero ONCE at startup (sticky-OR; once
//                                     set, the host aborts at end-of-pipeline)
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

        // Fused: validity + d_counts[i] + counter deltas in a single switch.
        // Returns false on unknown opcode (already flagged in
        // *d_invalid_mode_flag and d_counts[i] zeroed for safe prefix sum).
        // The block-op spill logic below only runs on valid ops.
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
                        s.memop_idx       = i;
                        s.aligned_base    = op.addr;     // already aligned for blocks
                        s.count           = count;
                        s.kind_w          = is_block_write ? 1u : 0u;
                        // potential_base lookup is deferred to blockop_emit_kernel
                        // (reads d_potential_offsets[memop_idx] there).
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

// [Step 4] One thread per memop. Writes the memop's potential emissions at
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

// [Step 5] One CTA per spilled big block. The CTA's threads stride through
// the block's `count` aligned addresses and write a PotentialEmit per slot.
//
// Grid is fixed at MAX_BLOCKOP_SPILL_PER_CHUNK; CTAs with blockIdx.x >=
// *d_spill_count early-exit (avoiding a host sync). decode_emit_kernel
// already skipped these slots, so this step completes coverage of
// d_potentials[0..total_potentials).
//
// *d_spill_count may exceed MAX_BLOCKOP_SPILL_PER_CHUNK on overflow
// (decode_count_kernel keeps the atomic counter monotonic but only writes
// records for slot < MAX). The min() cap keeps us from reading uninitialised
// d_spill slots.
//
// Input:  d_spill[0..*d_spill_count)             — spill records
// Input:  d_spill_count                          — number of valid spill entries
// Input:  d_potential_offsets[memop_idx]         — slot base lookup
// Output: d_potentials[base..+count)             — populated for each spill
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

// Sort-key layout (everything in one 64-bit key — the sort uses CUB SortKeys,
// no separate value array):
//   bit  0          = kind_w        (Read/Write)
//   bits [1..21]    = orig_pos      (≤ 21 bits, fits POTENTIAL_CAP_PER_CHUNK=2M)
//   bits [22..47]   = compact_addr  (26 bits, fits 64M RAM slots)
//
// Total used bits: 48. RAM_KEY_END_BIT is passed to CUB.
//
// `orig_pos` is unique per event, so the key alone fully determines the sort
// order — kind_w in the low bit is a free passenger.
//
// AFTER the sort, extract_sorted_packed_kernel walks the sorted keys once and
// writes a compact 32-bit value `(kind_w << 31) | orig_pos` per event into
// d_ram_vals_sorted. state_machine_by_run_kernel then reads only those 4-byte
// values (NOT the 8-byte sorted keys), keeping its per-event memory traffic
// half the size — this is what makes the SortKeys-vs-SortPairs swap a net win.
//
// Earlier attempt (state_machine reading the 64-bit keys directly) regressed
// by 13 ms because state_machine is bandwidth-bound and dominates GPU time.
//
// Bit widths:
#define KIND_W_BIT          0u
#define ORIG_POS_SHIFT      1u
#define ORIG_POS_BITS       21u
#define ORIG_POS_MASK       ((1u << ORIG_POS_BITS) - 1u)
#define COMPACT_ADDR_SHIFT  (ORIG_POS_SHIFT + ORIG_POS_BITS)   // 22
#define RAM_KEY_END_BIT     48

// [Step 5] One thread per potential slot. Splits the d_potentials stream:
//   - non-RAM slots: emit_bit set directly to 1 (always emit), no further
//     processing.
//   - RAM slots: the slot is atomically compacted into d_ram_keys/d_ram_vals
//     and emit_bit defaults to 0 (will be overwritten by step 10's state
//     machine, which decides whether the slot survives the duality collapse).
//
// Every slot in [0, n_potentials) is guaranteed populated by the previous
// phase (decode_emit + blockop_emit cover the range exactly), so there's no
// "is this a real slot?" check.
//
// Input:  d_potentials[0..n_potentials)          — populated by Steps 3 + 4
// Input:  n_potentials                           — exact upper bound (CPU pre-computed)
// Output: d_ram_keys[0..*d_ram_count)            — packed (compact_addr, orig_pos, kind_w)
// In/Out: d_ram_count                            — atomic compaction counter
// Output: d_emit_bits[0..n_potentials)           — 1 for non-RAM emits,
//                                                   0 for RAM (set later by
//                                                   state_machine_by_run_kernel)
__global__
void gather_ram_events_kernel(const PotentialEmit* __restrict__ d_potentials,
                              uint32_t n_potentials,
                              uint64_t* __restrict__ d_ram_keys,
                              uint32_t* __restrict__ d_ram_count,
                              uint32_t* __restrict__ d_emit_bits) {
    const uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n_potentials) return;
    PotentialEmit p = d_potentials[i];

    if (emit_is_ram(p)) {
        const uint32_t compact = ram_compact(emit_aligned_addr(p));
        const uint64_t key = ((uint64_t)compact << COMPACT_ADDR_SHIFT)
                           | ((uint64_t)i      << ORIG_POS_SHIFT)
                           | (emit_kind_w(p) ? 1ull : 0ull);
        const uint32_t slot = atomicAdd(d_ram_count, 1u);
        d_ram_keys[slot] = key;
        d_emit_bits[i] = 0;  // overwritten by state_machine_by_run_kernel
    } else {
        d_emit_bits[i] = 1;  // non-RAM always emits
    }
}

// [Step 8] Extract the compact_addr high-bits from each sorted key into a
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
    d_sorted_addr[i] = (uint32_t)(d_sorted_keys[i] >> COMPACT_ADDR_SHIFT);
}

// [Step 8b] Extract a small per-event packed (kind_w, orig_pos) value into a
// 32-bit array, so state_machine_by_run_kernel can read 4 bytes per event
// instead of the 8-byte sorted key.
//
// Input:  d_sorted_keys[0..n_events)             — sorted 64-bit keys
// Input:  n_events                               — n_ram for this chunk
// Output: d_sorted_packed[0..n_events)           — (kind_w << 31) | orig_pos
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

// [Step 10] Per-segment R/W state machine. One thread per unique RAM address
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
        const bool kind_w    = (v >> 31);
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

// [Step 12] Compaction: one thread per potential slot. For each slot that
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
    d_out[d_final_offsets[i]] = emit_aligned_addr(d_potentials[i]);
}

#endif  // MEM_PREPROCESS_CUH