#ifndef COUNT_AND_PLAN_CUH
#define COUNT_AND_PLAN_CUH

// =====================================================================
// CountAndPlan 
//
// Lifecycle (per block):
//     CountAndPlan p;
//     p.setup(buf, bytes, max_chunks, max_memops_per_chunk);
//     for (chunks)  p.add_chunk(memops, n);
//     p.run(0, metas_out, metas_out_bytes);   // worker 0 = production path
//     p.saveMetas("path");                    // optional persistence
//     p.reset();                              // optional, for next block
// =====================================================================

#include <cstdint>
#include <cstddef>
#include <cstdio>
#include <cuda_runtime.h>
#include <span>
#include <string>
#include <vector>

// ─── Public input type ──────────────────────────────────────────────
// One memop record: { raw 32-bit address, encoded flags }. Caller
// passes an array of these to add_chunk().
struct __align__(8) MemOp {
    uint32_t addr;
    uint32_t flags;
};

// ─── Public output type ─────────────────────────────────────────────
// One InstanceMeta per active instance produced by run(). Iterate
// via metas_data() / num_active_instances(); the spans point into
// pinned-host buffers owned by CountAndPlan.
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

// ─── Internal types 
struct PotentialEmit;
struct BlockOpSpill;
struct ChunkCounters;

// ─── Sizing constants visible to callers and to class-array bounds ──
constexpr int      N_STREAMS         = 4;
constexpr uint32_t N_WORKERS         = 16; //treure
constexpr uint32_t MAX_INST_ROM      = 32;
constexpr uint32_t MAX_INST_INPUT    = 64;
constexpr uint32_t MAX_INST_RAM      = 512;
constexpr uint32_t MAX_INSTANCES     = MAX_INST_ROM + MAX_INST_INPUT + MAX_INST_RAM;
constexpr uint32_t MASK_WORDS        = (MAX_INSTANCES + 31) / 32;
constexpr uint32_t MAX_ACTIVE        = (MAX_INSTANCES + N_WORKERS - 1) / N_WORKERS;
constexpr uint32_t MAX_CHUNKS        = 2<<15;

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
    bool setup(void* d_buf, size_t bytes,
               uint32_t max_chunks, uint32_t max_memops_per_chunk);

    // Submit one chunk's memops. cudaMemcpyAsync into the per-stream
    // reception region, then enqueue preprocessing kernels on that
    // chunk's assigned stream (round-robin across N_STREAMS).
    // FATAL crash if `n > max_memops_per_chunk`. Returns false if
    // max_chunks would be exceeded or the ops pool is full.
    bool add_chunk(const MemOp* memops, uint32_t n);

    // Block on chunks (first call only), prepare_global (cached), process
    // worker_id, populate metas_, serialize to caller buffer.
    //   metas_out_bytes == 0 → don't serialize; metas accessible via
    //                           metas_data() / saveMetas().
    //   too small            → returns 0 with stderr.
    //   sufficient           → writes, returns bytes written.
    size_t run(uint32_t worker_id, void* metas_out, size_t metas_out_bytes);

    // Persist the most recent run()'s metas to a binary file.
    void saveMetas(const std::string& path);

    // Reset for the next block. Asynchronous (no GPU sync); the caller is
    // expected to ensure prior block's run() finished before issuing
    // add_chunk() of the next block.
    void reset();

    // ─── Read-only state ──────────────────────────────────────────────

    float               last_chunk_to_final_ms() const { return last_chunk_to_final_ms_; }
    const InstanceMeta* metas_data()             const { return metas_.data(); }
    uint32_t            num_active_instances()   const { return num_active_; }
    uint32_t            n_chunks()               const { return n_chunks_; }
    cudaEvent_t         event_last_chunk_start() const { return e_last_chunk_start_; }
    cudaEvent_t         event_after_preproc()    const { return e_after_preproc_; }
    cudaEvent_t         event_after_prepare()    const { return e_after_prepare_; }
    cudaEvent_t         event_metas_ready()      const { return e_metas_ready_; }

