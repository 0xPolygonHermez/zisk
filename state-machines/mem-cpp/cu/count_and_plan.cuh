#ifndef COUNT_AND_PLAN_CUH
#define COUNT_AND_PLAN_CUH

// =====================================================================
// 
//                              CountAndPlan 
// 
// =====================================================================

#include <cstdint>
#include <cstddef>
#include <cstdio>
#include <cuda_runtime.h>
#include <string>
#include <vector>
#include <atomic>
#include <condition_variable>
#include <deque>
#include <mutex>
#include <thread>

// `InstanceMeta` is declared in this shared header so the plain C++ side
// (mem_count_and_plan.cpp) can read GPU-produced metas directly without
// pulling in any CUDA-only types.
#include "../cpp/instance_meta.hpp"

// ─── Public input type ──────────────────────────────────────────────

// One memory-ops record. `flags` holds the mode in bits 0-5; for non-block modes bits 6-25 hold
// `(step_in_chunk << 2) | slot`, for block modes bits 4-31 hold the word count and the step field
// sits in `payload` at the same position it has in a non-block header (bits 38-57). For non-block
// writes `payload` is the written value.
struct __align__(8) MemOp {
    uint32_t addr;
    uint32_t flags;
    uint64_t payload;
};
static_assert(sizeof(MemOp) == 16, "MemOp is one decoded record");

// Stream layout: cpp/mem_config.hpp. `load_record` decodes the record at a word position into a
// `MemOp`.

// ─── Internal types
struct PotentialEmit;
struct BlockOpSpill;

// Per-chunk mem-align counters. POD; same five u32 fields the CPU planner's
// `MemAlignCounters` uses (without the chunk_id — index in the per-chunk
// array is the chunk_id). Exposed in the header so the C ABI shim can
// hand a typed pointer back to Rust without dereferencing it.
struct ChunkCounters {
    uint32_t full_5;
    uint32_t full_3;
    uint32_t full_2;
    uint32_t read_byte;
    uint32_t write_byte;
};

// ZisK memory map (core/src/mem.rs): input 1 GB, ROM 128 MB, RAM 512 MB.
constexpr uint32_t ZISK_INPUT_ADDR_BASE   = 0x40000000u;
constexpr uint32_t ZISK_INPUT_SIZE_BYTES  = 1u << 30;
constexpr uint32_t ZISK_INPUT_ADDR_END    = ZISK_INPUT_ADDR_BASE + ZISK_INPUT_SIZE_BYTES;
constexpr uint32_t ZISK_ROM_ADDR_BASE     = 0x80000000u;
constexpr uint32_t ZISK_ROM_SIZE_BYTES    = 1u << 27;
constexpr uint32_t ZISK_ROM_ADDR_END      = ZISK_ROM_ADDR_BASE + ZISK_ROM_SIZE_BYTES;
constexpr uint32_t ZISK_RAM_ADDR_BASE     = 0xA0000000u;
constexpr uint32_t ZISK_RAM_SIZE_BYTES    = 1u << 29;
constexpr uint32_t ZISK_RAM_ADDR_END      = ZISK_RAM_ADDR_BASE + ZISK_RAM_SIZE_BYTES;
constexpr uint32_t ZISK_ALIGN_MASK        = 0xFFFFFFF8u;

// Compact address space: one entry per 8-byte word of each area, ROM then input then RAM.
constexpr uint32_t N_ADDR_ROM   = ZISK_ROM_SIZE_BYTES >> 3;    // 2^24
constexpr uint32_t N_ADDR_INPUT = ZISK_INPUT_SIZE_BYTES >> 3;  // 2^27
constexpr uint32_t N_ADDR_RAM   = ZISK_RAM_SIZE_BYTES >> 3;    // 2^26
constexpr uint32_t N_ADDR = N_ADDR_ROM + N_ADDR_INPUT + N_ADDR_RAM;
constexpr uint32_t REGION_ADDR_START[3] = {0, N_ADDR_ROM, N_ADDR_ROM + N_ADDR_INPUT};

