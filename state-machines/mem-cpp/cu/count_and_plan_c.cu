// =====================================================================
// C ABI shim around CountAndPlan
// =====================================================================

#include "count_and_plan.cuh"

extern "C" {

// ─── Lifecycle ──────────────────────────────────────────────────────
void* count_and_plan_create() {
    return new CountAndPlan();
}

void count_and_plan_destroy(void* h) {
    delete static_cast<CountAndPlan*>(h);
}

// ─── Pipeline ───────────────────────────────────────────────────────
bool count_and_plan_setup(void* h, void* d_buf, size_t bytes,
               uint32_t n_workers, uint32_t worker_id) {
    return static_cast<CountAndPlan*>(h)->setup(d_buf, bytes, n_workers, worker_id);
}

bool count_and_plan_add_chunk(void* h, const MemOp* memops, uint32_t n) {
    return static_cast<CountAndPlan*>(h)->add_chunk(memops, n);
}

bool count_and_plan_run(void* h, InstanceMeta** metas_out, uint32_t* n_metas) {
    uint32_t n = 0;
    const bool ok = static_cast<CountAndPlan*>(h)->run(metas_out, n);
    if (n_metas) *n_metas = n;
    return ok;
}

// Clear per-block state so the same planner instance can process the next
// block. Keeps the arena and per-stream resources alive (no cudaMalloc/Free).
void count_and_plan_reset(void* h) {
    if (!h) return;
    static_cast<CountAndPlan*>(h)->reset();
}

// Per-chunk mem-align counters, valid after `count_and_plan_run`. Returns a
// pointer to `*n_chunks` entries (one per submitted chunk) of the POD
// `ChunkCounters` struct declared in count_and_plan.cuh. Storage is owned by
// the planner; valid until the next `count_and_plan_reset` on this handle.
const ChunkCounters* count_and_plan_get_align_counters(void* h, uint32_t* n_chunks) {
    CountAndPlan* p = static_cast<CountAndPlan*>(h);
    if (n_chunks) *n_chunks = p->n_chunks();
    return p->align_counters_data();
}

}  // extern "C"
