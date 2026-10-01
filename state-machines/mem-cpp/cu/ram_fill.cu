// The RAM witness from the accesses the planner retained (see count_and_plan.cuh).
//
// prepare_ram_fill, once per block after run():
//   1. sort   — keys (compact address << 29 | arrival index): the sorted order is the lane order
//               once dual pairs merge, since lanes go by address and, inside one, by time.
//   2. lanes  — dual pairing as a scan: a read pairs into the lane before it when that lane is
//               open (created by the previous access, not yet paired), on the same address and in
//               the same chunk; within a run of pairable reads the lanes alternate open/paired, so
//               emit = parity of the distance to the last non-pairable access.
//   3. values — every read's value is what the writes before it on that address left: a scan by
//               address of byte-masked merges, run in blocks with a carry so the scratch stays
//               bounded. Block writes whose value the stream does not carry are counted; the rows
//               that depend on them are wrong and the caller's check reports them.
// fill_ram_instance packs one instance's lanes; the CPU `fill_mem_range` is the oracle.

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
                                   uint32_t* __restrict__ keys, unsigned long long* __restrict__ unresolved) {
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
        if (kind == 3) atomicAdd(unresolved, 1ull);
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
                               bool first_instance, const uint32_t* __restrict__ lane_first,
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

    for (uint32_t lane = 0; lane < L; ++lane) {
        const uint32_t v = r * L + lane;
        if (v < n_lanes_inst) {
            const size_t g = lane_from + v;
            const MemPackLane me = rf_lane(g, lane_first, sidx, emit, n_sorted, skeys, resolved);
            bool addr_changes;
            uint64_t inc;
            if (g == 0) {
                // No previous lane at all: the segment starts the memory cycle at the RAM base.
                addr_changes = true;
                inc = mempack_increment(true, me, RAM_W_ADDR_BASE - 1, 0);
            } else {
                const MemPackLane pv = rf_lane(g - 1, lane_first, sidx, emit, n_sorted, skeys, resolved);
                addr_changes = pv.addr != me.addr;
                inc = mempack_increment(addr_changes, me, pv.addr, pv.dual ? pv.step_dual : pv.step);
            }
            (void)first_instance;
            mempack_lane(layout, w, lane, me, addr_changes, inc);
        } else {
            mempack_padding(layout, w, lane, last.addr, pad_step, last.value);
        }
    }
    uint64_t* dst = out_rows + (size_t)r * layout.words_per_row;
    for (uint32_t k = 0; k < layout.words_per_row; ++k) dst[k] = w[k];
}

}  // namespace

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
    if (ram_prepared_) {
        out->status = 0;
        out->n_lanes = ram_n_lanes_;
        out->unresolved_writes = ram_unresolved_;
        out->n_instances = (uint32_t)((ram_n_lanes_ + instance_rows_[RF_REGION_RAM] - 1) / instance_rows_[RF_REGION_RAM]);
        return true;
    }
    cudaEvent_t ev[5];
    for (auto& e : ev) RF_TRY(cudaEventCreate(&e));
    RF_TRY(cudaEventRecord(ev[0]));

    // Scratch: the arena below the retained accesses. Every device structure of the plan is dead
    // once run() returned (metas, offset pages and align counters are on the host) until reset()
    // starts the next block. Carve: two key and two index buffers that the sort ping-pongs
    // between (the sorted keys stay: they give every lane its address and step; the sorted
    // indexes are sidx; the other two become anchors, emit and the resolved values), lane_first,
    // the propagation block, one instance's rows, cub temp: 28 bytes per access.
    uint8_t* cur = arena_;
    uint8_t* end = arena_ + (ram_low_edge_bytes(n) & ~(size_t)255);
    auto take = [&](size_t bytes) -> uint8_t* {
        uint8_t* p = (uint8_t*)(((uintptr_t)cur + 255) & ~(uintptr_t)255);
        cur = p + bytes;
        return p;
    };
    cub::DoubleBuffer<uint64_t> dkeys((uint64_t*)take(n * 8), (uint64_t*)take(n * 8));
    cub::DoubleBuffer<uint32_t> didx((uint32_t*)take(n * 4), (uint32_t*)take(n * 4));
    uint32_t* lfirst   = (uint32_t*)take(n * 4);
    uint32_t* d_n_lanes = (uint32_t*)take(4);
    Merge*    prop_in  = (Merge*)take(RF_PROP_BLOCK * sizeof(Merge));
    Merge*    prop_out = (Merge*)take(RF_PROP_BLOCK * sizeof(Merge));
    uint32_t* prop_keys = (uint32_t*)take(RF_PROP_BLOCK * 4);
    unsigned long long* d_unresolved = (unsigned long long*)take(8);
    const uint32_t n_rows = instance_rows_[RF_REGION_RAM] / mem_lanes_x_row_;
    uint64_t* rows = (uint64_t*)take((size_t)n_rows * mem_words_per_row_ * 8);
    size_t t_sort = 0, t_max = 0, t_sum = 0, t_bykey = 0, t_select = 0;
    cub::DeviceRadixSort::SortPairs(nullptr, t_sort, dkeys, didx, n, 0, (int)(RF_STEP_BITS + RF_ADDR_BITS));
    cub::DeviceScan::InclusiveScan(nullptr, t_max, (uint32_t*)nullptr, (uint32_t*)nullptr, MaxU32Op(), n);
    cub::DeviceScan::ExclusiveSum(nullptr, t_sum, (uint32_t*)nullptr, (uint32_t*)nullptr, n);
    cub::DeviceSelect::Flagged(nullptr, t_select, thrust::counting_iterator<uint32_t>(0),
                               (uint32_t*)nullptr, (uint32_t*)nullptr, (uint32_t*)nullptr, n);
    cub::DeviceScan::InclusiveScanByKey(nullptr, t_bykey, prop_keys, prop_in, prop_out, MergeOp(),
                                        RF_PROP_BLOCK, EqU32());
    const size_t t_bytes = std::max(std::max(std::max(t_sort, t_max), std::max(t_sum, t_bykey)), t_select);
    void* temp = take(t_bytes);
    if (cur > end) {
        fprintf(stderr, "ram_fill: scratch needs %zu MB, the arena has %zu MB below the retained accesses; RAM witness off\n",
                (size_t)(cur - arena_) >> 20, (size_t)(end - arena_) >> 20);
        out->status = -3;
        return false;
    }
    fprintf(stderr, "ram_fill: %zu accesses, scratch %zu MB (%.1f bytes per access)\n", n,
            (size_t)(cur - arena_) >> 20, (double)(cur - arena_) / (double)n);
    RF_TRY(cudaMemcpy(&ram_writes_, d_ram_nwrites_, 8, cudaMemcpyDeviceToHost));
    RF_TRY(cudaEventRecord(ev[4]));

    // 1. sort by (address, step), stable
    rf_keys_kernel<<<rf_grid(n), RF_BLOCK>>>(ram_records_, n, dkeys.Current(), didx.Current());
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
    // lane_first: the sorted positions that start a lane, in order.
    tb = t_bytes;
    RF_TRY(cub::DeviceSelect::Flagged(temp, tb, thrust::counting_iterator<uint32_t>(0), emit, lfirst,
                                      d_n_lanes, n));
    uint32_t n_lanes32 = 0;
    RF_TRY(cudaMemcpy(&n_lanes32, d_n_lanes, 4, cudaMemcpyDeviceToHost));
    const size_t n_lanes = n_lanes32;
    RF_TRY(cudaEventRecord(ev[2]));

    // 3. values, in blocks with a carry. The free key buffer takes the resolved values.
    uint64_t* resolved = dkeys.Alternate();
    RF_TRY(cudaMemset(d_unresolved, 0, 8));
    Merge carry{0, 0};
    uint32_t carry_addr = 0xFFFFFFFFu;
    for (size_t j0 = 0; j0 < n; j0 += RF_PROP_BLOCK) {
        const size_t len = std::min(RF_PROP_BLOCK, n - j0);
        rf_merge_in_kernel<<<rf_grid(len), RF_BLOCK>>>(sidx, keys_out, ram_records_, j0, len,
                                                       prop_in, prop_keys, d_unresolved);
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
    unsigned long long unresolved = 0;
    RF_TRY(cudaMemcpy(&unresolved, d_unresolved, 8, cudaMemcpyDeviceToHost));
    RF_TRY(cudaEventRecord(ev[3]));
    RF_TRY(cudaDeviceSynchronize());

    d_lane_sidx_ = sidx;
    d_lane_emit_ = emit;
    d_lane_first_ = lfirst;
    d_lane_value_ = resolved;
    d_lane_keys_ = keys_out;
    d_rows_scratch_ = rows;
    ram_n_lanes_ = n_lanes;
    ram_n_sorted_ = n;
    ram_unresolved_ = unresolved;
    ram_prepared_ = true;

    if (n_lanes != region_n_ops_[RF_REGION_RAM]) {
        fprintf(stderr, "ram_fill: %zu lanes from the retained accesses, the plan counted %u\n",
                n_lanes, region_n_ops_[RF_REGION_RAM]);
    }
    fprintf(stderr, "ram_fill: %zu accesses retained on the device (%zu MB), %llu writes (%.1f%%), pool used %zu MB\n",
            n, n * 20 >> 20, (unsigned long long)ram_writes_, n ? 100.0 * (double)ram_writes_ / (double)n : 0.0,
            pool_cursor_u32_.load(std::memory_order_relaxed) * 4 >> 20);
    auto ms = [&](int a, int b) { float t = 0; cudaEventElapsedTime(&t, ev[a], ev[b]); return t; };
    out->status = 0;
    out->n_lanes = n_lanes;
    out->unresolved_writes = unresolved;
    out->n_instances = (uint32_t)((n_lanes + instance_rows_[RF_REGION_RAM] - 1) / instance_rows_[RF_REGION_RAM]);
    out->ms_sort = ms(4, 1);
    out->ms_lanes = ms(1, 2);
    out->ms_values = ms(2, 3);
    out->ms_total = ms(0, 3);
    for (auto& e : ev) cudaEventDestroy(e);
    return true;
}