// Meta word of a retained RAM access: bits 0-39 mem step, 40-41 kind (0 read, 1 full write,
// 2 partial write; 3, a write without its value, declines the device witness for the block),
// 42-44 byte offset and 45-48 byte width of a partial write.
constexpr uint32_t RAM_META_KIND_SHIFT  = 40;
constexpr uint32_t RAM_META_OFF_SHIFT   = 42;
constexpr uint32_t RAM_META_WIDTH_SHIFT = 45;
constexpr uint64_t RAM_META_STEP_MASK   = (1ull << 40) - 1;
// Retained RAM accesses: 20-byte records (address, meta, value) growing down from the arena top,
// record k at top - 20 (k + 1). Each chunk holds one contiguous run of records in (address,
// arrival) order; chunks follow their reservation order. The ops pool grows up from the fixed
// regions and the reservations keep the two from crossing. Records are 4-byte aligned, so the
// 64-bit fields are word pairs.
constexpr size_t RAM_RECORD_WORDS = 5;
struct RamRecords {
    uint32_t* top = nullptr;  // one past the highest record
    __host__ __device__ __forceinline__ uint32_t* rec(size_t k) const { return top - RAM_RECORD_WORDS * (k + 1); }
    __device__ __forceinline__ uint32_t addr(size_t k) const { return rec(k)[0]; }
    __device__ __forceinline__ uint64_t meta(size_t k) const {
        const uint32_t* r = rec(k); return r[1] | ((uint64_t)r[2] << 32);
    }
    __device__ __forceinline__ uint64_t value(size_t k) const {
        const uint32_t* r = rec(k); return r[3] | ((uint64_t)r[4] << 32);
    }
    __device__ __forceinline__ void store(size_t k, uint32_t a, uint64_t m, uint64_t v) const {
        uint32_t* r = rec(k);
        r[0] = a; r[1] = (uint32_t)m; r[2] = (uint32_t)(m >> 32); r[3] = (uint32_t)v; r[4] = (uint32_t)(v >> 32);
    }
};
// A MemAlign access (unaligned, or one byte wide), retained in arrival order in its own region at
// the arena top. The read potentials of the access carry the index of its old-word slots, which
// the memory fills scatter the resolved words into.
struct AlignRecord {
    uint32_t addr;     // byte address
    uint32_t chunk;
    uint64_t info;     // mem step (40) | kind << 40 (3) | width << 43 (4) | wr << 47
    uint64_t value;    // the written value as the stream carries it; 0 for a read
    uint64_t old[2];   // the words the access found, filled in by the fills
};
constexpr uint32_t ALIGN_KIND_SHIFT = 40, ALIGN_WIDTH_SHIFT = 43, ALIGN_WR_SHIFT = 47;
constexpr uint32_t ALIGN_KINDS = 5;          // full_5, full_3, full_2, read_byte, write_byte
constexpr uint32_t ALIGN_KIND_NONE = ALIGN_KINDS;
constexpr uint32_t ALIGN_RUN_NONE = 0xFFFFFFFFu;
constexpr uint32_t MAX_ALIGN_RUNS = 1u << 19;  // one per piece
struct AlignRun { uint32_t base; uint32_t n; };
// One MemAlign instance of the host plan: `air_kind` 0 full (MemAlign, MemAlignLarge), 1 byte
// (MemAlignByte, MemAlignByteLarge), 2 read byte, 3 write byte; its chunks are
// entries[entry_from, entry_from + entry_n). POD, mirrored in gpu_bindings.rs.
struct AlignPlanDesc { uint32_t air_kind, air_id, segment, n_rows, entry_from, entry_n; };
// Per chunk of an instance: of the chunk's accesses of each kind, in arrival order, skip the
// first `skip` and take the next `count` (the CPU collector's counters). POD, mirrored in Rust.
struct AlignChunkEntry { uint32_t chunk; uint32_t skip[ALIGN_KINDS]; uint32_t count[ALIGN_KINDS]; };
static_assert(sizeof(AlignPlanDesc) == 24 && sizeof(AlignChunkEntry) == 44, "align plan layout changed: update gpu_bindings.rs");

// One memory instance to fill into a prover slot: the kernel input the witness stages. POD,
// mirrored in gpu_bindings.rs. `family`: 0 Mem, 1 RomData, 2 InputData, 3 MemAlign.
struct MemSlotOp { uint32_t family, air_id, segment, n_rows; };
static_assert(sizeof(MemSlotOp) == 16, "MemSlotOp layout changed: update gpu_bindings.rs");

// Word index of the first and last RAM addresses.
constexpr uint32_t RAM_W_ADDR_BASE = ZISK_RAM_ADDR_BASE >> 3;
constexpr uint32_t RAM_W_ADDR_LAST = (ZISK_RAM_ADDR_END - 8u) >> 3;

// What prepare_ram_fill established for the block. POD, mirrored in gpu_bindings.rs.
struct RamFillPrepared {
    int32_t  status;             // 0 ok
    uint32_t n_instances;        // RAM instances (lanes / instance_rows, rounded up)
    uint64_t n_accesses;         // retained RAM accesses
    uint64_t n_lanes;            // lanes after dual pairing
    float    ms_sort, ms_lanes, ms_values, ms_total;
};
static_assert(sizeof(RamFillPrepared) == 40, "RamFillPrepared layout changed: update gpu_bindings.rs");

// One filled RAM instance. POD, mirrored in gpu_bindings.rs.
struct RamFillResult {
    int32_t  status;
    uint32_t n_lanes;            // lanes the instance's accesses occupy (before padding)
    uint32_t prev_addr_w;        // last lane of the previous instance (continuation)
    uint32_t last_addr_w;        // last lane of this instance
    uint64_t prev_step;
    uint64_t prev_value;
    uint64_t last_step;
    uint64_t last_value;
    float    ms_rows, ms_d2h;
};
static_assert(sizeof(RamFillResult) == 56, "RamFillResult layout changed: update gpu_bindings.rs");