private:
    static constexpr uint32_t POTENTIAL_FACTOR = 8;        // worst-case emits per memop
    static constexpr int      N_PRE_STREAMS    = N_STREAMS;

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

    cudaStream_t   streams_[N_PRE_STREAMS]             = {nullptr};
    MemOp*         d_memops_[N_PRE_STREAMS]            = {nullptr};
    uint32_t*      d_counts_[N_PRE_STREAMS]            = {nullptr};
    uint32_t*      d_potential_offsets_[N_PRE_STREAMS] = {nullptr};
    PotentialEmit* d_potentials_[N_PRE_STREAMS]        = {nullptr};
    uint32_t*      d_emit_bits_[N_PRE_STREAMS]         = {nullptr};
    uint32_t*      d_final_offsets_[N_PRE_STREAMS]     = {nullptr};
    uint64_t*      d_ram_keys_[N_PRE_STREAMS]          = {nullptr};
    uint64_t*      d_ram_keys_sorted_[N_PRE_STREAMS]   = {nullptr};
    uint32_t*      d_ram_vals_sorted_[N_PRE_STREAMS]   = {nullptr};
    uint32_t*      d_ram_count_[N_PRE_STREAMS]         = {nullptr};
    BlockOpSpill*  d_spill_[N_PRE_STREAMS]             = {nullptr};
    uint32_t*      d_spill_count_[N_PRE_STREAMS]       = {nullptr};
    uint8_t*       d_spill_status_[N_PRE_STREAMS]      = {nullptr};
    uint32_t*      d_sorted_addr_[N_PRE_STREAMS]       = {nullptr};
    uint32_t*      d_run_lengths_[N_PRE_STREAMS]       = {nullptr};
    uint32_t*      d_run_offsets_[N_PRE_STREAMS]       = {nullptr};
    uint32_t*      d_num_unique_[N_PRE_STREAMS]        = {nullptr};
    void*          d_cub_temp_[N_PRE_STREAMS]          = {nullptr};
    size_t         cub_temp_bytes_                     = 0;
    uint32_t*      h_n_emits_[N_PRE_STREAMS]           = {nullptr};

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

    uint32_t              max_chunks_           = 0;
    uint32_t              max_memops_per_chunk_ = 0;
    uint32_t              max_pot_per_chunk_    = 0;
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

    uint32_t                  active_mask_[MASK_WORDS]        = {0};
    uint32_t                  h_active_local_ids_[MAX_ACTIVE] = {0};
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

    // ─── Internal helpers (definitions in count_and_plan.cu) ────────

    void   free_all_();
    void   free_pinned_();
    void   query_cub_sizes_(size_t& scan_counts_b, size_t& scan_emit_b,
                            size_t& scan_runs_b,   size_t& sort_b,
                            size_t& rle_b,         size_t& hist_scan_b);
    void   prepare_global_();
    void   process_worker_(uint32_t worker_id);
    void   set_active_worker_(uint32_t w);
    void   pick_active_instances_();
    size_t serialize_metas_(void* out, size_t out_bytes);
};

// ─── Binary-meta save helpers (used by both CountAndPlan::saveMetas and
// the multi-worker save loop in main_full.cu).
// File layout: [u32 num_metas][per-meta record × num_metas]
//   per-meta: u32 inst_id, u8 type, u8 pad[3],
//             u32 first_addr, u32 last_addr,
//             u32 first_addr_chunk, u32 first_addr_skip,
//             u32 last_addr_chunk,  u32 last_addr_include,
//             u32 cps, u32 aos, u32[cps], u32[aos]
// Begin writes a placeholder count, then end seeks back and writes the real one.
FILE* save_metas_begin (const std::string& path);
void  save_metas_append(FILE* f, const InstanceMeta& m);
void  save_metas_end   (FILE* f, uint32_t total);

#endif  // COUNT_AND_PLAN_CUH