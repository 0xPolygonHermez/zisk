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

struct __align__(8) MemOp {
    uint32_t addr;
    uint32_t flags;
};

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

// ─── Sizing constants visible to callers and to class-array bounds ──
//     to be revised...
constexpr int      N_STREAMS             = 4;
constexpr uint32_t MAX_INSTANCES         = 1024;
constexpr uint32_t MASK_WORDS            = (MAX_INSTANCES + 31) / 32;
// MAX_CHUNKS MUST stay <= the C++ consumer cap MAX_CHUNKS 
constexpr uint32_t MAX_CHUNKS            = 1u << 13;          // 8192
constexpr uint32_t MAX_MEMOPS_PER_CHUNK  = 1u << 20;          // 1048576 (2 memops/step at CHUNK_SIZE=2^18 -> now 4/step; ~+0.9 GB GPU device mem vs 1<<19). Raising this forces ORIG_POS_BITS/RAM_KEY_END_BIT up (static_asserts in count_and_plan.cu)
constexpr uint32_t POTENTIAL_FACTOR      = 8;                 
constexpr uint32_t MAX_POT_PER_CHUNK     = MAX_MEMOPS_PER_CHUNK * POTENTIAL_FACTOR;
constexpr uint32_t MAX_TOTAL_MEMOPS      = 1u << 29;          // 512M ops

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
    bool setup(void* d_buf, size_t bytes,
               uint32_t n_workers, uint32_t worker_id);

    // Submit one chunk's memops.
    bool add_chunk(const MemOp* memops, uint32_t n);

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


    bool register_input_pinned(void* ptr, size_t bytes);
    void unregister_input_pinned(void* ptr);

    // ─── Read-only state ──────────────────────────────────────────────

    const InstanceMeta* metas_data()             const { return metas_.data(); }
    uint32_t            num_active_instances()   const { return num_active_; }

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
    uint32_t*      d_addr_offsets_           = nullptr;
    uint32_t*      d_offset_starts_          = nullptr;
    uint32_t*      d_page_starts_            = nullptr;
    uint32_t*      d_page_single_            = nullptr;
    uint32_t*      d_pages_dense_            = nullptr;
    uint32_t*      d_present_counters_       = nullptr;
    uint32_t*      d_page_meta_starts_       = nullptr;
    uint32_t*      d_pages_dense_starts_     = nullptr;
    uint32_t*      h_present_counters_       = nullptr;

    // ─── Per-stream device buffers (parallel arrays) ──────────────────

    cudaStream_t   streams_[N_STREAMS]             = {nullptr};
    MemOp*         d_memops_[N_STREAMS]            = {nullptr};
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

    // ─── Ops pool (bump-allocated by add_chunk) ──────────────────────

    uint32_t* d_ops_pool_         = nullptr;
    size_t    d_ops_pool_cap_u32_ = 0;
    size_t    d_ops_pool_used_u32_= 0;

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
    bool                    pool_enabled_         = false; // ZISK_MOPS_POOL

    struct ChunkJob { const MemOp* memops; uint32_t n; uint32_t c; };
    std::deque<ChunkJob>     pool_q_[N_STREAMS];
    std::mutex               pool_mtx_[N_STREAMS];
    std::condition_variable  pool_cv_[N_STREAMS];
    std::thread              pool_threads_[N_STREAMS];
    bool                     pool_should_stop_   = false;

    std::atomic<size_t>     pool_cursor_u32_{0};
    std::atomic<bool>       add_error_{false};

    

    // ─── Internal helpers

    void   free_all_();
    void   free_pinned_();
    void   query_cub_sizes_(size_t& scan_counts_b, size_t& scan_emit_b,
                            size_t& scan_runs_b,   size_t& sort_b,
                            size_t& rle_b,         size_t& hist_scan_b);
    void   prepare_global_();
    void   process_worker_();
    void   set_active_worker_();
    void   pick_active_instances_();

    bool   add_chunk_core_(const MemOp* memops, uint32_t n, uint32_t c);
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