// ─── Sizing constants visible to callers and to class-array bounds ──
//     to be revised...
constexpr int      N_STREAMS             = 4;
constexpr uint32_t MAX_INSTANCES         = MEM_GPU_MAX_INSTANCES;
constexpr uint32_t MASK_WORDS            = (MAX_INSTANCES + 31) / 32;
// MUST stay <= the C++ consumer cap MAX_CHUNKS (mem_config.hpp)
constexpr uint32_t MAX_CHUNKS            = MEM_GPU_MAX_META_CHUNKS; // 16384
// A chunk is processed in pieces of whole records, each with at most MAX_POT_PER_PIECE potentials
// and MAX_WORDS_PER_PIECE stream words, so the per-stream buffers are sized for one piece and a
// chunk of any size fits. The RAM pairing state crosses pieces as one bit per address, which
// enters the next piece's sort as a carry entry (MAX_SORT_PER_PIECE).
constexpr uint32_t MAX_POT_PER_PIECE   = 1u << 20;
constexpr uint32_t MAX_WORDS_PER_PIECE = MAX_POT_PER_PIECE + 128;
constexpr uint32_t MAX_SORT_PER_PIECE  = 2 * MAX_POT_PER_PIECE;

// Internal compile-time toggle for the add_chunk worker pool 
#define ZISK_MOPS_POOL 1

class CountAndPlan {
public:
    CountAndPlan();
    ~CountAndPlan();

    // ─── Public API ───────────────────────────────────────────────────

    // Initialize / reset prior state.
    //   d_buf == nullptr OR bytes == 0 → class allocates internally.
    //   d_buf != nullptr && bytes > 0  → caller-owned; if too small for
    //                                     the fixed regions, returns false
    //                                     (with stderr) and does not alloc.
    // n_workers ≥ 1 splits the MAX_INSTANCES space into n_workers slices via
    // (gid % n_workers); worker_id ∈ [0, n_workers) selects this instance's slice.
    // Each CountAndPlan object computes metas for its slice only.
    // gpu_id: device the buffer lives on (proofman's my_gpu_ids[0]); negative
    // keeps the current device (self-allocated path).
    // instance_rows[3]: rows per instance for {ROM, INPUT, RAM}, taken from
    // the PIL trace sizes (RomDataTrace / InputDataTrace / MemTrace
    // NUM_ROWS). Each must be a non-zero power of two.
    // `retain_rows`: keep the accesses and MemAlign records the device witness needs; off, the
    // planner only counts and plans.
    bool setup(void* d_buf, size_t bytes,
               uint32_t n_workers, uint32_t worker_id,
               int gpu_id, const uint32_t instance_rows[3], bool retain_rows);

    // Submit one chunk's memops.
    // Submit one chunk's memory-ops stream: `n_words` tagged 8-byte words.
    bool add_chunk(const uint64_t* words, uint32_t n_words);

    // Drains the per-chunk preprocessing streams and generated instances metadata 
    // On success: *metas_out = internal pointer (== metas_data()),
    //             n_metas   = num_active_instances(),
    //             returns true.
    // On failure: returns false, outputs unspecified.
    //
    // Lifetime: *metas_out is valid until the next reset() on this
    // CountAndPlan instance 
    bool run(InstanceMeta** metas_out, uint32_t& n_metas);

    // Reset for the next block.
    void reset();

    // ─── RAM witness from the retained accesses (after run()) ─────────
    // Every RAM access of the block is retained as it arrives (compact address, mem step, kind,
    // value). After the plan is closed, `prepare_ram_fill` sorts them by (address, time), pairs
    // dual lanes and resolves every read's value from the writes before it; `fill_ram_instance`
    // then packs one Mem instance's rows exactly as the CPU fill does.
    void set_chunk_size_bits(uint32_t bits) { chunk_size_bits_ = bits; }
    bool set_mem_layout(const uint32_t* col_widths, uint32_t n_cols, uint32_t words_per_row,
                        uint32_t lanes_x_row);
    bool ram_retention_ok() const { return ram_retention_enabled_.load(std::memory_order_relaxed); }
    size_t ram_accesses() const { return ram_cursor_.load(std::memory_order_relaxed); }
    bool prepare_ram_fill(RamFillPrepared* out);
    bool fill_ram_instance(uint32_t inst, uint64_t* out_rows, uint32_t n_rows, RamFillResult* res,
                           uint64_t* d_out = nullptr);
    // Packs every RAM instance into pinned host memory while the arena is still borrowed; the
    // witness phase then copies rows out with `ram_instance_rows`. Prepares if needed.
    // `insts` (n_insts of them; null = all): the instances this process owns; the others are
    // not filled and have no rows.
    bool fill_all_ram_instances(uint32_t n_rows, const uint32_t* insts, uint32_t n_insts, RamFillPrepared* prepared);
    const uint64_t* ram_instance_rows(uint32_t inst, RamFillResult* res) const;

