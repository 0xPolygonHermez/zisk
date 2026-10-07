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
               uint32_t n_workers, uint32_t worker_id, int gpu_id,
               const uint32_t* instance_rows, bool retain_rows) {
    return static_cast<CountAndPlan*>(h)->setup(d_buf, bytes, n_workers, worker_id, gpu_id,
                                               instance_rows, retain_rows);
}

bool count_and_plan_add_chunk(void* h, const uint64_t* words, uint32_t n_words) {
    return static_cast<CountAndPlan*>(h)->add_chunk(words, n_words);
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

bool count_and_plan_register_input_pinned(void* h, void* ptr, size_t bytes) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->register_input_pinned(ptr, bytes);
}

size_t count_and_plan_max_used_bytes(void* h) {
    if (h == nullptr) return 0;
    return static_cast<CountAndPlan*>(h)->max_used_bytes();
}

void count_and_plan_unregister_input_pinned(void* h, void* ptr) {
    if (!h) return;
    static_cast<CountAndPlan*>(h)->unregister_input_pinned(ptr);
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

// Serialize `n` metas to `path` in the canonical `metas.bin` format (see
// instance_meta_loader.hpp), reusing the same save_metas_* helpers as the
// standalone runner so the file is loadable by `load_instance_metas`.
// Returns false on bad args; the save_metas_* helpers abort the process on an I/O error.
bool count_and_plan_save_metas(const InstanceMeta* metas, uint32_t n,
                               const char* path) {
    if (!metas || !path) return false;
    FILE* f = save_metas_begin(path);
    for (uint32_t i = 0; i < n; i++) save_metas_append(f, metas[i]);
    save_metas_end(f, n);
    return true;
}

// ─── RAM witness from the retained accesses ───────────────────────────
void count_and_plan_set_chunk_size_bits(void* h, uint32_t bits) {
    if (h) static_cast<CountAndPlan*>(h)->set_chunk_size_bits(bits);
}

bool count_and_plan_set_mem_layout(void* h, const uint32_t* col_widths, uint32_t n_cols,
                                   uint32_t words_per_row, uint32_t lanes_x_row) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->set_mem_layout(col_widths, n_cols, words_per_row, lanes_x_row);
}

bool count_and_plan_ram_retention_ok(void* h) {
    return h && static_cast<CountAndPlan*>(h)->ram_retention_ok();
}

bool count_and_plan_prepare_ram_fill(void* h, RamFillPrepared* out) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->prepare_ram_fill(out);
}

bool count_and_plan_fill_ram_instance(void* h, uint32_t inst, uint64_t* out_rows, uint32_t n_rows,
                                      RamFillResult* res) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->fill_ram_instance(inst, out_rows, n_rows, res);
}

bool count_and_plan_fill_all_ram_instances(void* h, uint32_t n_rows, const uint32_t* insts, uint32_t n_insts,
                                           RamFillPrepared* prepared) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->fill_all_ram_instances(n_rows, insts, n_insts, prepared);
}

const uint64_t* count_and_plan_ram_instance_rows(void* h, uint32_t inst, RamFillResult* res) {
    if (!h) return nullptr;
    return static_cast<CountAndPlan*>(h)->ram_instance_rows(inst, res);
}

bool count_and_plan_set_rom_layout(void* h, const uint32_t* col_widths, uint32_t n_cols,
                                   uint32_t words_per_row, uint32_t lanes_x_row) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->set_rom_layout(col_widths, n_cols, words_per_row, lanes_x_row);
}

bool count_and_plan_fill_all_rom_instances(void* h, uint32_t n_rows, const uint32_t* insts, uint32_t n_insts,
                                           RamFillPrepared* prepared) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->fill_all_rom_instances(n_rows, insts, n_insts, prepared);
}

const uint64_t* count_and_plan_rom_instance_rows(void* h, uint32_t inst, RamFillResult* res) {
    if (!h) return nullptr;
    return static_cast<CountAndPlan*>(h)->rom_instance_rows(inst, res);
}

bool count_and_plan_set_input_layout(void* h, const uint32_t* col_widths, uint32_t n_cols,
                                     uint32_t words_per_row, uint32_t lanes_x_row) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->set_input_layout(col_widths, n_cols, words_per_row, lanes_x_row);
}

bool count_and_plan_fill_all_input_instances(void* h, uint32_t n_rows, const void* image, size_t image_bytes,
                                             const uint32_t* insts, uint32_t n_insts, RamFillPrepared* prepared) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->fill_all_input_instances(n_rows, image, image_bytes, insts, n_insts, prepared);
}

const uint64_t* count_and_plan_input_instance_rows(void* h, uint32_t inst, RamFillResult* res) {
    if (!h) return nullptr;
    return static_cast<CountAndPlan*>(h)->input_instance_rows(inst, res);
}

bool count_and_plan_set_align_layout(void* h, uint32_t air_kind, const uint32_t* col_widths, uint32_t n_cols,
                                     uint32_t words_per_row) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->set_align_layout(air_kind, col_widths, n_cols, words_per_row);
}

bool count_and_plan_fill_all_align_instances(void* h, const AlignPlanDesc* plans, uint32_t n_plans,
                                             const AlignChunkEntry* entries, uint32_t n_entries, RamFillPrepared* prepared) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->fill_all_align_instances(plans, n_plans, entries, n_entries, prepared);
}

const uint64_t* count_and_plan_align_instance_rows(void* h, uint32_t air_id, uint32_t segment, RamFillResult* res) {
    if (!h) return nullptr;
    return static_cast<CountAndPlan*>(h)->align_instance_rows(air_id, segment, res);
}

bool count_and_plan_prepare_slot_fills(void* h, const void* image, size_t image_bytes, const AlignPlanDesc* plans,
                                       uint32_t n_plans, const AlignChunkEntry* entries, uint32_t n_entries,
                                       RamFillPrepared* prepared) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->prepare_slot_fills(image, image_bytes, plans, n_plans, entries, n_entries, prepared);
}

int count_and_plan_device(void* h) {
    return h ? static_cast<CountAndPlan*>(h)->device() : -1;
}

bool count_and_plan_fill_slot(void* h, const void* d_ops, uint64_t n_ops, uint64_t* dst, void* stream, RamFillResult* res) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->fill_slot(d_ops, n_ops, dst, stream, res);
}

void count_and_plan_slot_quiesce(void* h) {
    if (h) static_cast<CountAndPlan*>(h)->slot_quiesce();
}

bool count_and_plan_instance_scalars(void* h, uint32_t family, uint32_t inst, RamFillResult* res) {
    if (!h) return false;
    return static_cast<CountAndPlan*>(h)->instance_scalars(family, inst, res);
}

}  // extern "C"
