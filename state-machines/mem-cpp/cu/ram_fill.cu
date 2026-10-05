// The RAM witness from the accesses the planner retained (see count_and_plan.cuh).
//
// prepare_ram_fill, once per block after run(), finds every instance's address range and, per
// chunk, the slice of the chunk's address-sorted records that falls in it. Instances are cut by
// row count, so an address can straddle two of them: each instance takes its whole address range,
// pairs and resolves it, and packs only its window of lanes. Each instance is filled on its own,
// so the scratch scales with the largest instance, not the block:
//   0. gather — the instance's slices from all chunks into sort keys (address << 38 | step) and
//               arrival indexes.
//   1. sort   — the sorted order is the lane order once dual pairs merge, since lanes go by
//               address and, inside one, by time.
//   2. lanes  — dual pairing as a scan: a read pairs into the lane before it when that lane is
//               open (created by the previous access, not yet paired), on the same address and in
//               the same chunk; within a run of pairable reads the lanes alternate open/paired, so
//               emit = parity of the distance to the last non-pairable access.
//   3. values — every read's value is what the writes before it on that address left: a scan by
//               address of byte-masked merges, run in blocks with a carry so the scratch stays
//               bounded. A block whose stream carries writes without values never reaches the
//               fill: the planner declines the device witness for it.
//   4. rows   — the instance's window of lanes packed into its rows. The range starts at the
//               address holding the row before the window, so the lane the window continues
//               from is in the same data and every instance is filled on its own. The CPU
//               `fill_mem_range` is the oracle.

#include "count_and_plan.cuh"
#include "mem_pack.cuh"

#include <cub/cub.cuh>
#include <thrust/iterator/counting_iterator.h>
#include <cuda_runtime.h>

#include <algorithm>
#include <cstdio>