    // ─── RomData witness from the retained ROM accesses (after the RAM fill) ─────────
    // ROM and input accesses are retained as they arrive, in arrival order. `fill_all_rom_instances`
    // sorts each RomData instance's accesses by (address, step), resolves every read from the init
    // write of its word and packs the rows as the CPU fill does.
    bool set_rom_layout(const uint32_t* col_widths, uint32_t n_cols, uint32_t words_per_row,
                        uint32_t lanes_x_row);
    bool fill_all_rom_instances(uint32_t n_rows, const uint32_t* insts, uint32_t n_insts, RamFillPrepared* prepared);
    const uint64_t* rom_instance_rows(uint32_t inst, RamFillResult* res) const;

    // ─── InputData witness from the retained input accesses (after the RomData fill) ─────────
    // `image` is the input region as the guest sees it (the input shared memory from its start,
    // `image_bytes` of it); every read resolves to its word, the free-input word to its record.
    bool set_input_layout(const uint32_t* col_widths, uint32_t n_cols, uint32_t words_per_row,
                          uint32_t lanes_x_row);
    bool fill_all_input_instances(uint32_t n_rows, const void* image, size_t image_bytes, const uint32_t* insts,
                                  uint32_t n_insts, RamFillPrepared* prepared);
    const uint64_t* input_instance_rows(uint32_t inst, RamFillResult* res) const;

    // ─── MemAlign witness from the retained align accesses (after the three memory fills) ─────
    // The fills leave every access's old words in its record; each instance of the host plan then
    // selects its accesses (per chunk, per kind, skip/count), builds its rows in arrival order and
    // packs them into pinned memory, served by (air id, segment).
    bool set_align_layout(uint32_t air_kind, const uint32_t* col_widths, uint32_t n_cols, uint32_t words_per_row);
    bool fill_all_align_instances(const AlignPlanDesc* plans, uint32_t n_plans, const AlignChunkEntry* entries,
                                  uint32_t n_entries, RamFillPrepared* prepared);
    const uint64_t* align_instance_rows(uint32_t air_id, uint32_t segment, RamFillResult* res) const;

    // ─── Memory instances filled into the prover's slots (ZISK_MEM_GPU_FILL=slot) ─────────
    // `prepare_slot_fills` does the block's work once: RAM tables, the resolve pass over every
    // instance (its scalars, and the old words the MemAlign records need), the ROM/input index and
    // image, the MemAlign access order and the owned MemAlign plans. `fill_slot` then builds one
    // instance's rows straight into `dst` (device) from the staged `MemSlotOp`; the arena must still
    // be borrowed. `instance_scalars` serves the air values of a resolved instance without rows.
    bool prepare_slot_fills(const void* image, size_t image_bytes, const AlignPlanDesc* plans, uint32_t n_plans,
                            const AlignChunkEntry* entries, uint32_t n_entries, RamFillPrepared* prepared);
    bool fill_slot(const void* d_ops, uint64_t n_ops, uint64_t* dst, void* stream, RamFillResult* res);
    // Waits for the slot copies still in flight on the prover's streams: before the arena is released.
    void slot_quiesce();
    // The device the planner and the fills run on (`setup`'s gpu_id).
    int device() const { return gpu_device_; }
    bool instance_scalars(uint32_t family, uint32_t inst, RamFillResult* res) const;


    bool register_input_pinned(void* ptr, size_t bytes);
    void unregister_input_pinned(void* ptr);

    // ─── Read-only state ──────────────────────────────────────────────

    const InstanceMeta* metas_data()             const { return metas_.data(); }
    uint32_t            num_active_instances()   const { return num_active_; }

    // Arena usage of the CURRENT block in BYTES: fixed carve end + ops-pool cursor.
    // Valid between run() and the next reset() (reset zeroes the pool cursor). 
    size_t max_used_bytes() const {
        const size_t used = cursor_ + pool_cursor_u32_.load(std::memory_order_relaxed) * 4;
        // The retained accesses sit at the top of the arena: any retention touches its end.
        if (ram_cursor_.load(std::memory_order_relaxed) > 0) return arena_bytes_;
        return used;
    }

    // Per-chunk mem-align counters, valid after `run()`. Length == n_chunks().
    const ChunkCounters* align_counters_data()   const { return h_chunk_counters_per_chunk_; }
    uint32_t             n_chunks()              const { return n_chunks_; }

private:

    // ─── Single device buffer + slicing cursor ────────────────────────

    uint8_t* arena_       = nullptr;
    size_t   arena_bytes_ = 0;
    bool     arena_owned_ = false;
    size_t   cursor_      = 0;

    // ─── Globals (one of each, lifetime spans the whole block) ────────

