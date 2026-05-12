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

}  // extern "C"