bool CountAndPlan::fill_ram_instance(uint32_t inst, uint64_t* out_rows, uint32_t n_rows, RamFillResult* res) {
    if (res) *res = RamFillResult{};
    if (!res || !out_rows) return false;
    cudaSetDevice(gpu_device_);
    if (!ram_prepared_) { res->status = -1; return false; }
    const size_t R = instance_rows_[RF_REGION_RAM];
    if ((size_t)n_rows * mem_lanes_x_row_ != R) {
        fprintf(stderr, "ram_fill: instance has %zu lanes, the caller's trace %u rows x %u lanes\n",
                R, n_rows, mem_lanes_x_row_);
        res->status = -2;
        return false;
    }
    const size_t lane_from = (size_t)inst * R;
    if (lane_from >= ram_n_lanes_) { res->status = -3; return false; }
    const uint32_t n_lanes_inst = (uint32_t)std::min(R, ram_n_lanes_ - lane_from);

    MemPackLayout layout{};
    mempack_layout(layout, mem_col_widths_, mem_n_cols_, mem_words_per_row_, mem_lanes_x_row_);

    cudaEvent_t ev[3];
    for (auto& e : ev) RF_TRY(cudaEventCreate(&e));
    RF_TRY(cudaEventRecord(ev[0]));
    rf_rows_kernel<<<(n_rows + RF_BLOCK - 1) / RF_BLOCK, RF_BLOCK>>>(
        layout, lane_from, n_lanes_inst, n_rows, inst == 0, d_lane_first_, d_lane_sidx_, d_lane_emit_,
        ram_n_sorted_, d_lane_keys_, d_lane_value_, d_rows_scratch_);
    RF_TRY(cudaGetLastError());
    RF_TRY(cudaEventRecord(ev[1]));
    RF_TRY(cudaMemcpy(out_rows, d_rows_scratch_, (size_t)n_rows * mem_words_per_row_ * 8, cudaMemcpyDeviceToHost));
    RF_TRY(cudaEventRecord(ev[2]));
    RF_TRY(cudaDeviceSynchronize());

    // Continuation scalars: the lane before the instance and its last lane.
    auto lane_scalars = [&](size_t g, uint32_t& addr_w, uint64_t& step, uint64_t& value) -> bool {
        uint32_t j = 0, i = 0, e_next = 1, i_next = 0;
        RF_TRY(cudaMemcpy(&j, d_lane_first_ + g, 4, cudaMemcpyDeviceToHost));
        (void)i; (void)i_next;
        uint64_t k = 0;
        RF_TRY(cudaMemcpy(&k, d_lane_keys_ + j, 8, cudaMemcpyDeviceToHost));
        RF_TRY(cudaMemcpy(&value, d_lane_value_ + j, 8, cudaMemcpyDeviceToHost));
        addr_w = (uint32_t)(k >> RF_STEP_BITS) + RAM_W_ADDR_BASE;
        step = k & RF_STEP_MASK;
        if ((size_t)j + 1 < ram_n_sorted_) {
            RF_TRY(cudaMemcpy(&e_next, d_lane_emit_ + j + 1, 4, cudaMemcpyDeviceToHost));
            if (e_next == 0) {
                uint64_t k2 = 0;
                RF_TRY(cudaMemcpy(&k2, d_lane_keys_ + j + 1, 8, cudaMemcpyDeviceToHost));
                step = k2 & RF_STEP_MASK;
            }
        }
        return true;
    };
    if (lane_from > 0) {
        if (!lane_scalars(lane_from - 1, res->prev_addr_w, res->prev_step, res->prev_value)) return false;
    } else {
        res->prev_addr_w = RAM_W_ADDR_BASE;
        res->prev_step = 0;
        res->prev_value = 0;
    }
    if (!lane_scalars(lane_from + n_lanes_inst - 1, res->last_addr_w, res->last_step, res->last_value)) return false;
    res->n_lanes = n_lanes_inst;
    auto ms = [&](int a, int b) { float t = 0; cudaEventElapsedTime(&t, ev[a], ev[b]); return t; };
    res->ms_rows = ms(0, 1);
    res->ms_d2h = ms(1, 2);
    res->status = 0;
    for (auto& e : ev) cudaEventDestroy(e);
    return true;
}

bool CountAndPlan::fill_all_ram_instances(uint32_t n_rows, RamFillPrepared* prepared) {
    if (!prepare_ram_fill(prepared)) return false;
    const uint32_t n_inst = prepared->n_instances;
    const size_t stride = (size_t)n_rows * mem_words_per_row_;
    const size_t need = stride * n_inst;
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
    for (uint32_t i = 0; i < n_inst; ++i) {
        if (!fill_ram_instance(i, h_ram_rows_ + (size_t)i * stride, n_rows, &ram_results_[i])) return false;
    }
    return true;
}

const uint64_t* CountAndPlan::ram_instance_rows(uint32_t inst, RamFillResult* res) const {
    if (inst >= ram_results_.size() || h_ram_rows_ == nullptr) return nullptr;
    if (res) *res = ram_results_[inst];
    return h_ram_rows_ + (size_t)inst * ram_rows_stride_;
}