    uint32_t*      d_histogram_              = nullptr;
    uint32_t*      d_prefix_                 = nullptr;
    void*          d_temp_hist_              = nullptr;
    size_t         d_temp_hist_bytes_        = 0;
    uint32_t*      d_max_compact_              = nullptr;
    uint32_t*      d_invalid_mode_flag_        = nullptr;
    ChunkCounters* d_chunk_counters_per_chunk_ = nullptr;  // device, MAX_CHUNKS slots
    uint32_t*      d_gappy_offsets_          = nullptr;
    uint32_t*      d_chunk_lens_             = nullptr;
    uint32_t*      d_packed_chunk_offsets_   = nullptr;
    uint32_t*      d_active_ids_             = nullptr;
    uint32_t*      d_active_first_           = nullptr;
    uint32_t*      d_active_last_            = nullptr;
    uint32_t*      d_fml_                    = nullptr;
    uint32_t*      d_result_nops_            = nullptr;
    uint32_t*      d_meta_scalars_           = nullptr;
    uint32_t*      d_inst_base_pos_          = nullptr;
    uint32_t*      d_page_starts_            = nullptr;
    uint32_t*      d_page_single_            = nullptr;
    uint32_t*      d_pages_dense_            = nullptr;
    uint32_t*      d_present_counters_       = nullptr;
    uint32_t*      d_page_meta_starts_       = nullptr;
    uint32_t*      d_pages_dense_starts_     = nullptr;
    uint32_t*      h_present_counters_       = nullptr;

    // ─── Per-stream device buffers (parallel arrays) ──────────────────

    cudaStream_t   streams_[N_STREAMS]             = {nullptr};
    uint64_t*      d_words_[N_STREAMS]             = {nullptr};   // the chunk's stream
    uint32_t*      d_tag_flags_[N_STREAMS]         = {nullptr};   // header flag per word, then its scan
    uint32_t*      d_rec_start_[N_STREAMS]         = {nullptr};   // record -> first word
    uint32_t*      d_n_records_[N_STREAMS]         = {nullptr};
    uint32_t*      d_counts_[N_STREAMS]            = {nullptr};
    uint32_t*      d_potential_offsets_[N_STREAMS] = {nullptr};
    PotentialEmit* d_potentials_[N_STREAMS]        = {nullptr};
    uint32_t*      d_emit_bits_[N_STREAMS]         = {nullptr};
    uint32_t*      d_final_offsets_[N_STREAMS]     = {nullptr};
    uint64_t*      d_ram_keys_[N_STREAMS]          = {nullptr};
    uint64_t*      d_ram_keys_sorted_[N_STREAMS]   = {nullptr};
    uint32_t*      d_ram_vals_sorted_[N_STREAMS]   = {nullptr};
    uint32_t*      d_ram_count_[N_STREAMS]         = {nullptr};
    BlockOpSpill*  d_spill_[N_STREAMS]             = {nullptr};
    uint32_t*      d_spill_count_[N_STREAMS]       = {nullptr};
    uint8_t*       d_spill_status_[N_STREAMS]      = {nullptr};
    uint32_t*      d_sorted_addr_[N_STREAMS]       = {nullptr};
    uint32_t*      d_run_lengths_[N_STREAMS]       = {nullptr};
    uint32_t*      d_run_offsets_[N_STREAMS]       = {nullptr};
    uint32_t*      d_num_unique_[N_STREAMS]        = {nullptr};
    void*          d_cub_temp_[N_STREAMS]          = {nullptr};
    size_t         cub_temp_bytes_                 = 0;
    uint32_t*      h_n_emits_[N_STREAMS]           = {nullptr};
    uint64_t*      d_carry_[N_STREAMS]             = {nullptr};   // pairing state per address, as sort keys
    uint32_t*      d_carry_rank_[N_STREAMS]        = {nullptr};   // sorted position -> real entries before it
    uint32_t*      h_n_carry_[N_STREAMS]           = {nullptr};

    // ─── Ops pool (bump-allocated by add_chunk) ──────────────────────

    uint32_t* d_ops_pool_         = nullptr;
    size_t    d_ops_pool_cap_u32_ = 0;
    size_t    d_ops_pool_used_u32_= 0;