namespace {

constexpr int RF_BLOCK = 256;
constexpr uint32_t RF_STEP_BITS = 38;         // mem step bits inside a sort key (address above)
constexpr uint32_t RF_ADDR_BITS = 26;         // compact RAM word address bits
constexpr uint64_t RF_STEP_MASK = (1ull << RF_STEP_BITS) - 1;
constexpr size_t   RF_PROP_BLOCK = (size_t)1 << 24;  // accesses per value-propagation block
constexpr int      RF_REGION_RAM = 2;

#define RF_TRY(expr)                                                                      \
    do {                                                                                  \
        cudaError_t _e = (expr);                                                          \
        if (_e != cudaSuccess) {                                                          \
            fprintf(stderr, "ram_fill: %s failed: %s\n", #expr, cudaGetErrorString(_e));  \
            return false;                                                                 \
        }                                                                                 \
    } while (0)

inline uint32_t rf_grid(size_t n) { return (uint32_t)((n + RF_BLOCK - 1) / RF_BLOCK); }

struct MaxU32Op {
    __host__ __device__ __forceinline__ uint32_t operator()(uint32_t a, uint32_t b) const {
        return a > b ? a : b;
    }
};

// A byte-masked write as a function of the previous word: f(x) = (x & ~mask) | val.
struct Merge {
    uint64_t mask;
    uint64_t val;
};
// `b` after `a`.
struct MergeOp {
    __host__ __device__ __forceinline__ Merge operator()(const Merge& a, const Merge& b) const {
        Merge r;
        r.mask = a.mask | b.mask;
        r.val = (a.val & ~b.mask) | b.val;
        return r;
    }
};
struct EqU32 {
    __host__ __device__ __forceinline__ bool operator()(uint32_t a, uint32_t b) const { return a == b; }
};
struct SumU32 {
    __host__ __device__ __forceinline__ uint32_t operator()(uint32_t a, uint32_t b) const { return a + b; }
};

__device__ __forceinline__ uint32_t rf_kind(uint64_t meta) { return (uint32_t)((meta >> RAM_META_KIND_SHIFT) & 3u); }
__device__ __forceinline__ uint64_t rf_step(uint64_t meta) { return meta & RAM_META_STEP_MASK; }
__device__ __forceinline__ uint64_t rf_chunk(uint64_t meta, uint32_t chunk_bits) {
    return (rf_step(meta) - 1) >> (chunk_bits + 2);
}

// Sort keys: the address above the mem step, so equal keys are the same word at the same step; the
// stable sort then keeps their arrival order, which inside a chunk is the record order (the read
// of a partial write precedes its write).
// The arrival index travels through the sort with the access kind in its top two bits, so the
// passes that only need the kind do not read records at random.
constexpr uint32_t RF_IDX_KIND_SHIFT = 30;
constexpr uint32_t RF_IDX_MASK       = (1u << RF_IDX_KIND_SHIFT) - 1;
__device__ __forceinline__ uint32_t rf_sidx_kind(uint32_t s) { return s >> RF_IDX_KIND_SHIFT; }
__device__ __forceinline__ uint32_t rf_sidx_index(uint32_t s) { return s & RF_IDX_MASK; }

__global__ void rf_keys_kernel(RamRecords rec, size_t n, uint64_t* __restrict__ keys, uint32_t* __restrict__ idx) {
    const size_t i = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (i >= n) return;
    const uint64_t m = rec.meta(i);
    keys[i] = ((uint64_t)rec.addr(i) << RF_STEP_BITS) | (rf_step(m) & RF_STEP_MASK);
    idx[i] = (uint32_t)i | (rf_kind(m) << RF_IDX_KIND_SHIFT);
}

// sidx and the lane anchors from the sorted keys.
__device__ __forceinline__ uint64_t rf_key_chunk(uint64_t key, uint32_t chunk_bits) {
    return ((key & RF_STEP_MASK) - 1) >> (chunk_bits + 2);
}

__global__ void rf_anchors_kernel(const uint64_t* __restrict__ keys, const uint32_t* __restrict__ sidx,
                                  size_t n, uint32_t chunk_bits, uint32_t* __restrict__ anchor) {
    const size_t j = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (j >= n) return;
    bool pairable = false;
    if (j > 0 && rf_sidx_kind(sidx[j]) == 0) {
        const uint64_t k = keys[j], kp = keys[j - 1];
        pairable = (k >> RF_STEP_BITS) == (kp >> RF_STEP_BITS) &&
                   rf_key_chunk(k, chunk_bits) == rf_key_chunk(kp, chunk_bits);
    }
    anchor[j] = pairable ? 0u : (uint32_t)(j + 1);
}

__global__ void rf_emit_kernel(const uint32_t* __restrict__ last_anchor, size_t n, uint32_t* __restrict__ emit) {
    const size_t j = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (j >= n) return;
    const uint32_t a = last_anchor[j] - 1;
    emit[j] = (((uint32_t)j - a) & 1u) == 0u ? 1u : 0u;
}

// Merge elements and sorted addresses for one propagation block.
__global__ void rf_merge_in_kernel(const uint32_t* __restrict__ sidx, const uint64_t* __restrict__ skeys,
                                   RamRecords rec, size_t j0, size_t n, Merge* __restrict__ in,
                                   uint32_t* __restrict__ keys) {
    const size_t k = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (k >= n) return;
    const uint32_t s = sidx[j0 + k];
    const uint32_t i = rf_sidx_index(s);
    const uint32_t kind = rf_sidx_kind(s);
    Merge e;
    if (kind == 0) {
        e.mask = 0; e.val = 0;
    } else if (kind == 2) {
        const uint64_t m = rec.meta(i);
        const uint32_t off = (uint32_t)((m >> RAM_META_OFF_SHIFT) & 7u);
        const uint32_t width = (uint32_t)((m >> RAM_META_WIDTH_SHIFT) & 15u);
        const uint64_t bytes = width >= 8 ? ~0ull : ((1ull << (8 * width)) - 1);
        e.mask = bytes << (8 * off);
        e.val = rec.value(i) & e.mask;
    } else {
        e.mask = ~0ull; e.val = rec.value(i);
    }
    in[k] = e;
    keys[k] = (uint32_t)(skeys[j0 + k] >> RF_STEP_BITS);
}

// Resolved value of every access of the block: the state after it, composed with the carry for
// the first address run when it continues the previous block's last address.
__global__ void rf_merge_out_kernel(const Merge* __restrict__ out, const uint32_t* __restrict__ keys, size_t n,
                                    uint32_t carry_addr, Merge carry, uint64_t* __restrict__ resolved, size_t j0) {
    const size_t k = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (k >= n) return;
    Merge st = out[k];
    if (keys[k] == carry_addr && keys[0] == carry_addr) {
        // Same run as the carry only if every key before k equals it; the run is a prefix of the
        // block, so the test is that key k equals the first key and the first key equals the carry.
        MergeOp op;
        st = op(carry, st);
    }
    resolved[j0 + k] = st.val;
}

__device__ __forceinline__ MemPackLane rf_lane(size_t g, const uint32_t* __restrict__ lane_first,
                                               const uint32_t* __restrict__ sidx, const uint32_t* __restrict__ emit,
                                               size_t n_sorted, const uint64_t* __restrict__ skeys,
                                               const uint64_t* __restrict__ resolved) {
    const uint32_t j = lane_first[g];
    const uint64_t k = skeys[j];
    MemPackLane l;
    l.addr = (uint32_t)(k >> RF_STEP_BITS) + RAM_W_ADDR_BASE;
    l.step = k & RF_STEP_MASK;
    l.value = resolved[j];
    l.wr = rf_sidx_kind(sidx[j]) != 0;
    l.dual = ((size_t)j + 1 < n_sorted) && emit[j + 1] == 0;
    l.step_dual = l.dual ? (skeys[j + 1] & RF_STEP_MASK) : 0;
    return l;
}

__global__ void rf_rows_kernel(MemPackLayout layout, size_t lane_from, uint32_t n_lanes_inst, uint32_t n_rows,
                               const uint32_t* __restrict__ lane_first,
                               const uint32_t* __restrict__ sidx, const uint32_t* __restrict__ emit,
                               size_t n_sorted, const uint64_t* __restrict__ skeys,
                               const uint64_t* __restrict__ resolved, uint64_t* __restrict__ out_rows) {
    const uint32_t r = blockIdx.x * blockDim.x + threadIdx.x;
    if (r >= n_rows) return;
    const uint32_t L = layout.lanes_x_row;
    uint64_t w[MEMPACK_MAX_WORDS_PER_ROW];
#pragma unroll
    for (uint32_t k = 0; k < MEMPACK_MAX_WORDS_PER_ROW; ++k) w[k] = 0;

    const MemPackLane last = rf_lane(lane_from + n_lanes_inst - 1, lane_first, sidx, emit, n_sorted, skeys, resolved);
    const uint64_t pad_step = last.dual ? last.step_dual : last.step;
    // Padding reads the last word of the region; when the last access sat elsewhere the first
    // padding lane changes address, reads 0 and carries the distance.
    const bool pad_changes = last.addr != RAM_W_ADDR_LAST;
    const uint64_t pad_value = pad_changes ? 0 : last.value;
    const uint64_t pad_inc = pad_changes ? (uint64_t)RAM_W_ADDR_LAST - last.addr - 1 : 0;

    for (uint32_t lane = 0; lane < L; ++lane) {
        const uint32_t v = r * L + lane;
        if (v < n_lanes_inst) {
            const size_t g = lane_from + v;
            const MemPackLane me = rf_lane(g, lane_first, sidx, emit, n_sorted, skeys, resolved);
            bool addr_changes;
            uint64_t inc;
            if (g > 0) {
                const MemPackLane pv = rf_lane(g - 1, lane_first, sidx, emit, n_sorted, skeys, resolved);
                addr_changes = pv.addr != me.addr;
                inc = mempack_increment(addr_changes, me, pv.addr, pv.dual ? pv.step_dual : pv.step);
            } else {
                // No previous lane at all: the segment starts the memory cycle at the RAM base.
                addr_changes = true;
                inc = mempack_increment(true, me, RAM_W_ADDR_BASE - 1, 0);
            }
            mempack_lane(layout, w, lane, me, addr_changes, inc);
        } else {
            const bool first = v == n_lanes_inst;
            mempack_padding(layout, w, lane, RAM_W_ADDR_LAST, pad_step, pad_value, first && pad_changes,
                            first ? pad_inc : 0);
        }
    }
    uint64_t* dst = out_rows + (size_t)r * layout.words_per_row;
    for (uint32_t k = 0; k < layout.words_per_row; ++k) dst[k] = w[k];
}

// Per (chunk, instance): the ranks inside the chunk's address-sorted run where the instance's
// address range starts and ends (two columns per instance).
__global__ void rf_bounds_kernel(RamRecords rec, const uint32_t* __restrict__ chunk_base,
                                 const uint32_t* __restrict__ chunk_n, uint32_t n_chunks,
                                 const uint32_t* __restrict__ inst_first, const uint32_t* __restrict__ inst_last,
                                 uint32_t n_inst, uint32_t* __restrict__ bound) {
    const size_t t = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (t >= (size_t)n_chunks * 2 * n_inst) return;
    const uint32_t c = (uint32_t)(t / (2 * n_inst)), col = (uint32_t)(t % (2 * n_inst));
    const uint32_t n = chunk_n[c];
    const uint32_t a = (col & 1) ? inst_last[col >> 1] + 1 : inst_first[col >> 1];
    const size_t base = chunk_base[c];
    uint32_t lo = 0, hi = n;
    while (lo < hi) {
        const uint32_t mid = lo + (hi - lo) / 2;
        if (rec.addr(base + mid) < a) lo = mid + 1; else hi = mid;
    }
    bound[t] = lo;
}

// The instance's records from every chunk's slice, as sort keys and arrival indexes. `pref` is
// the exclusive prefix of the slice sizes over the chunks.
__global__ void rf_gather_kernel(RamRecords rec, const uint32_t* __restrict__ chunk_base,
                                 const uint32_t* __restrict__ pref, uint32_t n_chunks,
                                 const uint32_t* __restrict__ bound, uint32_t stride, uint32_t inst,
                                 size_t n, uint64_t* __restrict__ keys, uint32_t* __restrict__ idx) {
    const size_t j = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (j >= n) return;
    uint32_t lo = 0, hi = n_chunks;   // last chunk whose prefix is <= j
    while (lo + 1 < hi) {
        const uint32_t mid = (lo + hi) / 2;
        if (pref[mid] <= j) lo = mid; else hi = mid;
    }
    const size_t k = (size_t)chunk_base[lo] + bound[(size_t)lo * stride + 2 * inst] + (j - pref[lo]);
    const uint64_t m = rec.meta(k);
    keys[j] = ((uint64_t)rec.addr(k) << RF_STEP_BITS) | (rf_step(m) & RF_STEP_MASK);
    idx[j] = (uint32_t)k | (rf_kind(m) << RF_IDX_KIND_SHIFT);
}

}  // namespace

// count_and_plan.cu
__global__ void instance_boundaries_kernel(const uint32_t* prefix, uint32_t prefix_base_addr,
                                           uint32_t num_addr_region, uint32_t num_ops_region,
                                           uint32_t instance_size, const uint32_t* active_ids,
                                           uint32_t* active_first, uint32_t* active_last,
                                           uint32_t num_active);

bool CountAndPlan::set_mem_layout(const uint32_t* col_widths, uint32_t n_cols, uint32_t words_per_row,
                                  uint32_t lanes_x_row) {
    MemPackLayout l{};
    if (!mempack_layout(l, col_widths, n_cols, words_per_row, lanes_x_row)) return false;
    for (uint32_t c = 0; c < n_cols; ++c) mem_col_widths_[c] = col_widths[c];
    mem_n_cols_ = n_cols;
    mem_words_per_row_ = words_per_row;
    mem_lanes_x_row_ = lanes_x_row;
    return true;
}

// Tables for the per-instance fill, carved at the bottom of the dynamic region (the ops pool
// there is dead once run() returned; the fixed regions below it keep the prefix the fills read):
// the record runs' bases and counts (one run per piece of a chunk), every RAM instance's address
// range, the (run, instance) bounds and the per-instance run prefixes.
bool CountAndPlan::prepare_ram_fill(RamFillPrepared* out) {
    if (out) *out = RamFillPrepared{};
    if (!out) return false;
    cudaSetDevice(gpu_device_);
    const size_t n = ram_cursor_.load(std::memory_order_relaxed);
    out->n_accesses = n;
    if (!ram_retention_enabled_.load(std::memory_order_relaxed) || n == 0 || mem_n_cols_ == 0) {
        out->status = -1;
        return false;
    }
    if (n > (size_t)RF_IDX_MASK + 1) {
        fprintf(stderr, "ram_fill: %zu accesses exceed the %u the arrival index can address; RAM witness off\n",
                n, RF_IDX_MASK + 1);
        out->status = -3;
        return false;
    }
    const uint32_t n_inst = num_inst_[RF_REGION_RAM];
    if (ram_prepared_) {
        out->status = 0;
        out->n_instances = n_inst;
        out->n_lanes = ram_n_lanes_;
        out->ms_sort = ram_ms_[0]; out->ms_lanes = ram_ms_[1]; out->ms_values = ram_ms_[2]; out->ms_total = ram_ms_[3];
        return true;
    }
    if (!ram_tables_ready_) {
        std::vector<RamRun> runs;
        { std::lock_guard<std::mutex> lk(ram_runs_mtx_); runs = ram_runs_; }
        const uint32_t nc = (uint32_t)runs.size();
        rf_n_runs_ = nc;
        uint8_t* cur = arena_ + cursor_;
        auto take = [&](size_t bytes) -> uint8_t* {
            uint8_t* p = (uint8_t*)(((uintptr_t)cur + 255) & ~(uintptr_t)255);
            cur = p + bytes;
            return p;
        };
        d_rf_chunk_base_ = (uint32_t*)take((size_t)nc * 4);
        d_rf_chunk_n_    = (uint32_t*)take((size_t)nc * 4);
        d_rf_inst_ids_   = (uint32_t*)take((size_t)n_inst * 4);
        d_rf_inst_first_ = (uint32_t*)take((size_t)n_inst * 4);
        d_rf_inst_last_  = (uint32_t*)take((size_t)n_inst * 4);
        d_rf_bound_      = (uint32_t*)take((size_t)nc * 2 * n_inst * 4);
        d_rf_pref_       = (uint32_t*)take((size_t)nc * n_inst * 4);
        rf_scratch_      = (uint8_t*)take(0);
        std::vector<uint32_t> h_base(nc), h_n(nc), h_ids(n_inst);
        for (uint32_t c = 0; c < nc; ++c) { h_base[c] = runs[c].base; h_n[c] = runs[c].n; }
        for (uint32_t i = 0; i < n_inst; ++i) h_ids[i] = i;
        RF_TRY(cudaMemcpy(d_rf_chunk_base_, h_base.data(), (size_t)nc * 4, cudaMemcpyHostToDevice));
        RF_TRY(cudaMemcpy(d_rf_chunk_n_, h_n.data(), (size_t)nc * 4, cudaMemcpyHostToDevice));
        RF_TRY(cudaMemcpy(d_rf_inst_ids_, h_ids.data(), (size_t)n_inst * 4, cudaMemcpyHostToDevice));
        // Address range of every RAM instance of the block, from the prefix the plan left in place.
        instance_boundaries_kernel<<<1, n_inst>>>(d_prefix_, REGION_ADDR_START[RF_REGION_RAM],
            h_max_compact_[RF_REGION_RAM] + 1, region_n_ops_[RF_REGION_RAM], instance_rows_[RF_REGION_RAM],
            d_rf_inst_ids_, d_rf_inst_first_, d_rf_inst_last_, n_inst);
        RF_TRY(cudaGetLastError());
        // Instance ranges as compact RAM words, and the row the range starts at, so the instance
        // can skip the lanes of a straddling address that belong to the instance before it.
        const uint32_t region = REGION_ADDR_START[RF_REGION_RAM];
        std::vector<uint32_t> h_first(n_inst), h_last(n_inst);
        RF_TRY(cudaMemcpy(h_first.data(), d_rf_inst_first_, (size_t)n_inst * 4, cudaMemcpyDeviceToHost));
        RF_TRY(cudaMemcpy(h_last.data(), d_rf_inst_last_, (size_t)n_inst * 4, cudaMemcpyDeviceToHost));
        uint32_t region_row = 0;
        RF_TRY(cudaMemcpy(&region_row, d_prefix_ + region, 4, cudaMemcpyDeviceToHost));
        h_rf_inst_skip_.assign(n_inst, 0);
        h_rf_inst_lanes_.assign(n_inst, 0);
        for (uint32_t i = 0; i < n_inst; ++i) {
            uint32_t row_first = 0, row_end = 0;
            RF_TRY(cudaMemcpy(&row_first, d_prefix_ + h_first[i], 4, cudaMemcpyDeviceToHost));
            RF_TRY(cudaMemcpy(&row_end, d_prefix_ + h_last[i] + 1, 4, cudaMemcpyDeviceToHost));
            h_rf_inst_skip_[i] = (size_t)i * instance_rows_[RF_REGION_RAM] - (row_first - region_row);
            h_rf_inst_lanes_[i] = row_end - row_first;
            h_first[i] -= region;
            h_last[i] -= region;
        }
        RF_TRY(cudaMemcpy(d_rf_inst_first_, h_first.data(), (size_t)n_inst * 4, cudaMemcpyHostToDevice));
        RF_TRY(cudaMemcpy(d_rf_inst_last_, h_last.data(), (size_t)n_inst * 4, cudaMemcpyHostToDevice));
        rf_bounds_kernel<<<rf_grid((size_t)nc * 2 * n_inst), RF_BLOCK>>>(ram_records_, d_rf_chunk_base_,
            d_rf_chunk_n_, nc, d_rf_inst_first_, d_rf_inst_last_, n_inst, d_rf_bound_);
        RF_TRY(cudaGetLastError());
        std::vector<uint32_t> h_bound((size_t)nc * 2 * n_inst);
        RF_TRY(cudaMemcpy(h_bound.data(), d_rf_bound_, h_bound.size() * 4, cudaMemcpyDeviceToHost));
        std::vector<uint32_t> h_pref((size_t)nc * n_inst);
        h_rf_inst_count_.assign(n_inst, 0);
        for (uint32_t i = 0; i < n_inst; ++i) {
            size_t acc = 0;
            for (uint32_t c = 0; c < nc; ++c) {
                h_pref[(size_t)i * nc + c] = (uint32_t)acc;
                acc += h_bound[(size_t)c * 2 * n_inst + 2 * i + 1] - h_bound[(size_t)c * 2 * n_inst + 2 * i];
            }
            h_rf_inst_count_[i] = acc;
        }
        RF_TRY(cudaMemcpy(d_rf_pref_, h_pref.data(), h_pref.size() * 4, cudaMemcpyHostToDevice));
        RF_TRY(cudaMemcpy(&ram_writes_, d_ram_nwrites_, 8, cudaMemcpyDeviceToHost));
        ram_tables_ready_ = true;
    }
    out->status = 0;
    out->n_instances = n_inst;
    return true;
}

// One instance: gather, sort, pair, resolve, pack and copy out.
bool CountAndPlan::fill_ram_instance(uint32_t inst, uint64_t* out_rows, uint32_t n_rows, RamFillResult* res) {
    if (res) *res = RamFillResult{};
    if (!res || !out_rows) return false;
    cudaSetDevice(gpu_device_);
    if (!ram_tables_ready_) { res->status = -1; return false; }
    const size_t R = instance_rows_[RF_REGION_RAM];
    if ((size_t)n_rows * mem_lanes_x_row_ != R) {
        fprintf(stderr, "ram_fill: instance has %zu lanes, the caller's trace %u rows x %u lanes\n",
                R, n_rows, mem_lanes_x_row_);
        res->status = -2;
        return false;
    }
    const uint32_t n_inst = num_inst_[RF_REGION_RAM];
    if (inst >= n_inst) { res->status = -3; return false; }
    const size_t n = h_rf_inst_count_[inst];
    const size_t n_total = ram_cursor_.load(std::memory_order_relaxed);
    if (n == 0) { res->status = -3; return false; }
    const size_t   lane_from = h_rf_inst_skip_[inst];
    const uint32_t n_lanes_inst = (uint32_t)std::min(R, (size_t)region_n_ops_[RF_REGION_RAM] - (size_t)inst * R);
    if (inst > 0 && lane_from == 0) {
        fprintf(stderr, "ram_fill: instance %u's address range does not hold the lane before it\n", inst);
        res->status = -3;
        return false;
    }

    cudaEvent_t ev[6];
    for (auto& e : ev) RF_TRY(cudaEventCreate(&e));
    RF_TRY(cudaEventRecord(ev[0]));

    // Scratch above the tables, up to the retained accesses: two key and two index buffers that
    // the sort ping-pongs between (the sorted keys stay: they give every lane its address and
    // step; the sorted indexes are sidx; the other two become anchors, emit and the resolved
    // values), lane_first, the propagation block, the instance's rows, cub temp.
    uint8_t* cur = rf_scratch_;
    uint8_t* end = arena_ + (ram_low_edge_bytes(n_total) & ~(size_t)255);
    auto take = [&](size_t bytes) -> uint8_t* {
        uint8_t* p = (uint8_t*)(((uintptr_t)cur + 255) & ~(uintptr_t)255);
        cur = p + bytes;
        return p;
    };
    cub::DoubleBuffer<uint64_t> dkeys((uint64_t*)take(n * 8), (uint64_t*)take(n * 8));
    cub::DoubleBuffer<uint32_t> didx((uint32_t*)take(n * 4), (uint32_t*)take(n * 4));
    uint32_t* lfirst   = (uint32_t*)take(n * 4);
    uint32_t* d_n_lanes = (uint32_t*)take(4);
    const size_t prop_block = std::min(RF_PROP_BLOCK, n);
    Merge*    prop_in  = (Merge*)take(prop_block * sizeof(Merge));
    Merge*    prop_out = (Merge*)take(prop_block * sizeof(Merge));
    uint32_t* prop_keys = (uint32_t*)take(prop_block * 4);
    uint64_t* rows = (uint64_t*)take((size_t)n_rows * mem_words_per_row_ * 8);
    size_t t_sort = 0, t_max = 0, t_bykey = 0, t_select = 0;
    cub::DeviceRadixSort::SortPairs(nullptr, t_sort, dkeys, didx, n, 0, (int)(RF_STEP_BITS + RF_ADDR_BITS));
    cub::DeviceScan::InclusiveScan(nullptr, t_max, (uint32_t*)nullptr, (uint32_t*)nullptr, MaxU32Op(), n);
    cub::DeviceSelect::Flagged(nullptr, t_select, thrust::counting_iterator<uint32_t>(0),
                               (uint32_t*)nullptr, (uint32_t*)nullptr, (uint32_t*)nullptr, n);
    cub::DeviceScan::InclusiveScanByKey(nullptr, t_bykey, prop_keys, prop_in, prop_out, MergeOp(),
                                        prop_block, EqU32());
    const size_t t_bytes = std::max(std::max(t_sort, t_max), std::max(t_bykey, t_select));
    void* temp = take(t_bytes);
    const size_t scratch_bytes = (size_t)(cur - rf_scratch_);
    if (cur > end) {
        fprintf(stderr, "ram_fill: instance %u needs %zu MB of scratch, the arena has %zu MB below the retained "
                        "accesses; RAM witness off\n", inst, scratch_bytes >> 20, (size_t)(end - rf_scratch_) >> 20);
        res->status = -3;
        return false;
    }
    if (scratch_bytes > rf_scratch_peak_) rf_scratch_peak_ = scratch_bytes;

    // 0. gather, 1. sort by (address, step), stable
    rf_gather_kernel<<<rf_grid(n), RF_BLOCK>>>(ram_records_, d_rf_chunk_base_, d_rf_pref_ + (size_t)inst * rf_n_runs_,
                                               rf_n_runs_, d_rf_bound_, 2 * n_inst, inst, n,
                                               dkeys.Current(), didx.Current());
    RF_TRY(cudaGetLastError());
    size_t tb = t_bytes;
    RF_TRY(cub::DeviceRadixSort::SortPairs(temp, tb, dkeys, didx, n, 0, (int)(RF_STEP_BITS + RF_ADDR_BITS)));
    uint64_t* keys_out = dkeys.Current();
    uint32_t* sidx     = didx.Current();
    RF_TRY(cudaEventRecord(ev[1]));

    // 2. lanes. The free key buffer holds the anchors and the max-scan; the free index buffer, emit.
    uint32_t* anchor = (uint32_t*)dkeys.Alternate();
    uint32_t* last_anchor = anchor + n;
    uint32_t* emit = didx.Alternate();
    rf_anchors_kernel<<<rf_grid(n), RF_BLOCK>>>(keys_out, sidx, n, chunk_size_bits_, anchor);
    RF_TRY(cudaGetLastError());
    tb = t_bytes;
    RF_TRY(cub::DeviceScan::InclusiveScan(temp, tb, anchor, last_anchor, MaxU32Op(), n));
    rf_emit_kernel<<<rf_grid(n), RF_BLOCK>>>(last_anchor, n, emit);
    RF_TRY(cudaGetLastError());
    tb = t_bytes;
    RF_TRY(cub::DeviceSelect::Flagged(temp, tb, thrust::counting_iterator<uint32_t>(0), emit, lfirst,
                                      d_n_lanes, n));
    uint32_t n_lanes = 0;
    RF_TRY(cudaMemcpy(&n_lanes, d_n_lanes, 4, cudaMemcpyDeviceToHost));
    RF_TRY(cudaEventRecord(ev[2]));
    if (n_lanes != h_rf_inst_lanes_[inst] || lane_from + n_lanes_inst > n_lanes) {
        fprintf(stderr, "ram_fill: instance %u: %u lanes from the retained accesses, the plan counted %zu "
                        "(window %zu + %u)\n", inst, n_lanes, h_rf_inst_lanes_[inst], lane_from, n_lanes_inst);
        res->status = -4;
        return false;
    }

    // 3. values, in blocks with a carry. The free key buffer takes the resolved values. Instances
    // split at addresses, so no state carries in from the previous one.
    uint64_t* resolved = dkeys.Alternate();
    Merge carry{0, 0};
    uint32_t carry_addr = 0xFFFFFFFFu;
    for (size_t j0 = 0; j0 < n; j0 += prop_block) {
        const size_t len = std::min(prop_block, n - j0);
        rf_merge_in_kernel<<<rf_grid(len), RF_BLOCK>>>(sidx, keys_out, ram_records_, j0, len,
                                                       prop_in, prop_keys);
        RF_TRY(cudaGetLastError());
        tb = t_bytes;
        RF_TRY(cub::DeviceScan::InclusiveScanByKey(temp, tb, prop_keys, prop_in, prop_out, MergeOp(), len, EqU32()));
        rf_merge_out_kernel<<<rf_grid(len), RF_BLOCK>>>(prop_out, prop_keys, len, carry_addr, carry, resolved, j0);
        RF_TRY(cudaGetLastError());
        // Carry: the state after the block's last access, composed with the incoming carry when
        // the whole block was one run continuing it.
        Merge last_state;
        uint32_t first_key, last_key;
        RF_TRY(cudaMemcpy(&last_state, prop_out + (len - 1), sizeof(Merge), cudaMemcpyDeviceToHost));
        RF_TRY(cudaMemcpy(&first_key, prop_keys, 4, cudaMemcpyDeviceToHost));
        RF_TRY(cudaMemcpy(&last_key, prop_keys + (len - 1), 4, cudaMemcpyDeviceToHost));
        if (first_key == carry_addr && last_key == carry_addr) {
            last_state = MergeOp()(carry, last_state);
        }
        carry = last_state;
        carry_addr = last_key;
    }
    RF_TRY(cudaEventRecord(ev[3]));

    // 4. rows, then out to the caller's buffer.
    MemPackLayout layout{};
    mempack_layout(layout, mem_col_widths_, mem_n_cols_, mem_words_per_row_, mem_lanes_x_row_);
    rf_rows_kernel<<<(n_rows + RF_BLOCK - 1) / RF_BLOCK, RF_BLOCK>>>(
        layout, lane_from, n_lanes_inst, n_rows, lfirst, sidx, emit, n, keys_out, resolved, rows);
    RF_TRY(cudaGetLastError());
    RF_TRY(cudaEventRecord(ev[4]));
    RF_TRY(cudaMemcpy(out_rows, rows, (size_t)n_rows * mem_words_per_row_ * 8, cudaMemcpyDeviceToHost));
    RF_TRY(cudaEventRecord(ev[5]));
    RF_TRY(cudaDeviceSynchronize());

    // Continuation scalars: the lane before the window and the window's last lane.
    auto lane_scalars = [&](size_t g, uint32_t& addr_w, uint64_t& step, uint64_t& value) -> bool {
        uint32_t j = 0, e_next = 1;
        uint64_t k = 0;
        RF_TRY(cudaMemcpy(&j, lfirst + g, 4, cudaMemcpyDeviceToHost));
        RF_TRY(cudaMemcpy(&k, keys_out + j, 8, cudaMemcpyDeviceToHost));
        RF_TRY(cudaMemcpy(&value, resolved + j, 8, cudaMemcpyDeviceToHost));
        addr_w = (uint32_t)(k >> RF_STEP_BITS) + RAM_W_ADDR_BASE;
        step = k & RF_STEP_MASK;
        if ((size_t)j + 1 < n) {
            RF_TRY(cudaMemcpy(&e_next, emit + j + 1, 4, cudaMemcpyDeviceToHost));
            if (e_next == 0) {
                uint64_t k2 = 0;
                RF_TRY(cudaMemcpy(&k2, keys_out + j + 1, 8, cudaMemcpyDeviceToHost));
                step = k2 & RF_STEP_MASK;
            }
        }
        return true;
    };
    if (!lane_scalars(lane_from + n_lanes_inst - 1, res->last_addr_w, res->last_step, res->last_value)) return false;
    if (lane_from > 0) {
        if (!lane_scalars(lane_from - 1, res->prev_addr_w, res->prev_step, res->prev_value)) return false;
    } else {
        res->prev_addr_w = RAM_W_ADDR_BASE;
        res->prev_step = 0;
        res->prev_value = 0;
    }
    res->n_lanes = n_lanes_inst;
    auto ms = [&](int a, int b) { float t = 0; cudaEventElapsedTime(&t, ev[a], ev[b]); return t; };
    ram_ms_[0] += ms(0, 1); ram_ms_[1] += ms(1, 2); ram_ms_[2] += ms(2, 3); ram_ms_[3] += ms(0, 3);
    res->ms_rows = ms(3, 4);
    res->ms_d2h = ms(4, 5);
    res->status = 0;
    ram_n_lanes_ += n_lanes_inst;
    fprintf(stderr, "ram_fill: instance %u: %zu accesses, %u lanes (window %zu + %u), scratch %zu MB\n",
            inst, n, n_lanes, lane_from, n_lanes_inst, scratch_bytes >> 20);
    for (auto& e : ev) cudaEventDestroy(e);
    return true;
}

bool CountAndPlan::fill_all_ram_instances(uint32_t n_rows, RamFillPrepared* prepared) {
    if (!prepare_ram_fill(prepared)) return false;
    if (ram_prepared_) return true;
    const uint32_t n_inst = prepared->n_instances;
    const size_t stride = (size_t)n_rows * mem_words_per_row_;
    const size_t need = stride * n_inst;
    join_rows_prealloc_();
    if (need > h_ram_rows_cap_) {
        if (h_ram_rows_) { cudaFreeHost(h_ram_rows_); h_ram_rows_ = nullptr; h_ram_rows_cap_ = 0; }
        if (cudaMallocHost(&h_ram_rows_, need * 8) != cudaSuccess) {
            fprintf(stderr, "ram_fill: pinned allocation of %zu MB for the instance rows failed\n", (need * 8) >> 20);
            h_ram_rows_ = nullptr;
            return false;
        }
        h_ram_rows_cap_ = need;
    }
    ram_rows_stride_ = stride;
    ram_results_.assign(n_inst, RamFillResult{});
    ram_n_lanes_ = 0;
    for (int t = 0; t < 4; ++t) ram_ms_[t] = 0;
    rf_scratch_peak_ = 0;
    for (uint32_t i = 0; i < n_inst; ++i) {
        if (!fill_ram_instance(i, h_ram_rows_ + (size_t)i * stride, n_rows, &ram_results_[i])) {
            prepared->status = ram_results_[i].status;
            return false;
        }
    }
    ram_prepared_ = true;
    const size_t n = ram_cursor_.load(std::memory_order_relaxed);
    size_t largest = 0;
    for (uint32_t i = 0; i < n_inst; ++i) largest = std::max(largest, h_rf_inst_count_[i]);
    fprintf(stderr, "ram_fill: %zu accesses retained (%zu MB), %llu writes (%.1f%%), %u instances, largest %zu accesses, "
                    "scratch peak %zu MB, tables %zu MB\n",
            n, n * 20 >> 20, (unsigned long long)ram_writes_, n ? 100.0 * (double)ram_writes_ / (double)n : 0.0,
            n_inst, largest, rf_scratch_peak_ >> 20, (size_t)(rf_scratch_ - arena_) >> 20);
    return prepare_ram_fill(prepared);
}

const uint64_t* CountAndPlan::ram_instance_rows(uint32_t inst, RamFillResult* res) const {
    if (inst >= ram_results_.size() || h_ram_rows_ == nullptr) return nullptr;
    if (res) *res = ram_results_[inst];
    return h_ram_rows_ + (size_t)inst * ram_rows_stride_;
}

// ─── RomData: the ROM accesses retained in arrival order, per instance ──────────────────────────
//
// A RomData lane is one access: address, step, value, and addr_change on the first lane of each
// word. The init write of every word (mem step MEMORY_INIT_STEP) is in the stream with its value;
// every read of the word resolves to it with the same merge scan as RAM. Lanes follow (address,
// step), which inside a word is the arrival order the CPU fill keeps. Padding lanes repeat the last
// lane with step MEMORY_INIT_STEP and addr_change 0.

constexpr int      RF_REGION_ROM = 0;
// Kinds of a retained record (count_and_plan.cu): read, full write, partial write.
constexpr uint32_t RF_KIND_READ = 0, RF_KIND_PARTIAL = 2;
constexpr uint32_t ROM_W_ADDR_BASE = ZISK_ROM_ADDR_BASE >> 3;
constexpr uint64_t MEMORY_INIT_STEP = 3;

bool CountAndPlan::set_rom_layout(const uint32_t* col_widths, uint32_t n_cols, uint32_t words_per_row,
                                  uint32_t lanes_x_row) {
    RowPackLayout l;
    if (lanes_x_row == 0 || n_cols != 5 * lanes_x_row || !rowpack_layout(l, col_widths, n_cols, words_per_row))
        return false;
    for (uint32_t c = 0; c < n_cols; ++c) rom_col_widths_[c] = col_widths[c];
    rom_n_cols_ = n_cols;
    rom_words_per_row_ = words_per_row;
    rom_lanes_x_row_ = lanes_x_row;
    return true;
}

// Every retained ROM or input access: its record index and compact address, in run order.
__global__ void other_index_kernel(RamRecords rec, const uint32_t* __restrict__ run_base,
                                   const uint32_t* __restrict__ run_pref, uint32_t n_runs, size_t n,
                                   uint32_t* __restrict__ idx, uint32_t* __restrict__ addr) {
    const size_t j = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (j >= n) return;
    uint32_t lo = 0, hi = n_runs;   // last run whose prefix is <= j
    while (lo + 1 < hi) {
        const uint32_t mid = (lo + hi) / 2;
        if (run_pref[mid] <= j) lo = mid; else hi = mid;
    }
    const size_t k = (size_t)run_base[lo] + (j - run_pref[lo]);
    idx[j] = (uint32_t)k;
    addr[j] = rec.addr(k);
}

// The ROM and input accesses as one table (index and address per access), carved after the RAM
// fill's tables: the RAM instance scratch is dead by then.
bool CountAndPlan::prepare_other_index_() {
    if (d_other_idx_) return true;
    std::vector<RamRun> runs;
    { std::lock_guard<std::mutex> lk(ram_runs_mtx_); runs = other_runs_; }
    size_t total = 0;
    std::vector<uint32_t> h_base(runs.size()), h_pref(runs.size() + 1);
    for (size_t r = 0; r < runs.size(); ++r) { h_base[r] = runs[r].base; h_pref[r] = (uint32_t)total; total += runs[r].n; }
    h_pref[runs.size()] = (uint32_t)total;
    other_total_ = total;
    if (total == 0) return true;
    if (total > (size_t)RF_IDX_MASK + 1) {
        fprintf(stderr, "rom_fill: %zu ROM and input accesses exceed the index; witness off\n", total);
        return false;
    }
    uint8_t* cur = rf_scratch_ ? rf_scratch_ : arena_ + cursor_;
    auto take = [&](size_t bytes) -> uint8_t* {
        uint8_t* p = (uint8_t*)(((uintptr_t)cur + 255) & ~(uintptr_t)255);
        cur = p + bytes;
        return p;
    };
    uint32_t* d_base = (uint32_t*)take(h_base.size() * 4);
    uint32_t* d_pref = (uint32_t*)take(h_pref.size() * 4);
    d_other_idx_  = (uint32_t*)take(total * 4);
    d_other_addr_ = (uint32_t*)take(total * 4);
    other_scratch_ = (uint8_t*)take(0);
    if (other_scratch_ > arena_ + ram_low_edge_bytes(ram_cursor_.load(std::memory_order_relaxed))) {
        fprintf(stderr, "rom_fill: no room for the access table below the retained accesses; witness off\n");
        d_other_idx_ = nullptr;
        return false;
    }
    RF_TRY(cudaMemcpy(d_base, h_base.data(), h_base.size() * 4, cudaMemcpyHostToDevice));
    RF_TRY(cudaMemcpy(d_pref, h_pref.data(), h_pref.size() * 4, cudaMemcpyHostToDevice));
    other_index_kernel<<<rf_grid(total), RF_BLOCK>>>(ram_records_, d_base, d_pref, (uint32_t)runs.size(), total,
                                                     d_other_idx_, d_other_addr_);
    RF_TRY(cudaGetLastError());
    RF_TRY(cudaDeviceSynchronize());
    return true;
}

__global__ void other_flag_kernel(const uint32_t* __restrict__ addr, size_t n, uint32_t first, uint32_t last,
                                  uint32_t* __restrict__ flag) {
    const size_t j = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (j >= n) return;
    const uint32_t a = addr[j];
    flag[j] = (a >= first && a <= last) ? 1u : 0u;
}

// Sort keys of the selected accesses: the mem step, then (stable) the address.
__global__ void other_keys_kernel(RamRecords rec, const uint32_t* __restrict__ sel, const uint32_t* __restrict__ other_idx,
                                  size_t n, uint64_t* __restrict__ step_keys, uint32_t* __restrict__ idx) {
    const size_t j = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (j >= n) return;
    const uint32_t k = other_idx[sel[j]];
    idx[j] = k;
    step_keys[j] = rf_step(rec.meta(k));
}
__global__ void other_addr_keys_kernel(RamRecords rec, const uint32_t* __restrict__ idx, size_t n,
                                       uint32_t* __restrict__ addr_keys) {
    const size_t j = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (j >= n) return;
    addr_keys[j] = rec.addr(idx[j]);
}

// Bump allocator over a scratch range, 256-byte aligned.
struct ScratchCursor {
    uint8_t* cur;
    uint8_t* take(size_t bytes) {
        uint8_t* p = (uint8_t*)(((uintptr_t)cur + 255) & ~(uintptr_t)255);
        cur = p + bytes;
        return p;
    }
};

// Address range and lane window of every instance of a ROM or input region, from the prefix the
// plan left in place: `skip` lanes of the instance's address range belong to the instance before
// it (an address can straddle two instances), `count` is the accesses in the range.
bool CountAndPlan::other_geometry_(int region, std::vector<uint32_t>& first, std::vector<uint32_t>& last,
                                   std::vector<size_t>& skip, std::vector<size_t>& count) {
    const uint32_t n_inst = num_inst_[region];
    uint32_t* d_ids = (uint32_t*)other_scratch_; uint32_t* d_first = d_ids + n_inst; uint32_t* d_last = d_first + n_inst;
    std::vector<uint32_t> h_ids(n_inst);
    for (uint32_t i = 0; i < n_inst; ++i) h_ids[i] = i;
    RF_TRY(cudaMemcpy(d_ids, h_ids.data(), (size_t)n_inst * 4, cudaMemcpyHostToDevice));
    instance_boundaries_kernel<<<1, n_inst>>>(d_prefix_, REGION_ADDR_START[region], h_max_compact_[region] + 1,
                                              region_n_ops_[region], instance_rows_[region], d_ids, d_first, d_last, n_inst);
    RF_TRY(cudaGetLastError());
    first.assign(n_inst, 0); last.assign(n_inst, 0);
    RF_TRY(cudaMemcpy(first.data(), d_first, (size_t)n_inst * 4, cudaMemcpyDeviceToHost));
    RF_TRY(cudaMemcpy(last.data(), d_last, (size_t)n_inst * 4, cudaMemcpyDeviceToHost));
    uint32_t region_row = 0;
    RF_TRY(cudaMemcpy(&region_row, d_prefix_ + REGION_ADDR_START[region], 4, cudaMemcpyDeviceToHost));
    skip.assign(n_inst, 0); count.assign(n_inst, 0);
    for (uint32_t i = 0; i < n_inst; ++i) {
        uint32_t row_first = 0, row_end = 0;
        RF_TRY(cudaMemcpy(&row_first, d_prefix_ + first[i], 4, cudaMemcpyDeviceToHost));
        RF_TRY(cudaMemcpy(&row_end, d_prefix_ + last[i] + 1, 4, cudaMemcpyDeviceToHost));
        skip[i] = (size_t)i * instance_rows_[region] - (row_first - region_row);
        count[i] = row_end - row_first;   // no pairing: one lane per access
    }
    return true;
}

// The retained accesses with a compact address in [first, last], in (address, step) order:
// `addr_sorted` and `idx` (record index) of each, `n` of them. Carved from `sc`.
bool CountAndPlan::other_sorted_(ScratchCursor& sc, uint32_t first, uint32_t last, size_t expect,
                                 uint32_t** addr_sorted, uint32_t** idx, size_t* n_out) {
    const size_t N = other_total_;
    uint32_t* flag = (uint32_t*)sc.take(N * 4);
    uint32_t* sel = (uint32_t*)sc.take(N * 4);
    uint32_t* d_n_sel = (uint32_t*)sc.take(4);
    size_t t_select = 0;
    cub::DeviceSelect::Flagged(nullptr, t_select, thrust::counting_iterator<uint32_t>(0), (uint32_t*)nullptr,
                               (uint32_t*)nullptr, (uint32_t*)nullptr, N);
    void* temp0 = sc.take(t_select);
    other_flag_kernel<<<rf_grid(N), RF_BLOCK>>>(d_other_addr_, N, first, last, flag);
    RF_TRY(cudaGetLastError());
    RF_TRY(cub::DeviceSelect::Flagged(temp0, t_select, thrust::counting_iterator<uint32_t>(0), flag, sel, d_n_sel, N));
    uint32_t n32 = 0;
    RF_TRY(cudaMemcpy(&n32, d_n_sel, 4, cudaMemcpyDeviceToHost));
    const size_t n = n32;
    if (n != expect) {
        fprintf(stderr, "other_fill: %zu accesses selected in [%u, %u], the plan counted %zu\n", n, first, last, expect);
        return false;
    }
    cub::DoubleBuffer<uint64_t> skeys((uint64_t*)sc.take(n * 8), (uint64_t*)sc.take(n * 8));
    cub::DoubleBuffer<uint32_t> didx((uint32_t*)sc.take(n * 4), (uint32_t*)sc.take(n * 4));
    cub::DoubleBuffer<uint32_t> akeys((uint32_t*)sc.take(n * 4), (uint32_t*)sc.take(n * 4));
    size_t t_sort1 = 0, t_sort2 = 0;
    cub::DeviceRadixSort::SortPairs(nullptr, t_sort1, skeys, didx, n, 0, (int)RF_STEP_BITS);
    cub::DeviceRadixSort::SortPairs(nullptr, t_sort2, akeys, didx, n, 0, 32);
    const size_t t_bytes = std::max(t_sort1, t_sort2);
    void* temp = sc.take(t_bytes);
    // Sort by step, then stable by address: (address, step) order.
    other_keys_kernel<<<rf_grid(n), RF_BLOCK>>>(ram_records_, sel, d_other_idx_, n, skeys.Current(), didx.Current());
    RF_TRY(cudaGetLastError());
    size_t tb = t_bytes;
    RF_TRY(cub::DeviceRadixSort::SortPairs(temp, tb, skeys, didx, n, 0, (int)RF_STEP_BITS));
    other_addr_keys_kernel<<<rf_grid(n), RF_BLOCK>>>(ram_records_, didx.Current(), n, akeys.Current());
    RF_TRY(cudaGetLastError());
    tb = t_bytes;
    RF_TRY(cub::DeviceRadixSort::SortPairs(temp, tb, akeys, didx, n, 0, 32));
    *addr_sorted = akeys.Current();
    *idx = didx.Current();
    *n_out = n;
    return true;
}

// Merge elements for one propagation block: a write seeds the word, a read carries it.
__global__ void rom_merge_in_kernel(RamRecords rec, const uint32_t* __restrict__ idx, size_t j0, size_t n,
                                    Merge* __restrict__ in) {
    const size_t k = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (k >= n) return;
    const uint32_t i = idx[j0 + k];
    const uint64_t m = rec.meta(i);
    const uint32_t kind = rf_kind(m);
    Merge e;
    if (kind == RF_KIND_READ) { e.mask = 0; e.val = 0; }
    else if (kind == RF_KIND_PARTIAL) {
        const uint32_t off = (uint32_t)((m >> RAM_META_OFF_SHIFT) & 7u);
        const uint32_t width = (uint32_t)((m >> RAM_META_WIDTH_SHIFT) & 15u);
        const uint64_t bytes = width >= 8 ? ~0ull : ((1ull << (8 * width)) - 1);
        e.mask = bytes << (8 * off);
        e.val = rec.value(i) & e.mask;
    } else { e.mask = ~0ull; e.val = rec.value(i); }
    in[k] = e;
}

__global__ void rom_merge_out_kernel(const Merge* __restrict__ out, const uint32_t* __restrict__ keys, size_t n,
                                     uint32_t carry_addr, Merge carry, uint64_t* __restrict__ resolved, size_t j0) {
    const size_t k = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
    if (k >= n) return;
    Merge st = out[k];
    if (keys[k] == carry_addr && keys[0] == carry_addr) { MergeOp op; st = op(carry, st); }
    resolved[j0 + k] = st.val;
}

// Column index of a RomData lane's field: groups addr_change, addr, step, value[2], each with one
// column per lane (two for value), in the trace's declaration order.
__device__ __forceinline__ uint32_t rom_col(uint32_t lanes, uint32_t group, uint32_t lane, uint32_t idx) {
    return group < 3 ? group * lanes + lane : 3 * lanes + lane * 2 + idx;
}

__global__ void rom_rows_kernel(RowPackLayout layout, uint32_t lanes_x_row, size_t lane_from, uint32_t n_lanes_inst,
                                uint32_t n_rows, const uint32_t* __restrict__ addr_sorted,
                                const uint32_t* __restrict__ idx, const uint64_t* __restrict__ resolved,
                                RamRecords rec, uint64_t* __restrict__ out_rows) {
    const uint32_t r = blockIdx.x * blockDim.x + threadIdx.x;
    if (r >= n_rows) return;
    uint64_t w[MEMPACK_MAX_WORDS_PER_ROW];
#pragma unroll
    for (uint32_t k = 0; k < MEMPACK_MAX_WORDS_PER_ROW; ++k) w[k] = 0;
    const size_t g_last = lane_from + n_lanes_inst - 1;
    const uint32_t last_addr = addr_sorted[g_last];
    const uint32_t ki = idx[g_last];
    const uint64_t last_value = rf_kind(rec.meta(ki)) == RF_KIND_READ ? resolved[g_last] : rec.value(ki);
    for (uint32_t lane = 0; lane < lanes_x_row; ++lane) {
        const uint32_t v = r * lanes_x_row + lane;
        uint32_t addr; uint64_t step, value; bool change;
        if (v < n_lanes_inst) {
            const size_t g = lane_from + v;
            const uint32_t k = idx[g];
            const uint64_t m = rec.meta(k);
            addr = addr_sorted[g];
            step = rf_step(m);
            value = rf_kind(m) == RF_KIND_READ ? resolved[g] : rec.value(k);
            change = g == 0 || addr_sorted[g - 1] != addr;
        } else {
            addr = last_addr; step = MEMORY_INIT_STEP; value = last_value; change = false;
        }
        rowpack_put(layout, w, rom_col(lanes_x_row, 0, lane, 0), change ? 1 : 0);
        rowpack_put(layout, w, rom_col(lanes_x_row, 1, lane, 0), ROM_W_ADDR_BASE + addr);
        rowpack_put(layout, w, rom_col(lanes_x_row, 2, lane, 0), step);
        rowpack_put(layout, w, rom_col(lanes_x_row, 3, lane, 0), (uint32_t)value);
        rowpack_put(layout, w, rom_col(lanes_x_row, 3, lane, 1), (uint32_t)(value >> 32));
    }
    uint64_t* dst = out_rows + (size_t)r * layout.words_per_row;
    for (uint32_t k = 0; k < layout.words_per_row; ++k) dst[k] = w[k];
}

bool CountAndPlan::fill_rom_instance(uint32_t inst, uint64_t* out_rows, uint32_t n_rows, RamFillResult* res) {
    if (res) *res = RamFillResult{};
    if (!res || !out_rows) return false;
    const size_t R = instance_rows_[RF_REGION_ROM];
    if ((size_t)n_rows * rom_lanes_x_row_ != R) {
        fprintf(stderr, "rom_fill: instance has %zu lanes, the caller's trace %u rows x %u lanes\n", R, n_rows, rom_lanes_x_row_);
        res->status = -2;
        return false;
    }
    const uint32_t n_inst = num_inst_[RF_REGION_ROM];
    if (inst >= n_inst || other_total_ == 0) { res->status = -3; return false; }
    const size_t lane_from = h_rom_inst_skip_[inst];
    const uint32_t n_lanes_inst = (uint32_t)std::min(R, (size_t)region_n_ops_[RF_REGION_ROM] - (size_t)inst * R);
    const size_t n_total = ram_cursor_.load(std::memory_order_relaxed);

    cudaEvent_t ev[4];
    for (auto& e : ev) RF_TRY(cudaEventCreate(&e));
    RF_TRY(cudaEventRecord(ev[0]));

    // Scratch above the access table, up to the retained accesses.
    ScratchCursor sc{other_scratch_};
    uint8_t* end = arena_ + (ram_low_edge_bytes(n_total) & ~(size_t)255);
    uint32_t* addr_sorted; uint32_t* idx; size_t n;
    if (!other_sorted_(sc, h_rom_inst_first_[inst], h_rom_inst_last_[inst], h_rom_inst_count_[inst], &addr_sorted, &idx, &n)
        || lane_from + n_lanes_inst > n) {
        res->status = -4;
        return false;
    }
    uint64_t* resolved = (uint64_t*)sc.take(n * 8);
    const size_t prop_block = std::min(RF_PROP_BLOCK, n);
    Merge* prop_in = (Merge*)sc.take(prop_block * sizeof(Merge));
    Merge* prop_out = (Merge*)sc.take(prop_block * sizeof(Merge));
    uint64_t* rows = (uint64_t*)sc.take((size_t)n_rows * rom_words_per_row_ * 8);
    size_t t_bytes = 0;
    cub::DeviceScan::InclusiveScanByKey(nullptr, t_bytes, addr_sorted, prop_in, prop_out, MergeOp(), prop_block, EqU32());
    void* temp = sc.take(t_bytes);
    if (sc.cur > end) {
        fprintf(stderr, "rom_fill: instance %u needs %zu MB of scratch, %zu MB free below the retained accesses\n",
                inst, (size_t)(sc.cur - other_scratch_) >> 20, (size_t)(end - other_scratch_) >> 20);
        res->status = -3;
        return false;
    }
    RF_TRY(cudaEventRecord(ev[1]));

    // Values: the init write seeds each word, every read of it resolves to the word.
    Merge carry{0, 0};
    uint32_t carry_addr = 0xFFFFFFFFu;
    for (size_t j0 = 0; j0 < n; j0 += prop_block) {
        const size_t len = std::min(prop_block, n - j0);
        rom_merge_in_kernel<<<rf_grid(len), RF_BLOCK>>>(ram_records_, idx, j0, len, prop_in);
        RF_TRY(cudaGetLastError());
        size_t tb = t_bytes;
        RF_TRY(cub::DeviceScan::InclusiveScanByKey(temp, tb, addr_sorted + j0, prop_in, prop_out, MergeOp(), len, EqU32()));
        rom_merge_out_kernel<<<rf_grid(len), RF_BLOCK>>>(prop_out, addr_sorted + j0, len, carry_addr, carry, resolved, j0);
        RF_TRY(cudaGetLastError());
        Merge last_state;
        uint32_t first_key, last_key;
        RF_TRY(cudaMemcpy(&last_state, prop_out + (len - 1), sizeof(Merge), cudaMemcpyDeviceToHost));
        RF_TRY(cudaMemcpy(&first_key, addr_sorted + j0, 4, cudaMemcpyDeviceToHost));
        RF_TRY(cudaMemcpy(&last_key, addr_sorted + j0 + (len - 1), 4, cudaMemcpyDeviceToHost));
        if (first_key == carry_addr && last_key == carry_addr) last_state = MergeOp()(carry, last_state);
        carry = last_state;
        carry_addr = last_key;
    }
    RF_TRY(cudaEventRecord(ev[2]));

    // Rows, then out.
    RowPackLayout layout{};
    rowpack_layout(layout, rom_col_widths_, rom_n_cols_, rom_words_per_row_);
    rom_rows_kernel<<<(n_rows + RF_BLOCK - 1) / RF_BLOCK, RF_BLOCK>>>(
        layout, rom_lanes_x_row_, lane_from, n_lanes_inst, n_rows, addr_sorted, idx, resolved, ram_records_, rows);
    RF_TRY(cudaGetLastError());
    RF_TRY(cudaMemcpy(out_rows, rows, (size_t)n_rows * rom_words_per_row_ * 8, cudaMemcpyDeviceToHost));
    RF_TRY(cudaEventRecord(ev[3]));
    RF_TRY(cudaDeviceSynchronize());

    // The last lane; the lane before the instance is the previous instance's last.
    {
        const size_t g = lane_from + n_lanes_inst - 1;
        uint32_t a = 0, k = 0;
        RF_TRY(cudaMemcpy(&a, addr_sorted + g, 4, cudaMemcpyDeviceToHost));
        RF_TRY(cudaMemcpy(&k, idx + g, 4, cudaMemcpyDeviceToHost));
        uint32_t r[RAM_RECORD_WORDS];
        RF_TRY(cudaMemcpy(r, ram_records_.rec(k), RAM_RECORD_WORDS * 4, cudaMemcpyDeviceToHost));
        const uint64_t m = r[1] | ((uint64_t)r[2] << 32);
        res->last_addr_w = ROM_W_ADDR_BASE + a;
        res->last_step = m & RAM_META_STEP_MASK;
        if (((m >> RAM_META_KIND_SHIFT) & 3u) == RF_KIND_READ) {
            RF_TRY(cudaMemcpy(&res->last_value, resolved + g, 8, cudaMemcpyDeviceToHost));
        } else {
            res->last_value = r[3] | ((uint64_t)r[4] << 32);
        }
    }
    if (inst > 0) {
        const RamFillResult& p = rom_results_[inst - 1];
        res->prev_addr_w = p.last_addr_w; res->prev_step = p.last_step; res->prev_value = p.last_value;
    } else {
        res->prev_addr_w = ROM_W_ADDR_BASE; res->prev_step = 0; res->prev_value = 0;
    }
    res->n_lanes = n_lanes_inst;
    auto ms = [&](int a, int b) { float t = 0; cudaEventElapsedTime(&t, ev[a], ev[b]); return t; };
    res->ms_rows = ms(2, 3);
    res->ms_d2h = 0;
    res->status = 0;
    fprintf(stderr, "rom_fill: instance %u: %zu accesses, %u lanes (window %zu), sort %.1f values %.1f rows+out %.1f ms\n",
            inst, n, n_lanes_inst, lane_from, ms(0, 1), ms(1, 2), ms(2, 3));
    for (auto& e : ev) cudaEventDestroy(e);
    return true;
}

bool CountAndPlan::fill_all_rom_instances(uint32_t n_rows, RamFillPrepared* prepared) {
    if (prepared) *prepared = RamFillPrepared{};
    if (!prepared) return false;
    cudaSetDevice(gpu_device_);
    if (!ram_retention_enabled_.load(std::memory_order_relaxed) || rom_n_cols_ == 0) { prepared->status = -1; return false; }
    const uint32_t n_inst = num_inst_[RF_REGION_ROM];
    if (rom_prepared_) {
        prepared->status = 0; prepared->n_instances = n_inst; prepared->n_lanes = region_n_ops_[RF_REGION_ROM];
        return true;
    }
    if (!prepare_other_index_() || other_total_ == 0 || n_inst == 0) { prepared->status = -1; return false; }
    prepared->n_accesses = other_total_;
    if (!other_geometry_(RF_REGION_ROM, h_rom_inst_first_, h_rom_inst_last_, h_rom_inst_skip_, h_rom_inst_count_)) {
        prepared->status = -1;
        return false;
    }
    const size_t stride = (size_t)n_rows * rom_words_per_row_;
    const size_t need = stride * n_inst;
    if (need > h_rom_rows_cap_) {
        if (h_rom_rows_) { cudaFreeHost(h_rom_rows_); h_rom_rows_ = nullptr; h_rom_rows_cap_ = 0; }
        if (cudaMallocHost(&h_rom_rows_, need * 8) != cudaSuccess) {
            fprintf(stderr, "rom_fill: pinned allocation of %zu MB for the instance rows failed\n", (need * 8) >> 20);
            h_rom_rows_ = nullptr;
            return false;
        }
        h_rom_rows_cap_ = need;
    }
    rom_rows_stride_ = stride;
    rom_results_.assign(n_inst, RamFillResult{});
    for (uint32_t i = 0; i < n_inst; ++i) {
        if (!fill_rom_instance(i, h_rom_rows_ + (size_t)i * stride, n_rows, &rom_results_[i])) {
            prepared->status = rom_results_[i].status;
            return false;
        }
    }
    rom_prepared_ = true;
    prepared->status = 0;
    prepared->n_instances = n_inst;
    prepared->n_lanes = region_n_ops_[RF_REGION_ROM];
    return true;
}

const uint64_t* CountAndPlan::rom_instance_rows(uint32_t inst, RamFillResult* res) const {
    if (inst >= rom_results_.size() || h_rom_rows_ == nullptr) return nullptr;
    if (res) *res = rom_results_[inst];
    return h_rom_rows_ + (size_t)inst * rom_rows_stride_;
}

// ─── InputData: the input accesses retained in arrival order, per instance ──────────────────────
//
// An InputData lane is one read: address, step, the word's value, sel 1, addr_changes on the first
// lane of each word, is_free_read on the free-input word (the region's first word). A read's value
// is the word of the input image, which the guest sees unchanged; the free-input word is written
// by the emulator per read and its reads carry the value in the stream. The first lane of the
// block changes address unless it is the free-input word. Padding lanes repeat the last lane with
// sel 0, addr_changes 0.

constexpr int      RF_REGION_INPUT = 1;
constexpr uint32_t INPUT_W_ADDR_BASE = ZISK_INPUT_ADDR_BASE >> 3;

bool CountAndPlan::set_input_layout(const uint32_t* col_widths, uint32_t n_cols, uint32_t words_per_row,
                                    uint32_t lanes_x_row) {
    RowPackLayout l;
    if (lanes_x_row == 0 || n_cols != 9 * lanes_x_row || !rowpack_layout(l, col_widths, n_cols, words_per_row))
        return false;
    for (uint32_t c = 0; c < n_cols; ++c) input_col_widths_[c] = col_widths[c];
    input_n_cols_ = n_cols;
    input_words_per_row_ = words_per_row;
    input_lanes_x_row_ = lanes_x_row;
    return true;
}

// Column index of an InputData lane's field: groups addr, step, addr_changes, sel, value_word[4],
// is_free_read, each with one column per lane (four for value_word), in the trace's declaration order.
__device__ __forceinline__ uint32_t input_col(uint32_t lanes, uint32_t group, uint32_t lane, uint32_t idx) {
    return group < 4 ? group * lanes + lane : group == 4 ? 4 * lanes + lane * 4 + idx : 8 * lanes + lane;
}

__host__ __device__ __forceinline__ uint64_t input_word(const uint64_t* image, size_t image_words, uint32_t off) {
    return off < image_words ? image[off] : 0;
}

__global__ void input_rows_kernel(RowPackLayout layout, uint32_t lanes_x_row, size_t lane_from, uint32_t n_lanes_inst,
                                  uint32_t n_rows, const uint32_t* __restrict__ addr_sorted,
                                  const uint32_t* __restrict__ idx, RamRecords rec,
                                  const uint64_t* __restrict__ image, size_t image_words, uint32_t prev_addr,
                                  uint64_t* __restrict__ out_rows) {
    const uint32_t r = blockIdx.x * blockDim.x + threadIdx.x;
    if (r >= n_rows) return;
    uint64_t w[MEMPACK_MAX_WORDS_PER_ROW];
#pragma unroll
    for (uint32_t k = 0; k < MEMPACK_MAX_WORDS_PER_ROW; ++k) w[k] = 0;
    const uint32_t base = REGION_ADDR_START[RF_REGION_INPUT];
    const size_t g_last = lane_from + n_lanes_inst - 1;
    const uint32_t last_off = addr_sorted[g_last] - base;
    const uint32_t k_last = idx[g_last];
    const uint64_t m_last = rec.meta(k_last);
    const uint64_t last_step = rf_step(m_last);
    const uint64_t last_value = rf_kind(m_last) == RF_KIND_READ ? input_word(image, image_words, last_off) : rec.value(k_last);
    for (uint32_t lane = 0; lane < lanes_x_row; ++lane) {
        const uint32_t v = r * lanes_x_row + lane;
        uint32_t off; uint64_t step, value; bool change, sel;
        if (v < n_lanes_inst) {
            const size_t g = lane_from + v;
            const uint32_t k = idx[g];
            const uint64_t m = rec.meta(k);
            const uint32_t a = addr_sorted[g];
            off = a - base;
            step = rf_step(m);
            value = rf_kind(m) == RF_KIND_READ ? input_word(image, image_words, off) : rec.value(k);
            change = (g == 0 ? prev_addr : addr_sorted[g - 1]) != a;
            sel = true;
        } else {
            off = last_off; step = last_step; value = last_value; change = false; sel = false;
        }
        rowpack_put(layout, w, input_col(lanes_x_row, 0, lane, 0), INPUT_W_ADDR_BASE + off);
        rowpack_put(layout, w, input_col(lanes_x_row, 1, lane, 0), step);
        rowpack_put(layout, w, input_col(lanes_x_row, 2, lane, 0), change ? 1 : 0);
        rowpack_put(layout, w, input_col(lanes_x_row, 3, lane, 0), sel ? 1 : 0);
#pragma unroll
        for (uint32_t i = 0; i < 4; ++i)
            rowpack_put(layout, w, input_col(lanes_x_row, 4, lane, i), (value >> (16 * i)) & 0xFFFFu);
        rowpack_put(layout, w, input_col(lanes_x_row, 5, lane, 0), off == 0 ? 1 : 0);
    }
    uint64_t* dst = out_rows + (size_t)r * layout.words_per_row;
    for (uint32_t k = 0; k < layout.words_per_row; ++k) dst[k] = w[k];
}

bool CountAndPlan::fill_input_instance(uint32_t inst, const uint64_t* d_image, const uint64_t* h_image, size_t image_words,
                                       uint8_t* scratch, uint64_t* out_rows, uint32_t n_rows, RamFillResult* res) {
    if (res) *res = RamFillResult{};
    if (!res || !out_rows) return false;
    const size_t R = instance_rows_[RF_REGION_INPUT];
    if ((size_t)n_rows * input_lanes_x_row_ != R) {
        fprintf(stderr, "input_fill: instance has %zu lanes, the caller's trace %u rows x %u lanes\n", R, n_rows, input_lanes_x_row_);
        res->status = -2;
        return false;
    }
    const uint32_t n_inst = num_inst_[RF_REGION_INPUT];
    if (inst >= n_inst || other_total_ == 0) { res->status = -3; return false; }
    const size_t lane_from = h_input_inst_skip_[inst];
    const uint32_t n_lanes_inst = (uint32_t)std::min(R, (size_t)region_n_ops_[RF_REGION_INPUT] - (size_t)inst * R);
    const size_t n_total = ram_cursor_.load(std::memory_order_relaxed);

    cudaEvent_t ev[3];
    for (auto& e : ev) RF_TRY(cudaEventCreate(&e));
    RF_TRY(cudaEventRecord(ev[0]));

    ScratchCursor sc{scratch};
    uint8_t* end = arena_ + (ram_low_edge_bytes(n_total) & ~(size_t)255);
    uint32_t* addr_sorted; uint32_t* idx; size_t n;
    if (!other_sorted_(sc, h_input_inst_first_[inst], h_input_inst_last_[inst], h_input_inst_count_[inst], &addr_sorted, &idx, &n)
        || lane_from + n_lanes_inst > n) {
        res->status = -4;
        return false;
    }
    uint64_t* rows = (uint64_t*)sc.take((size_t)n_rows * input_words_per_row_ * 8);
    if (sc.cur > end) {
        fprintf(stderr, "input_fill: instance %u needs %zu MB of scratch, %zu MB free below the retained accesses\n",
                inst, (size_t)(sc.cur - scratch) >> 20, (size_t)(end - scratch) >> 20);
        res->status = -3;
        return false;
    }
    RF_TRY(cudaEventRecord(ev[1]));

    const uint32_t prev_addr = inst == 0 ? REGION_ADDR_START[RF_REGION_INPUT] : 0xFFFFFFFFu;
    RowPackLayout layout{};
    rowpack_layout(layout, input_col_widths_, input_n_cols_, input_words_per_row_);
    input_rows_kernel<<<(n_rows + RF_BLOCK - 1) / RF_BLOCK, RF_BLOCK>>>(
        layout, input_lanes_x_row_, lane_from, n_lanes_inst, n_rows, addr_sorted, idx, ram_records_,
        d_image, image_words, prev_addr, rows);
    RF_TRY(cudaGetLastError());
    RF_TRY(cudaMemcpy(out_rows, rows, (size_t)n_rows * input_words_per_row_ * 8, cudaMemcpyDeviceToHost));
    RF_TRY(cudaEventRecord(ev[2]));
    RF_TRY(cudaDeviceSynchronize());

    // The last lane; the lane before the instance is the previous instance's last.
    {
        const size_t g = lane_from + n_lanes_inst - 1;
        uint32_t a = 0, k = 0;
        RF_TRY(cudaMemcpy(&a, addr_sorted + g, 4, cudaMemcpyDeviceToHost));
        RF_TRY(cudaMemcpy(&k, idx + g, 4, cudaMemcpyDeviceToHost));
        uint32_t r[RAM_RECORD_WORDS];
        RF_TRY(cudaMemcpy(r, ram_records_.rec(k), RAM_RECORD_WORDS * 4, cudaMemcpyDeviceToHost));
        const uint64_t m = r[1] | ((uint64_t)r[2] << 32);
        const uint32_t off = a - REGION_ADDR_START[RF_REGION_INPUT];
        res->last_addr_w = INPUT_W_ADDR_BASE + off;
        res->last_step = m & RAM_META_STEP_MASK;
        res->last_value = ((m >> RAM_META_KIND_SHIFT) & 3u) == RF_KIND_READ ? input_word(h_image, image_words, off)
                                                                            : (r[3] | ((uint64_t)r[4] << 32));
    }
    if (inst > 0) {
        const RamFillResult& p = input_results_[inst - 1];
        res->prev_addr_w = p.last_addr_w; res->prev_step = p.last_step; res->prev_value = p.last_value;
    } else {
        res->prev_addr_w = INPUT_W_ADDR_BASE; res->prev_step = 0; res->prev_value = 0;
    }
    res->n_lanes = n_lanes_inst;
    auto ms = [&](int a, int b) { float t = 0; cudaEventElapsedTime(&t, ev[a], ev[b]); return t; };
    res->ms_rows = ms(1, 2);
    res->ms_d2h = 0;
    res->status = 0;
    fprintf(stderr, "input_fill: instance %u: %zu accesses, %u lanes (window %zu), sort %.1f rows+out %.1f ms\n",
            inst, n, n_lanes_inst, lane_from, ms(0, 1), ms(1, 2));
    for (auto& e : ev) cudaEventDestroy(e);
    return true;
}

bool CountAndPlan::fill_all_input_instances(uint32_t n_rows, const void* image, size_t image_bytes, RamFillPrepared* prepared) {
    if (prepared) *prepared = RamFillPrepared{};
    if (!prepared) return false;
    cudaSetDevice(gpu_device_);
    if (!ram_retention_enabled_.load(std::memory_order_relaxed) || input_n_cols_ == 0) { prepared->status = -1; return false; }
    const uint32_t n_inst = num_inst_[RF_REGION_INPUT];
    if (input_prepared_) {
        prepared->status = 0; prepared->n_instances = n_inst; prepared->n_lanes = region_n_ops_[RF_REGION_INPUT];
        return true;
    }
    if (!prepare_other_index_() || other_total_ == 0 || n_inst == 0) { prepared->status = -1; return false; }
    prepared->n_accesses = other_total_;
    if (!other_geometry_(RF_REGION_INPUT, h_input_inst_first_, h_input_inst_last_, h_input_inst_skip_, h_input_inst_count_)) {
        prepared->status = -1;
        return false;
    }
    // The input image, whole words, ahead of the per-instance scratch.
    const size_t image_words = (image_bytes + 7) / 8;
    std::vector<uint64_t> h_image(image_words, 0);
    memcpy(h_image.data(), image, image_bytes);
    ScratchCursor sc{other_scratch_};
    uint64_t* d_image = (uint64_t*)sc.take(image_words * 8);
    if (sc.cur > arena_ + ram_low_edge_bytes(ram_cursor_.load(std::memory_order_relaxed))) {
        fprintf(stderr, "input_fill: no room for the %zu MB input image below the retained accesses\n", (image_words * 8) >> 20);
        prepared->status = -3;
        return false;
    }
    RF_TRY(cudaMemcpy(d_image, h_image.data(), image_words * 8, cudaMemcpyHostToDevice));
    const size_t stride = (size_t)n_rows * input_words_per_row_;
    const size_t need = stride * n_inst;
    if (need > h_input_rows_cap_) {
        if (h_input_rows_) { cudaFreeHost(h_input_rows_); h_input_rows_ = nullptr; h_input_rows_cap_ = 0; }
        if (cudaMallocHost(&h_input_rows_, need * 8) != cudaSuccess) {
            fprintf(stderr, "input_fill: pinned allocation of %zu MB for the instance rows failed\n", (need * 8) >> 20);
            h_input_rows_ = nullptr;
            return false;
        }
        h_input_rows_cap_ = need;
    }
    input_rows_stride_ = stride;
    input_results_.assign(n_inst, RamFillResult{});
    for (uint32_t i = 0; i < n_inst; ++i) {
        if (!fill_input_instance(i, d_image, h_image.data(), image_words, sc.cur, h_input_rows_ + (size_t)i * stride, n_rows,
                                 &input_results_[i])) {
            prepared->status = input_results_[i].status;
            return false;
        }
    }
    input_prepared_ = true;
    prepared->status = 0;
    prepared->n_instances = n_inst;
    prepared->n_lanes = region_n_ops_[RF_REGION_INPUT];
    return true;
}

const uint64_t* CountAndPlan::input_instance_rows(uint32_t inst, RamFillResult* res) const {
    if (inst >= input_results_.size() || h_input_rows_ == nullptr) return nullptr;
    if (res) *res = input_results_[inst];
    return h_input_rows_ + (size_t)inst * input_rows_stride_;
}

