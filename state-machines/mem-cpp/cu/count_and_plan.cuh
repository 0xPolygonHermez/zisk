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

#include "../cpp/gpu_raw_instance_meta.hpp"

// ─── Public input type ──────────────────────────────────────────────

struct __align__(8) MemOp {
    uint32_t addr;
    uint32_t flags;
};

// ─── Public output type ─────────────────────────────────────────────
// Aliased to the shared layout in gpu_raw_instance_meta.hpp so the plain
// C++ side (mem_count_and_plan.cpp) can read these metas directly.
using InstanceMeta = RawInstanceMeta;

// ─── Internal types 
struct PotentialEmit;
struct BlockOpSpill;
struct ChunkCounters;

// ─── Sizing constants visible to callers and to class-array bounds ──
//     to be revised...
constexpr int      N_STREAMS             = 4;
constexpr uint32_t MAX_INSTANCES         = 1024;
constexpr uint32_t MASK_WORDS            = (MAX_INSTANCES + 31) / 32;
constexpr uint32_t MAX_CHUNKS            = 1u << 13;          // 8192
constexpr uint32_t MAX_MEMOPS_PER_CHUNK  = 1u << 18;          // 262144
constexpr uint32_t POTENTIAL_FACTOR      = 8;                 
constexpr uint32_t MAX_POT_PER_CHUNK     = MAX_MEMOPS_PER_CHUNK * POTENTIAL_FACTOR;
constexpr uint32_t MAX_TOTAL_MEMOPS      = 1u << 29;          // 512M ops 

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

    // ─── Read-only state ──────────────────────────────────────────────

    float               last_chunk_to_final_ms() const { return last_chunk_to_final_ms_; }
    const InstanceMeta* metas_data()             const { return metas_.data(); }
    uint32_t            num_active_instances()   const { return num_active_; }

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
    uint32_t*      d_max_compact_            = nullptr;
    uint32_t*      d_invalid_mode_flag_      = nullptr;
    ChunkCounters* d_chunk_counters_scratch_ = nullptr;
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

    MemOp*    h_memops_           = nullptr;
    size_t    h_memops_used_      = 0;
    uint32_t* h_n_emits_all_      = nullptr;
    uint32_t* h_offsets_buf_      = nullptr;
    size_t    h_offsets_buf_size_ = 0;
    uint32_t* h_result_nops_      = nullptr;
    uint32_t* h_meta_scalars_     = nullptr;

    // ─── Streams + events ────────────────────────────────────────────

    cudaStream_t  d2h_stream_         = nullptr;
    cudaStream_t  meta_stream_        = nullptr;
    cudaEvent_t   e_last_chunk_start_ = nullptr;
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
    float                 last_chunk_to_final_ms_  = 0.f;

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
};

// ─── Binary-meta save helpers ───────────────────────────────────────
//
// Three-call protocol per file: begin → append × N → end.
//   FILE* f = save_metas_begin(path);
//   for (i) save_metas_append(f, metas[i]);
//   save_metas_end(f, /*total=*/N);
//
// On-disk wire format (little-endian, all sizes in bytes unless noted):
//
//   uint32_t num_metas                    // header at offset 0
//
//   for each meta (no padding between records):
//     uint32_t inst_id
//     uint32_t kind                       // 0=ROM, 1=INPUT, 2=RAM
//     uint32_t first_addr
//     uint32_t last_addr
//     uint32_t first_addr_chunk
//     uint32_t first_addr_skip
//     uint32_t last_addr_chunk
//     uint32_t last_addr_include
//     uint32_t cps                        // = n_chunks
//     uint32_t aos                        // = addr_offsets_size
//     uint32_t count_per_chunk[cps]       // n_chunks entries (total per-chunk
//                                          //  surviving emits in this instance)
//     uint32_t addr_offsets[aos]          // num_addrs entries (cumulative
//                                          //  write offset per 8-byte slot in
//                                          //  [first_addr, last_addr])
//
FILE* save_metas_begin (const std::string& path);
void  save_metas_append(FILE* f, const InstanceMeta& m);
void  save_metas_end   (FILE* f, uint32_t total);

#endif  // COUNT_AND_PLAN_CUH