    // Retained RAM accesses, in arrival order: compact word address, packed step/kind, value.
    // Device resident, carved from the top of the arena at setup (the pool grows towards them);
    // they survive run() and feed the post-plan phase, whose scratch is everything below them.
    RamRecords         ram_records_;
    size_t             top_bytes_    = 0;   // arena offset of ram_records_.top
    std::atomic<size_t> ram_cursor_{0};
    // Byte bounds of the two stacks of the dynamic region, used by the reservations.
    size_t pool_end_bytes(size_t pool_words) const { return cursor_ + pool_words * 4; }
    size_t ram_low_edge_bytes(size_t records) const { return top_bytes_ - records * RAM_RECORD_WORDS * 4; }
    std::atomic<bool>   ram_retention_enabled_{false};
    struct RamRun { uint32_t base; uint32_t n; };               // one piece's records
    std::vector<RamRun> ram_runs_;
    std::mutex          ram_runs_mtx_;
    uint32_t            piece_potentials_ = MAX_POT_PER_PIECE;  // cut threshold (ZISK_MOPS_PIECE_POTENTIALS lowers it for tests)
    std::atomic<uint32_t> max_pieces_{0};
    unsigned long long* d_ram_nwrites_ = nullptr;             // writes among the retained accesses
    uint64_t           ram_writes_     = 0;                   // read back by prepare_ram_fill
    uint32_t           chunk_size_bits_ = 18;
    // Prepared lane table (see ram_fill.cu).
    bool               ram_prepared_    = false;    // every instance filled; totals valid
    bool               ram_tables_ready_ = false;   // per-instance fill tables built
    uint32_t*          d_rf_chunk_base_ = nullptr;  // chunk -> first record index
    uint32_t*          d_rf_chunk_n_    = nullptr;  // chunk -> record count
    uint32_t*          d_rf_inst_ids_   = nullptr;
    uint32_t*          d_rf_inst_first_ = nullptr;  // instance -> first compact RAM word
    uint32_t*          d_rf_inst_last_  = nullptr;
    uint32_t*          d_rf_bound_      = nullptr;  // (chunk, instance) -> rank where the range starts
    uint32_t*          d_rf_pref_       = nullptr;  // (instance, chunk) -> exclusive prefix of slice sizes
    uint8_t*           rf_scratch_      = nullptr;  // per-instance scratch starts here
    size_t             rf_scratch_peak_ = 0;
    std::vector<size_t> h_rf_inst_count_;           // instance -> accesses of its address range
    std::vector<size_t> h_rf_inst_skip_;            // instance -> lanes of the range before its window
    std::vector<size_t> h_rf_inst_lanes_;           // instance -> lanes of its whole address range
    float              ram_ms_[4]       = {0, 0, 0, 0};  // sort, lanes, values, total over the instances
    size_t             ram_n_lanes_     = 0;
    uint64_t*                  h_ram_rows_       = nullptr;   // pinned, n_instances x rows x words
    size_t                     h_ram_rows_cap_   = 0;         // u64 words
    std::thread                h_ram_rows_prealloc_;          // allocates the default capacity at setup
    void join_rows_prealloc_() { if (h_ram_rows_prealloc_.joinable()) h_ram_rows_prealloc_.join(); }
    size_t                     ram_rows_stride_  = 0;         // u64 words per instance
    std::vector<RamFillResult> ram_results_;

    // Retained ROM and input accesses: records of the same stack, one run per piece in arrival order.
    std::vector<RamRun>        other_runs_;
    uint32_t*                  d_other_rank_[N_STREAMS] = {nullptr};
    // RomData fill (see ram_fill.cu).
    uint32_t                   rom_col_widths_[64] = {0};
    uint32_t                   rom_n_cols_ = 0, rom_words_per_row_ = 0, rom_lanes_x_row_ = 0;
    bool                       rom_prepared_ = false;
    size_t                     other_total_ = 0;            // retained ROM and input accesses
    uint32_t*                  d_other_idx_ = nullptr;      // other access -> record index
    uint32_t*                  d_other_addr_ = nullptr;     // other access -> compact address
    uint8_t*                   other_scratch_ = nullptr;    // per-instance scratch starts here
    std::vector<size_t>        h_rom_inst_skip_, h_rom_inst_count_, h_rom_inst_extra_;
    std::vector<uint32_t>      h_rom_inst_first_, h_rom_inst_last_, h_rom_inst_ext_first_;
    uint64_t*                  h_rom_rows_ = nullptr;       // pinned, n_instances x rows x words
    size_t                     h_rom_rows_cap_ = 0;
    size_t                     rom_rows_stride_ = 0;
    std::vector<RamFillResult> rom_results_;
    bool prepare_other_index_();
    bool other_geometry_(int region, std::vector<uint32_t>& first, std::vector<uint32_t>& last,
                         std::vector<size_t>& skip, std::vector<size_t>& count,
                         std::vector<uint32_t>& ext_first, std::vector<size_t>& extra);
    bool other_sorted_(struct ScratchCursor& sc, uint32_t first, uint32_t last, size_t expect,
                       uint32_t** addr_sorted, uint32_t** idx, size_t* n_out);
    bool fill_rom_instance(uint32_t inst, uint64_t* out_rows, uint32_t n_rows, RamFillResult* res,
                           uint64_t* d_out = nullptr);
    // InputData fill (see ram_fill.cu).
    uint32_t                   input_col_widths_[64] = {0};
    uint32_t                   input_n_cols_ = 0, input_words_per_row_ = 0, input_lanes_x_row_ = 0;
    bool                       input_prepared_ = false;
    std::vector<size_t>        h_input_inst_skip_, h_input_inst_count_, h_input_inst_extra_;
    std::vector<uint32_t>      h_input_inst_first_, h_input_inst_last_, h_input_inst_ext_first_;
    uint64_t*                  h_input_rows_ = nullptr;     // pinned, n_instances x rows x words
    size_t                     h_input_rows_cap_ = 0;
    size_t                     input_rows_stride_ = 0;
    std::vector<RamFillResult> input_results_;
    bool fill_input_instance(uint32_t inst, const uint64_t* d_image, const uint64_t* h_image, size_t image_words,
                             uint8_t* scratch, uint64_t* out_rows, uint32_t n_rows, RamFillResult* res,
                             uint64_t* d_out = nullptr);
    // The input image on the device, and the per-instance scratch after it.
    uint64_t*                  d_image_ = nullptr;
    std::vector<uint64_t>      h_image_;
    size_t                     image_words_ = 0;
    uint8_t*                   input_scratch_ = nullptr;
    // Slot fills: every instance resolved, per-instance scratch after all the block's tables.
    bool                       slot_prepared_ = false;
    bool                       resolve_all_ = false;
    uint8_t*                   slot_scratch_ = nullptr;
    std::vector<AlignPlanDesc>   slot_align_plans_;
    std::vector<AlignChunkEntry> slot_align_entries_;
    // Row images the preparation built, staged below the retained accesses; `fill_slot` copies them
    // device to device. The per-instance scratch ends at `stage_low_` while any is staged.
    struct Staged { uint32_t family, air_id, segment, n_rows; uint64_t* ptr; size_t words; RamFillResult res; };
    std::vector<Staged>        staged_;
    uint8_t*                   stage_low_ = nullptr;
    bool                       stage_try_ = false;     // staged attempt: a scratch overflow is retried, not reported
    std::vector<cudaEvent_t>   slot_copy_events_;      // the copies in flight on the prover's streams
    uint8_t*  scratch_end_(size_t n_total) const;
    uint64_t* stage_take_(size_t words);
    template <class Fill>
    bool fill_staged_(uint32_t family, uint32_t air_id, uint32_t segment, uint32_t n_rows, size_t words,
                      RamFillResult* res, Fill&& fill);
    // MemAlign retention (count_and_plan.cu) and fill (ram_fill.cu).
    AlignRecord*               d_align_ = nullptr;           // region at the arena top, align_cap_ records
    size_t                     align_cap_ = 0;
    uint32_t*                  d_align_cursor_ = nullptr;    // records reserved (device atomic)
    uint32_t*                  d_align_overflow_ = nullptr;  // set when a reservation did not fit
    AlignRun*                  d_align_runs_ = nullptr;      // per piece slot
    std::atomic<uint32_t>      align_slot_{0};
    std::atomic<bool>          align_enabled_{false};
    struct AlignPieceRun { uint32_t chunk, slot; };
    std::vector<AlignPieceRun> align_runs_;                  // under ram_runs_mtx_
    uint8_t*                   d_align_kind_[N_STREAMS] = {nullptr};
    uint32_t*                  d_align_flag_[N_STREAMS] = {nullptr};
    uint32_t*                  d_align_rank_[N_STREAMS] = {nullptr};
    uint32_t                   align_col_widths_[4][64] = {{0}};
    uint32_t                   align_n_cols_[4] = {0}, align_words_per_row_[4] = {0};
    bool                       align_prepared_ = false;
    size_t                     align_total_ = 0;
    uint32_t*                  d_align_order_ = nullptr;     // access -> record index, in (chunk, arrival) order
    std::vector<size_t>        h_align_chunk_start_;         // first access of each chunk in that order
    uint8_t*                   align_scratch_ = nullptr;
    uint64_t*                  h_align_rows_ = nullptr;      // pinned
    size_t                     h_align_rows_cap_ = 0;
    struct AlignFilled { uint32_t air_id, segment; size_t offset; RamFillResult res; };
    std::vector<AlignFilled>   align_results_;
    bool prepare_align_index_();
    bool fill_align_instance(const AlignPlanDesc& plan, const AlignChunkEntry* entries, uint8_t* scratch,
                             uint64_t* out_rows, RamFillResult* res, uint64_t* d_out = nullptr);
    uint32_t           mem_col_widths_[64] = {0};
    uint32_t           mem_n_cols_ = 0, mem_words_per_row_ = 0, mem_lanes_x_row_ = 0;

    // ─── Pinned host buffers ─────────────────────────────────────────

    uint32_t*      h_n_emits_all_              = nullptr;
    // Pinned destinations for the compacted paged-offsets output.
    //   - h_page_starts_buf_ / h_page_single_buf_ are sized in pages
    //     (1 entry per page); cumulative across the active instances.
    //   - h_pages_dense_buf_ is sized in slots (MEM_OFFSETS_PAGE_SIZE per
    //     present page); bounded above by total_addrs (every page present).
    uint32_t*      h_page_starts_buf_          = nullptr;
    uint32_t*      h_page_single_buf_           = nullptr;
    size_t         h_page_meta_buf_size_       = 0;       // bytes (covers both)
    uint32_t*      h_pages_dense_buf_          = nullptr;
    size_t         h_pages_dense_buf_size_     = 0;
    uint32_t*      h_result_nops_              = nullptr;
    uint32_t*      h_meta_scalars_             = nullptr;
    ChunkCounters* h_chunk_counters_per_chunk_ = nullptr;  // pinned, MAX_CHUNKS slots

    // ─── Streams + events ────────────────────────────────────────────

    cudaStream_t  d2h_stream_         = nullptr;
    cudaStream_t  meta_stream_        = nullptr;
    cudaEvent_t   e_after_preproc_    = nullptr;
    cudaEvent_t   e_after_prepare_    = nullptr;
    cudaEvent_t   e_metas_ready_      = nullptr;

    // ─── Per-block state ────────────────────────────────────────────

    uint32_t              n_workers_            = 0;
    uint32_t              worker_id_            = 0;
    uint32_t              max_active_           = 0;
    uint32_t              n_chunks_             = 0;
    uint32_t              num_ops_              = 0;
    std::vector<size_t>   out_offsets_;
    std::vector<uint32_t> n_potentials_per_chunk_;
    std::vector<uint32_t> n_ram_per_chunk_;
    uint32_t              rf_n_runs_ = 0;
    std::vector<uint32_t> n_words_per_chunk_;
    std::vector<uint32_t> packed_chunk_offsets_h_;
    bool                  preprocessed_            = false;
    bool                  prepared_                = false;
    bool                  metas_ready_recorded_    = false;

    // ─── Worker state ───────────────────────────────────────────────

    uint32_t                  active_mask_[MASK_WORDS]           = {0};
    uint32_t                  h_active_local_ids_[MAX_INSTANCES] = {0};
    std::vector<uint32_t>     h_active_first_;
    std::vector<uint32_t>     h_active_last_;
    std::vector<InstanceMeta> metas_;
    uint32_t  num_inst_[3]         = {0};
    uint32_t  num_active_per_[3]   = {0};
    uint32_t  active_offset_[3]    = {0};
    uint32_t  region_n_ops_[3]     = {0};
    uint32_t  region_ops_start_[3] = {0};
    uint32_t  num_active_          = 0;
    uint32_t  num_instances_       = 0;
    uint32_t  h_max_compact_[3]    = {0};

    // ─── add_chunk concurrency (ZISK_MOPS_POOL) ───────────────────────
    int                     gpu_device_           = 0;     // captured in setup()
    bool                    retain_rows_          = false; // device witness requested, captured in setup()
    uint32_t                instance_rows_[3]     = {0, 0, 0};  // rows per instance {ROM, INPUT, RAM}, captured in setup()
    bool                    pool_enabled_         = false; // ZISK_MOPS_POOL

    struct ChunkJob { const uint64_t* words; uint32_t n; uint32_t c; };
    std::deque<ChunkJob>     pool_q_[N_STREAMS];
    std::mutex               pool_mtx_[N_STREAMS];
    std::condition_variable  pool_cv_[N_STREAMS];
    std::thread              pool_threads_[N_STREAMS];
    bool                     pool_should_stop_   = false;

    std::atomic<size_t>     pool_cursor_u32_{0};
    std::atomic<bool>       add_error_{false};

    

    // ─── Internal helpers

    void   free_all_();
    void   free_all_bound_();
    void   free_pinned_();
    void   query_cub_sizes_(size_t& scan_counts_b, size_t& scan_emit_b,
                            size_t& scan_runs_b,   size_t& sort_b,
                            size_t& rle_b,         size_t& hist_scan_b);
    void   prepare_global_();
    void   process_worker_();
    void   set_active_worker_();
    void   pick_active_instances_();

    bool   add_chunk_core_(const uint64_t* words, uint32_t n_words, uint32_t c);
    bool   add_piece_(const uint64_t* words, uint32_t n, uint32_t c, int s, uint32_t pot, uint32_t ram,
                      uint32_t* d_out, uint32_t n_carry, bool carry_out, uint32_t* h_emits);
    void   pool_start_();
    void   pool_stop_();
    void   pool_thread_loop_(int s);
};

// ─── Binary-meta save helpers ───────────────────────────────────────
//
// Three-call protocol per file: begin → append × N → end.
//   FILE* f = save_metas_begin(path);
//   for (i) save_metas_append(f, metas[i]);
//   save_metas_end(f, /*total=*/N);
//
// Wire-format version: paged v1 (incompatible with the previous dense
// `addr_offsets[]` and sparse-soa formats — keep `instance_meta_loader.hpp`
// in sync if you touch this).
//
// On-disk wire format (little-endian, all sizes in bytes unless noted):
//
//   uint32_t num_metas                       // header at offset 0
//
//   for each meta (no padding between records):
//     uint32_t inst_id
//     uint32_t kind                          // 0=ROM, 1=INPUT, 2=RAM
//     uint32_t first_addr
//     uint32_t last_addr
//     uint32_t first_addr_chunk
//     uint32_t first_addr_skip
//     uint32_t last_addr_chunk
//     uint32_t last_addr_include
//     uint32_t cps                           // = n_chunks
//     uint32_t np                            // = num_pages
//     uint32_t pc                            // = present_count
//     uint32_t ars                           // = addr_range_slots = (last_addr - first_addr)/8 + 1
//     uint32_t count_per_chunk[cps]          // per-chunk surviving emit counts
//     uint32_t page_starts[np]               // MEM_OFFSETS_PAGE_ABSENT or
//                                            //  page index into pages_dense
//     uint32_t page_single_value[np]         // value carried into each page
//     uint32_t pages_dense[pc * MEM_OFFSETS_PAGE_SIZE]
//                                            // present-page dense data
//
FILE* save_metas_begin (const std::string& path);
void  save_metas_append(FILE* f, const InstanceMeta& m);
void  save_metas_end   (FILE* f, uint32_t total);

#endif  // COUNT_AND_PLAN_CUH
