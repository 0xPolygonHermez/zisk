#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <algorithm>
#include <iomanip>
#include <iostream>
#include <omp.h>
#include <random>
#include <span>
#include <vector>
#include <cub/device/device_radix_sort.cuh>
#include <cub/device/device_scan.cuh>

// =====================================================================
// Constants
// =====================================================================

constexpr uint32_t N_ADDR_ROM   = 1u << 24;   // 16M
constexpr uint32_t N_ADDR_INPUT = 1u << 24;   // 16M
constexpr uint32_t N_ADDR_RAM   = 1u << 26;   // 64M
constexpr uint32_t N_ADDR = N_ADDR_ROM + N_ADDR_INPUT + N_ADDR_RAM;  // 96M

constexpr uint32_t INSTANCE_SIZE = 1u << 22;  // 4M entries per instance

constexpr uint32_t MAX_INST_ROM   = 32;
constexpr uint32_t MAX_INST_INPUT = 32;
constexpr uint32_t MAX_INST_RAM   = 256;
constexpr uint32_t MAX_INSTANCES  = MAX_INST_ROM + MAX_INST_INPUT + MAX_INST_RAM;
constexpr uint32_t MAX_INST[3]    = {MAX_INST_ROM, MAX_INST_INPUT, MAX_INST_RAM};
constexpr uint32_t MASK_WORDS     = (MAX_INSTANCES + 31) / 32;

constexpr uint32_t MAX_OPS    = (uint32_t)MAX_INSTANCES * INSTANCE_SIZE;
constexpr uint32_t N_WORKERS  = 16;
constexpr uint32_t MAX_ACTIVE = (MAX_INSTANCES + N_WORKERS - 1) / N_WORKERS;
constexpr uint32_t MAX_CHUNKS = 4096;
constexpr int      N_STREAMS  = 4;

// Region identifiers
constexpr uint8_t REGION_ROM            = 0;
constexpr uint8_t REGION_INPUT          = 1;
constexpr uint8_t REGION_RAM            = 2;
constexpr const char* REGION_NAME[3]    = {"ROM", "INPUT", "RAM"};
constexpr uint32_t REGION_ADDR_START[3] = {0, N_ADDR_ROM, N_ADDR_ROM + N_ADDR_INPUT};
constexpr uint32_t REGION_N_ADDR[3]     = {N_ADDR_ROM, N_ADDR_INPUT, N_ADDR_RAM};

// =====================================================================
// Helper Functions
// =====================================================================

// Maps raw hardware address to compact address space
inline uint32_t compact_addr(uint32_t raw) {
    if (raw >= 0xA0000000u)
        return ((raw - 0xA0000000u) >> 3) + N_ADDR_ROM + N_ADDR_INPUT;
    if (raw >= 0x90000000u)
        return ((raw - 0x90000000u) >> 3) + N_ADDR_ROM;
    return (raw - 0x80000000u) >> 3;
}

// Maps compact address back to raw hardware address
inline uint32_t expand_addr(uint32_t compact) {
    if (compact >= N_ADDR_ROM + N_ADDR_INPUT)
        return ((compact - N_ADDR_ROM - N_ADDR_INPUT) << 3) + 0xA0000000u;
    if (compact >= N_ADDR_ROM)
        return ((compact - N_ADDR_ROM) << 3) + 0x90000000u;
    return (compact << 3) + 0x80000000u;
}

// =====================================================================
// GPU Kernels
// =====================================================================

// Converts raw hardware addresses to compact address space in-place and
// accumulates a histogram of compact addresses. Can be called incrementally
// on different slices of ops — hist is accumulated across calls via atomicAdd,
// so the caller must zero it before the first call.
//
// In/Out: ops[num_ops]       — raw addresses on entry, compact addresses on exit
// In/Out: hist[N_ADDR]       — histogram accumulated in-place
// Input:  num_ops            — number of operations in this slice
__global__ void shift_and_histogram_kernel(uint32_t* ops, uint32_t* hist, uint32_t num_ops) {
    uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    uint32_t stride = gridDim.x * blockDim.x;
    for (; i < num_ops; i += stride) {
        uint32_t addr = ops[i];
        uint32_t compact;
        if (addr >= 0xA0000000u)
            compact = ((addr - 0xA0000000u) >> 3) + N_ADDR_ROM + N_ADDR_INPUT;
        else if (addr >= 0x90000000u)
            compact = ((addr - 0x90000000u) >> 3) + N_ADDR_ROM;
        else
            compact = (addr - 0x80000000u) >> 3;
        ops[i] = compact;
        atomicAdd(&hist[compact], 1);
    }
}

// Finds first/last compact addresses for each active instance via binary
// search on the prefix-sum array. Also computes offset_starts — packing offsets
// used later by compute_addr_offsets_kernel to store per-address write positions
// contiguously across all instances.
//
// Pointers marked (*) are pre-offset by the caller for this region's slice of
// the global arrays (e.g., caller passes d_active_ids + active_offset[region]).
//
// Input:  prefix[N_ADDR+1]               — exclusive prefix sum of histogram
// Input:  prefix_base_addr               — first compact address of this region
// Input:  num_addr_region                — number of compact addresses in this region
// Input:  num_ops_region                 — total ops in this region
// Input:  active_ids[num_active]     (*) — local instance IDs of active instances
// Output: active_first[num_active]   (*) — first compact address per active instance
// Output: active_last[num_active]    (*) — last compact address per active instance
// Output: offset_starts[num_active]  (*) — start index per instance into packed addr_offsets
// Input:  num_active                     — number of active instances in this region
__global__ void instance_boundaries_kernel(
    const uint32_t* prefix,
    uint32_t prefix_base_addr, uint32_t num_addr_region,
    uint32_t num_ops_region,
    const uint32_t* active_ids,
    uint32_t* active_first, uint32_t* active_last,
    uint32_t* offset_starts,
    uint32_t num_active)
{
    uint32_t idx = threadIdx.x;
    if (idx >= num_active) return;

    uint32_t local_inst  = active_ids[idx];
    uint32_t region_start = prefix[prefix_base_addr];
    uint32_t base_pos    = region_start + local_inst * INSTANCE_SIZE;
    uint32_t inst_size   = min(INSTANCE_SIZE, num_ops_region - local_inst * INSTANCE_SIZE);
    uint32_t inst_start  = (local_inst == 0) ? base_pos : base_pos - 1;
    uint32_t inst_end    = base_pos + inst_size;

    // Binary search: largest address with prefix[addr] <= inst_start
    uint32_t lo = prefix_base_addr, hi = prefix_base_addr + num_addr_region;
    while (lo < hi) {
        uint32_t mid = lo + (hi - lo + 1) / 2;
        if (prefix[mid] <= inst_start) lo = mid;
        else hi = mid - 1;
    }
    active_first[idx] = lo;

    // Binary search: largest address with prefix[addr] < inst_end
    lo = active_first[idx];
    hi = prefix_base_addr + num_addr_region;
    while (lo < hi) {
        uint32_t mid = lo + (hi - lo + 1) / 2;
        if (prefix[mid] < inst_end) lo = mid;
        else hi = mid - 1;
    }

    // Trim trailing addresses with 0 ops: find largest addr with prefix[addr] < prefix[lo+1]
    // (i.e. last address that actually has histogram entries)
    {
        uint32_t max_prefix = prefix[lo + 1];
        uint32_t tlo = active_first[idx], thi = lo;
        while (tlo < thi) {
            uint32_t mid = tlo + (thi - tlo + 1) / 2;
            if (prefix[mid] < max_prefix) tlo = mid;
            else thi = mid - 1;
        }
        lo = tlo;
    }
    active_last[idx] = lo;

    // Thread 0 computes offset_starts (serial prefix sum over address counts)
    // here we assume we launch one single block
    __syncthreads();
    if (idx == 0) {
        uint32_t offset = 0;
        for (uint32_t i = 0; i < num_active; i++) {
            offset_starts[i] = offset;
            offset += active_last[i] - active_first[i] + 1;
        }
    }
}

// Counts how many ops in each chunk hit the first, middle, or last compact
// address of each active instance. Uses warp-level ballot for efficiency.
//
// Input:  ops[num_ops]                          — compact addresses (sorted)
// Input:  active_first[num_active]              — first compact address per instance (all regions)
// Input:  active_last[num_active]               — last compact address per instance (all regions)
// Input:  chunk_offsets[num_chunks+1]           — start position of each chunk
// Output: d_fml[num_active * num_chunks * 3]    — [ai][chunk][0/1/2] = first/middle/last counts
// Input:  num_active, num_chunks, num_ops       — dimensions
__global__ void chunk_fml_count_kernel(
    const uint32_t* ops,
    const uint32_t* active_first, const uint32_t* active_last,
    const uint32_t* chunk_offsets,
    uint32_t* d_fml,
    uint32_t num_active, uint32_t num_chunks, uint32_t num_ops)
{
    __shared__ uint32_t s_first[MAX_ACTIVE];
    __shared__ uint32_t s_last[MAX_ACTIVE];
    if (threadIdx.x < num_active) {
        s_first[threadIdx.x] = active_first[threadIdx.x];
        s_last[threadIdx.x]  = active_last[threadIdx.x];
    }
    __syncthreads();

    uint32_t i = blockIdx.x * blockDim.x + threadIdx.x;
    uint32_t stride = gridDim.x * blockDim.x;
    uint32_t lane = threadIdx.x & 31;
    uint32_t chunk_id = 0;

    for (; i < num_ops; i += stride) {
        while (chunk_id + 1 < num_chunks && i >= chunk_offsets[chunk_id + 1])
            chunk_id++;

        uint32_t addr = ops[i];

        uint32_t warp_chunk_first = __shfl_sync(0xFFFFFFFF, chunk_id, 0);
        uint32_t warp_chunk_last  = __shfl_sync(0xFFFFFFFF, chunk_id, 31);
        bool same_chunk = (warp_chunk_first == warp_chunk_last);

        for (uint32_t ai = 0; ai < num_active; ai++) {
            uint32_t fa = s_first[ai];
            uint32_t la = s_last[ai];

            if (__all_sync(0xFFFFFFFF, addr < fa)) break;

            bool in_range = (addr >= fa && addr <= la);

            if (!__any_sync(0xFFFFFFFF, in_range)) continue;

            // Categorize: 0=first, 1=middle, 2=last, 3=out-of-range
            uint32_t cat = 3;
            if (in_range) {
                if (addr == fa)      cat = 0;
                else if (addr == la) cat = 2;
                else                 cat = 1;
            }

            if (same_chunk) {
                for (uint32_t c = 0; c < 3; c++) {
                    unsigned mask = __ballot_sync(0xFFFFFFFF, cat == c);
                    if (mask && lane == 0)
                        atomicAdd(&d_fml[(ai * num_chunks + chunk_id) * 3 + c], __popc(mask));
                }
            } else if (in_range) {
                atomicAdd(&d_fml[(ai * num_chunks + chunk_id) * 3 + cat], 1);
            }
        }
    }
}

// Builds per-instance metadata on GPU (one block per active instance, 256 threads).
//
// Pointers marked (*) are pre-offset by the caller for this region's slice of
// the global arrays (e.g., caller passes d_active_ids + active_offset[region]).
//
// Input:  d_fml[num_active * num_chunks * 3]  (*) — first/middle/last counts
// Input:  prefix[N_ADDR+1]                        — exclusive prefix sum of histogram
// Input:  prefix_base_addr                        — first compact address of this region
// Input:  num_ops_region                          — total ops in this region
// Input:  active_ids[num_active]              (*) — local instance IDs in this region
// Input:  active_first[num_active]            (*) — first compact address per instance
// Input:  active_last[num_active]             (*) — last compact address per instance
// Output: result_nops[num_active * num_chunks](*) — per-chunk op count (0=eliminated)
// Output: meta_scalars[num_active * 4]        (*) — [fa_chunk, fa_skip, la_chunk, la_include]
// Input:  num_active, num_chunks                  — dimensions
__global__ void build_metas_kernel(
    const uint32_t* d_fml,
    const uint32_t* prefix,
    uint32_t prefix_base_addr, uint32_t num_ops_region,
    const uint32_t* active_ids,
    const uint32_t* active_first, const uint32_t* active_last,
    uint32_t* result_nops,
    uint32_t* meta_scalars,
    uint32_t num_active, uint32_t num_chunks)
{
    const uint32_t ai = blockIdx.x;
    if (ai >= num_active) return;
    const uint32_t tid = threadIdx.x;
    const uint32_t nthreads = blockDim.x;

    __shared__ uint32_t s_total_compacted;
    __shared__ uint32_t s_first_addr_total_skip;
    __shared__ uint32_t s_last_addr_total_include;
    __shared__ bool     s_single_addr;
    __shared__ uint32_t s_fa_chunk, s_fa_skip, s_la_chunk, s_la_include;
    __shared__ uint32_t s_scan[256];

    const uint32_t* fml_base = d_fml + (size_t)ai * num_chunks * 3;
    uint32_t* scratch = result_nops + (size_t)ai * num_chunks;

    // Phase 1: Compact non-empty chunk IDs
    uint32_t chunks_per_thread = (num_chunks + nthreads - 1) / nthreads;
    uint32_t c_start = tid * chunks_per_thread;
    uint32_t c_end   = min(c_start + chunks_per_thread, num_chunks);

    uint32_t my_count = 0;
    for (uint32_t c = c_start; c < c_end; c++) {
        uint32_t base = c * 3;
        if (fml_base[base] + fml_base[base + 1] + fml_base[base + 2] > 0)
            my_count++;
    }

    s_scan[tid] = my_count;
    __syncthreads();
    if (tid == 0) {
        uint32_t total = 0;
        for (uint32_t i = 0; i < nthreads; i++) {
            uint32_t val = s_scan[i];
            s_scan[i] = total;
            total += val;
        }
        s_total_compacted = total;
    }
    __syncthreads();

    uint32_t write_pos = s_scan[tid];
    for (uint32_t c = c_start; c < c_end; c++) {
        uint32_t base = c * 3;
        if (fml_base[base] + fml_base[base + 1] + fml_base[base + 2] > 0)
            scratch[write_pos++] = c;
    }
    __syncthreads();

    uint32_t nc = s_total_compacted;

    // Phase 2: Compute skip/include counts for first/last address
    if (tid == 0) {
        uint32_t fa = active_first[ai];
        uint32_t la = active_last[ai];
        bool single_addr   = (fa == la);
        uint32_t num_addrs = la - fa + 1;

        s_single_addr = single_addr;

        uint32_t local_inst   = active_ids[ai];
        uint32_t region_start = prefix[prefix_base_addr];
        uint32_t base_pos     = region_start + local_inst * INSTANCE_SIZE;
        uint32_t inst_size    = min(INSTANCE_SIZE, num_ops_region - local_inst * INSTANCE_SIZE);
        uint32_t halo_base    = (local_inst == 0) ? base_pos : base_pos - 1;
        s_first_addr_total_skip = halo_base - prefix[fa];

        if (!single_addr) {
            uint32_t filled_before_last = prefix[fa + num_addrs - 1] - base_pos;
            s_last_addr_total_include = inst_size - filled_before_last;
        } else {
            s_last_addr_total_include = inst_size;
            if (halo_base != base_pos) s_last_addr_total_include++;
        }
    }
    __syncthreads();

    uint32_t first_addr_total_skip    = s_first_addr_total_skip;
    uint32_t last_addr_total_include  = s_last_addr_total_include;
    bool single_addr = s_single_addr;

    // Phase 3: Find first-address chunk and skip count
    uint32_t nc_per_thread = (nc + nthreads - 1) / nthreads;
    uint32_t ci_start = tid * nc_per_thread;
    uint32_t ci_end   = min(ci_start + nc_per_thread, nc);

    uint32_t my_count_first = 0;
    for (uint32_t ci = ci_start; ci < ci_end; ci++)
        my_count_first += fml_base[scratch[ci] * 3 + 0];

    s_scan[tid] = my_count_first;
    __syncthreads();

    if (tid == 0) {
        uint32_t cum = 0;
        for (uint32_t t = 0; t < nthreads; t++) {
            if (cum + s_scan[t] > first_addr_total_skip) {
                uint32_t t_start = t * nc_per_thread;
                uint32_t t_end   = min(t_start + nc_per_thread, nc);
                uint32_t local_cum = cum;
                for (uint32_t ci = t_start; ci < t_end; ci++) {
                    uint32_t cf = fml_base[scratch[ci] * 3 + 0];
                    if (local_cum + cf > first_addr_total_skip) {
                        s_fa_chunk = scratch[ci];
                        s_fa_skip  = first_addr_total_skip - local_cum;
                        break;
                    }
                    local_cum += cf;
                }
                break;
            }
            cum += s_scan[t];
        }
    }
    __syncthreads();

    // Phase 4: Find last-address chunk and include count
    uint32_t la_cat = single_addr ? 0 : 2;
    uint32_t la_threshold = single_addr
        ? (first_addr_total_skip + last_addr_total_include)
        : last_addr_total_include;

    uint32_t my_cum_last = 0;
    for (uint32_t ci = ci_start; ci < ci_end; ci++)
        my_cum_last += fml_base[scratch[ci] * 3 + la_cat];

    s_scan[tid] = my_cum_last;
    __syncthreads();

    if (tid == 0) {
        uint32_t cum = 0;
        for (uint32_t t = 0; t < nthreads; t++) {
            if (cum + s_scan[t] >= la_threshold) {
                uint32_t t_start = t * nc_per_thread;
                uint32_t t_end   = min(t_start + nc_per_thread, nc);
                uint32_t local_cum = cum;
                for (uint32_t ci = t_start; ci < t_end; ci++) {
                    uint32_t cv = fml_base[scratch[ci] * 3 + la_cat];
                    if (local_cum + cv >= la_threshold) {
                        s_la_chunk   = scratch[ci];
                        s_la_include = la_threshold - local_cum;
                        break;
                    }
                    local_cum += cv;
                }
                break;
            }
            cum += s_scan[t];
        }
    }
    __syncthreads();

    uint32_t fa_chunk   = s_fa_chunk;
    uint32_t fa_skip    = s_fa_skip;
    uint32_t la_chunk   = s_la_chunk;
    uint32_t la_include = s_la_include;

    // Phase 5: Write per-chunk op counts (chunk elimination)
    uint32_t* out_nops = result_nops + (size_t)ai * num_chunks;
    for (uint32_t c = tid; c < num_chunks; c += nthreads)
        out_nops[c] = 0;
    __syncthreads();

    for (uint32_t c = c_start; c < c_end; c++) {
        uint32_t base = c * 3;
        uint32_t cf = fml_base[base + 0];
        uint32_t cm = fml_base[base + 1];
        uint32_t cl = fml_base[base + 2];
        if (cf + cm + cl == 0) continue;

        bool needed = (cm > 0);
        if (cf > 0) {
            if (single_addr) {
                if (c >= fa_chunk && c <= la_chunk) needed = true;
            } else {
                if (c >= fa_chunk) needed = true;
            }
        }
        if (cl > 0 && c <= la_chunk)
            needed = true;
        if (needed)
            out_nops[c] = cf + cm + cl;
    }

    // Phase 6: Write scalar results
    if (tid == 0) {
        uint32_t* out = meta_scalars + ai * 4;
        out[0] = fa_chunk;
        out[1] = fa_skip;
        out[2] = la_chunk;
        out[3] = la_include;
    }
}

// Computes per-address write offsets within each instance for scatter positioning.
// For each address in an instance's range, stores the offset into the instance's
// output buffer where ops at that address should be written.
//
// Pointers marked (*) are pre-offset by the caller for this region's slice of
// the global arrays (e.g., caller passes d_active_ids + active_offset[region]).
//
// Input:  prefix[N_ADDR+1]                — exclusive prefix sum of histogram
// Input:  prefix_base_addr                — first compact address of this region
// Input:  num_ops_region                  — total ops in this region
// Input:  active_ids[num_active]      (*) — local instance IDs in this region
// Input:  active_first[num_active]    (*) — first compact address per instance
// Input:  active_last[num_active]     (*) — last compact address per instance
// Output: addr_offsets[total_addrs]   (*) — write offset per address per instance
// Input:  offset_starts[num_active]   (*) — start index in addr_offsets per instance
// Input:  num_active                      — number of active instances in this region
__global__ void compute_addr_offsets_kernel(
    const uint32_t* prefix,
    uint32_t prefix_base_addr, uint32_t num_ops_region,
    const uint32_t* active_ids,
    const uint32_t* active_first, const uint32_t* active_last,
    uint32_t* addr_offsets, const uint32_t* offset_starts,
    uint32_t num_active)
{
    uint32_t ai = blockIdx.x;
    if (ai >= num_active) return;

    uint32_t fa = active_first[ai];
    uint32_t la = active_last[ai];
    uint32_t num_addrs = la - fa + 1;

    uint32_t local_inst   = active_ids[ai];
    uint32_t region_start = prefix[prefix_base_addr];
    uint32_t base_pos     = region_start + local_inst * INSTANCE_SIZE;
    uint32_t halo_base    = (local_inst == 0) ? base_pos : base_pos - 1;

    uint32_t* out = addr_offsets + offset_starts[ai];
    uint32_t tid = threadIdx.x;
    uint32_t stride = blockDim.x;

    if (tid == 0)
        out[0] = (halo_base == base_pos) ? 1 : 0;

    for (uint32_t j = tid + 1; j < num_addrs; j += stride)
        out[j] = prefix[fa + j] - (base_pos - 1);
}

// =====================================================================
// InstanceMeta
// =====================================================================

struct InstanceMeta {
    uint32_t inst_id;            // local instance ID within region
    uint8_t  type;               // REGION_ROM / REGION_INPUT / REGION_RAM
    uint32_t first_addr;         // first raw hardware address covered
    uint32_t last_addr;          // last raw hardware address covered
    std::span<const uint32_t> nops_per_chunk;  // view into pinned h_result_nops
    std::span<uint32_t>       addr_offsets;    // view into pinned h_offsets_buf
    uint32_t first_addr_chunk;   // chunk where first-address data begins
    uint32_t first_addr_skip;    // entries to skip in first_addr_chunk
    uint32_t last_addr_chunk;    // chunk where last-address data ends
    uint32_t last_addr_include;  // entries to include in last_addr_chunk
};

// =====================================================================
// PairSortGPU
// =====================================================================

class PairSortGPU {
    // --- GPU memory ---
    uint32_t* d_ops;
    uint32_t* d_ops_aux;                 // H2D staging + sort input buffer
    uint32_t* d_hist;
    uint32_t* d_prefix;
    void*     d_temp;
    size_t    d_temp_bytes;
    void*     d_sort_temp[N_STREAMS];    // per-stream CUB radix-sort temp storage
    size_t    d_sort_temp_bytes;
    uint32_t* d_active_ids;
    uint32_t* d_active_first;
    uint32_t* d_active_last;
    uint32_t* d_fml;
    uint32_t* d_result_nops;
    uint32_t* d_meta_scalars;
    uint32_t* d_addr_offsets;
    uint32_t* d_offset_starts;
    uint32_t* d_chunk_offsets;

    // --- Streams ---
    cudaStream_t streams[N_STREAMS];
    cudaStream_t d2h_stream;
    cudaStream_t meta_stream;

    // --- Host pinned memory ---
    uint32_t* h_ops;
    uint32_t* h_offsets_buf;
    size_t    h_offsets_buf_size;
    uint32_t* h_result_nops;
    uint32_t* h_meta_scalars;

    // --- Host memory ---
    uint32_t* h_vals;

    // Per-region output arrays (GPU result)
    uint32_t* out_ops_rom;    uint32_t* out_vals_rom;
    uint32_t* out_ops_input;  uint32_t* out_vals_input;
    uint32_t* out_ops_ram;    uint32_t* out_vals_ram;

    // Per-region reference arrays (CPU verification)
    uint32_t* ref_ops_rom;    uint32_t* ref_vals_rom;
    uint32_t* ref_ops_input;  uint32_t* ref_vals_input;
    uint32_t* ref_ops_ram;    uint32_t* ref_vals_ram;

    // --- Active instance tracking ---
    uint32_t active_mask[MASK_WORDS];
    uint32_t h_active_local_ids[MAX_ACTIVE];  // packed local IDs: [ROM...][INPUT...][RAM...]
    std::vector<uint32_t> h_active_first;
    std::vector<uint32_t> h_active_last;
    std::vector<InstanceMeta> metas;

    // --- Runtime state ---
    uint32_t num_ops;
    uint32_t num_chunks;
    uint32_t num_instances;
    uint32_t num_active;
    std::vector<uint32_t> chunk_offsets;

    // --- Per-region state ---
    uint32_t num_inst[3];          // instance count per region
    uint32_t num_active_per[3];    // active instance count per region
    uint32_t active_offset[3];     // offset into packed active arrays per region
    uint32_t region_ops_start[3];  // sorted position where each region starts
    uint32_t region_n_ops[3];      // total ops per region

public:
    PairSortGPU();
    ~PairSortGPU();

    void generate(uint32_t block_number);
    void gpu_metadata();
    void cpu_fill();
    void reference_sort();
    void verify();

private:
    void create_active_mask();
    void pick_active_instances();
};

// =====================================================================
// PairSortGPU Implementation
// =====================================================================

PairSortGPU::PairSortGPU()
    : h_active_first(MAX_ACTIVE), h_active_last(MAX_ACTIVE), metas(MAX_ACTIVE),
      num_ops(0), num_chunks(0), num_instances(0), num_active(0),
      num_inst{}, num_active_per{}, active_offset{}, region_ops_start{}, region_n_ops{}
{
    // GPU allocations
    cudaMalloc(&d_ops,           (size_t)MAX_OPS * sizeof(uint32_t));
    cudaMalloc(&d_ops_aux,       (size_t)MAX_OPS * sizeof(uint32_t));
    cudaMalloc(&d_hist,          (size_t)N_ADDR * sizeof(uint32_t));
    cudaMalloc(&d_prefix,        (size_t)(N_ADDR + 1) * sizeof(uint32_t));

    d_temp = nullptr;
    d_temp_bytes = 0;
    cub::DeviceScan::ExclusiveSum(d_temp, d_temp_bytes, d_hist, d_prefix, N_ADDR);
    cudaMalloc(&d_temp, d_temp_bytes);

    // Per-stream radix-sort temp storage. Sized lazily in gpu_metadata once we
    // know the largest actual chunk — CUB's temp-bytes query depends on
    // num_items and querying for MAX_OPS produces wildly oversized temp that
    // misleads the dispatch for small per-chunk sorts.
    d_sort_temp_bytes = 0;
    for (int s = 0; s < N_STREAMS; s++)
        d_sort_temp[s] = nullptr;


    cudaMalloc(&d_active_ids,    MAX_ACTIVE * sizeof(uint32_t));
    cudaMalloc(&d_active_first,  MAX_ACTIVE * sizeof(uint32_t));
    cudaMalloc(&d_active_last,   MAX_ACTIVE * sizeof(uint32_t));
    cudaMalloc(&d_fml,           (size_t)MAX_ACTIVE * MAX_CHUNKS * 3 * sizeof(uint32_t));
    cudaMalloc(&d_result_nops,   (size_t)MAX_ACTIVE * MAX_CHUNKS * sizeof(uint32_t));
    cudaMalloc(&d_meta_scalars,  (size_t)MAX_ACTIVE * 4 * sizeof(uint32_t));
    cudaMalloc(&d_addr_offsets,  (size_t)MAX_ACTIVE * (INSTANCE_SIZE + 1) * sizeof(uint32_t));
    cudaMalloc(&d_offset_starts, MAX_ACTIVE * sizeof(uint32_t));
    cudaMalloc(&d_chunk_offsets, (MAX_CHUNKS + 1) * sizeof(uint32_t));

    // Streams
    for (int s = 0; s < N_STREAMS; s++)
        cudaStreamCreate(&streams[s]);
    cudaStreamCreate(&d2h_stream);
    cudaStreamCreate(&meta_stream);

    // Host pinned
    cudaMallocHost(&h_ops,          (size_t)MAX_OPS * sizeof(uint32_t));
    h_offsets_buf_size = 1ull << 28;  // 256 MB
    cudaMallocHost(&h_offsets_buf,   h_offsets_buf_size);
    cudaMallocHost(&h_result_nops,   (size_t)MAX_ACTIVE * MAX_CHUNKS * sizeof(uint32_t));
    cudaMallocHost(&h_meta_scalars,  (size_t)MAX_ACTIVE * 4 * sizeof(uint32_t));

    // Host — per-region output/reference arrays
    h_vals = new uint32_t[MAX_OPS];
    size_t rom_sz   = (size_t)MAX_INST_ROM   * INSTANCE_SIZE;
    size_t input_sz = (size_t)MAX_INST_INPUT * INSTANCE_SIZE;
    size_t ram_sz   = (size_t)MAX_INST_RAM   * INSTANCE_SIZE;

    out_ops_rom   = new uint32_t[rom_sz]();    out_vals_rom   = new uint32_t[rom_sz]();
    out_ops_input = new uint32_t[input_sz]();  out_vals_input = new uint32_t[input_sz]();
    out_ops_ram   = new uint32_t[ram_sz]();    out_vals_ram   = new uint32_t[ram_sz]();

    ref_ops_rom   = new uint32_t[rom_sz];      ref_vals_rom   = new uint32_t[rom_sz];
    ref_ops_input = new uint32_t[input_sz];    ref_vals_input = new uint32_t[input_sz];
    ref_ops_ram   = new uint32_t[ram_sz];      ref_vals_ram   = new uint32_t[ram_sz];

    // Print memory usage
    size_t gpu_bytes = ((size_t)MAX_OPS * 2 + N_ADDR + N_ADDR + 1) * sizeof(uint32_t)
                     + d_temp_bytes
                     + (size_t)N_STREAMS * d_sort_temp_bytes
                     + (size_t)MAX_ACTIVE * 3 * sizeof(uint32_t)
                     + (size_t)MAX_ACTIVE * MAX_CHUNKS * 3 * sizeof(uint32_t)
                     + (size_t)MAX_ACTIVE * MAX_CHUNKS * sizeof(uint32_t)
                     + (size_t)MAX_ACTIVE * 4 * sizeof(uint32_t)
                     + (size_t)MAX_ACTIVE * (INSTANCE_SIZE + 1) * sizeof(uint32_t)
                     + (size_t)MAX_ACTIVE * sizeof(uint32_t)
                     + (size_t)(MAX_CHUNKS + 1) * sizeof(uint32_t);
    size_t pinned_bytes = (size_t)MAX_OPS * sizeof(uint32_t)
                        + h_offsets_buf_size
                        + (size_t)MAX_ACTIVE * MAX_CHUNKS * sizeof(uint32_t)
                        + (size_t)MAX_ACTIVE * 4 * sizeof(uint32_t);
    std::cout << "=== Setup ===" << std::endl
              << std::fixed << std::setprecision(1)
              << "  GPU:         " << gpu_bytes / (double)(1 << 30) << " GB" << std::endl
              << "  Host pinned: " << pinned_bytes / (double)(1 << 30) << " GB" << std::endl;
}

PairSortGPU::~PairSortGPU() {
    cudaFree(d_ops);
    cudaFree(d_ops_aux);
    cudaFree(d_hist);
    cudaFree(d_prefix);
    cudaFree(d_temp);
    for (int s = 0; s < N_STREAMS; s++)
        cudaFree(d_sort_temp[s]);
    cudaFree(d_active_ids);
    cudaFree(d_active_first);
    cudaFree(d_active_last);
    cudaFree(d_fml);
    cudaFree(d_result_nops);
    cudaFree(d_meta_scalars);
    cudaFree(d_addr_offsets);
    cudaFree(d_offset_starts);
    cudaFree(d_chunk_offsets);

    for (int s = 0; s < N_STREAMS; s++)
        cudaStreamDestroy(streams[s]);
    cudaStreamDestroy(d2h_stream);
    cudaStreamDestroy(meta_stream);

    cudaFreeHost(h_ops);
    cudaFreeHost(h_offsets_buf);
    cudaFreeHost(h_result_nops);
    cudaFreeHost(h_meta_scalars);

    delete[] h_vals;
    delete[] out_ops_rom;
    delete[] out_vals_rom;
    delete[] out_ops_input;
    delete[] out_vals_input;
    delete[] out_ops_ram;
    delete[] out_vals_ram;
    delete[] ref_ops_rom;
    delete[] ref_vals_rom;
    delete[] ref_ops_input;
    delete[] ref_vals_input;
    delete[] ref_ops_ram;
    delete[] ref_vals_ram;
}

void PairSortGPU::create_active_mask() {
    std::mt19937 rng(123);
    std::uniform_int_distribution<uint32_t> dist(0, N_WORKERS - 1);
    uint32_t worker_id = dist(rng);

    memset(active_mask, 0, sizeof(active_mask));
    for (uint32_t i = 0; i < MAX_INSTANCES; i++)
        if (i % N_WORKERS == worker_id)
            active_mask[i / 32] |= (1u << (i % 32));
}

void PairSortGPU::pick_active_instances() {
    uint32_t pos = 0;
    uint32_t gid_base = 0;
    for (uint8_t r = 0; r < 3; r++) {
        num_active_per[r] = 0;
        active_offset[r] = pos;
        for (uint32_t lid = 0; lid < num_inst[r]; lid++) {
            uint32_t gid = gid_base + lid;
            if (active_mask[gid / 32] & (1u << (gid % 32)))
                h_active_local_ids[pos + num_active_per[r]++] = lid;
        }
        pos += num_active_per[r];
        gid_base += num_inst[r];
    }
    num_active = num_active_per[0] + num_active_per[1] + num_active_per[2];

    std::cout << "  Instances: " << num_instances
              << " (ROM: " << num_inst[0]
              << ", INPUT: " << num_inst[1]
              << ", RAM: " << num_inst[2] << ")"
              << ", Active: " << num_active
              << " (ROM: " << num_active_per[0]
              << ", INPUT: " << num_active_per[1]
              << ", RAM: " << num_active_per[2] << ")"
              << std::endl;
    std::cout << "  Active global IDs:";
    for (uint32_t i = 0; i < num_instances; i++)
        if (active_mask[i / 32] & (1u << (i % 32)))
            std::cout << " " << i;
    std::cout << std::endl;

    cudaMemcpy(d_active_ids, h_active_local_ids,
               num_active * sizeof(uint32_t), cudaMemcpyHostToDevice);
}

// =====================================================================
// Pipeline stages
// =====================================================================

void PairSortGPU::generate(uint32_t block_number) {
    std::cout << std::endl << "=== Generate (block " << block_number << ") ===" << std::endl;
    double t = omp_get_wtime();

    chunk_offsets.clear();
    chunk_offsets.push_back(0);
    uint32_t total_ops = 0;
    char path[512];

    for (uint32_t file_idx = 0; ; file_idx++) {
        snprintf(path, sizeof(path), "chunk_data/%u/mem_addr_%04u.bin", block_number, file_idx);
        FILE* f = fopen(path, "rb");
        if (!f) break;

        fseek(f, 0, SEEK_END);
        size_t file_size = ftell(f);
        fseek(f, 0, SEEK_SET);
        uint32_t n_entries = file_size / sizeof(uint32_t);

        if (total_ops + n_entries > MAX_OPS) {
            fclose(f);
            break;
        }

        size_t read = fread(h_ops + total_ops, sizeof(uint32_t), n_entries, f);
        if (read != n_entries) {
            fclose(f);
            break;
        }
        fclose(f);

        if (n_entries > 0) {
            total_ops += n_entries;
            chunk_offsets.push_back(total_ops);
        }

        if (chunk_offsets.size() - 1 >= MAX_CHUNKS) break;
    }

    num_ops = total_ops;
    if (num_ops == 0) {
        std::cerr << "  ERROR: no ops found" << std::endl;
        exit(1);
    }
    num_chunks = chunk_offsets.size() - 1;

    for (uint32_t i = 0; i < num_ops; i++)
        h_vals[i] = i;

    cudaMemcpy(d_chunk_offsets, chunk_offsets.data(),
               (num_chunks + 1) * sizeof(uint32_t), cudaMemcpyHostToDevice);

    create_active_mask();

    std::cout << std::fixed << std::setprecision(2)
              << "  " << num_ops << " ops in " << num_chunks << " chunks"
              << " (" << (omp_get_wtime() - t) * 1e3 << " ms)" << std::endl;
}

void PairSortGPU::gpu_metadata() {
    std::cout << std::endl << "=== GPU Metadata ===" << std::endl;
    double t_total = omp_get_wtime(), t;

    // H2D + shift + histogram (pipelined by chunks)
    cudaMemset(d_hist, 0, (size_t)N_ADDR * sizeof(uint32_t));

    // Size per-stream CUB temp to the largest actual chunk in this run.
    uint32_t max_chunk_size = 0;
    for (uint32_t c = 0; c < num_chunks; c++) {
        uint32_t sz = chunk_offsets[c + 1] - chunk_offsets[c];
        if (sz > max_chunk_size) max_chunk_size = sz;
    }
    size_t needed = 0;
    cub::DeviceRadixSort::SortKeys(
        nullptr, needed,
        (uint32_t*)nullptr, (uint32_t*)nullptr, max_chunk_size);
    if (needed > d_sort_temp_bytes) {
        for (int s = 0; s < N_STREAMS; s++) {
            if (d_sort_temp[s]) cudaFree(d_sort_temp[s]);
            cudaMalloc(&d_sort_temp[s], needed);
        }
        d_sort_temp_bytes = needed;
    }

    t = omp_get_wtime();
    int sh_block, sh_grid;
    cudaOccupancyMaxPotentialBlockSize(&sh_grid, &sh_block, shift_and_histogram_kernel, 0, 0);

    // Measurement variation: sort each chunk's raw addresses into d_ops_aux to
    // measure overhead, but discard the sorted output — shift_and_histogram still
    // reads the original unsorted d_ops, so downstream correctness is unchanged.
    for (uint32_t c = 0; c < num_chunks - 1; c++) {
        int s = c % N_STREAMS;
        uint32_t off  = chunk_offsets[c];
        uint32_t size = chunk_offsets[c + 1] - off;
        cudaMemcpyAsync(d_ops + off, h_ops + off,
                        (size_t)size * sizeof(uint32_t), cudaMemcpyHostToDevice, streams[s]);
        cub::DeviceRadixSort::SortKeys(
            d_sort_temp[s], d_sort_temp_bytes,
            d_ops + off, d_ops_aux + off, size,
            0, sizeof(uint32_t) * 8, streams[s]);
        shift_and_histogram_kernel<<<sh_grid, sh_block, 0, streams[s]>>>(
            d_ops + off, d_hist, size);
    }
    cudaDeviceSynchronize();

    double t_last_chunk = omp_get_wtime();
    {
        uint32_t c   = num_chunks - 1;
        int s        = c % N_STREAMS;
        uint32_t off  = chunk_offsets[c];
        uint32_t size = chunk_offsets[c + 1] - off;
        cudaMemcpyAsync(d_ops + off, h_ops + off,
                        (size_t)size * sizeof(uint32_t), cudaMemcpyHostToDevice, streams[s]);
        cub::DeviceRadixSort::SortKeys(
            d_sort_temp[s], d_sort_temp_bytes,
            d_ops + off, d_ops_aux + off, size,
            0, sizeof(uint32_t) * 8, streams[s]);
        shift_and_histogram_kernel<<<sh_grid, sh_block, 0, streams[s]>>>(
            d_ops + off, d_hist, size);
    }
    cudaDeviceSynchronize();

    // Force the sort output to be consumed so nothing (CUB/nvcc/driver) can
    // elide the sort as dead work: read the first and last sorted keys across
    // the whole range and fold them into a printed sentinel.
    // Force the sort output to be observed so nothing can elide the work:
    // read both ends of the sorted-per-chunk buffer.
    uint32_t sort_first = 0, sort_last = 0;
    cudaMemcpy(&sort_first, d_ops_aux,                sizeof(uint32_t), cudaMemcpyDeviceToHost);
    cudaMemcpy(&sort_last,  d_ops_aux + num_ops - 1,  sizeof(uint32_t), cudaMemcpyDeviceToHost);

    std::cout << std::fixed << std::setprecision(2)
              << "  H2D + sort + histogram: " << (omp_get_wtime() - t) * 1e3 << " ms"
              << " (sort sentinel: 0x" << std::hex << sort_first
              << " / 0x" << sort_last << std::dec << ")" << std::endl;

    // Prefix sum
    t = omp_get_wtime();
    cub::DeviceScan::ExclusiveSum(d_temp, d_temp_bytes, d_hist, d_prefix, N_ADDR);
    cudaMemcpy(d_prefix + N_ADDR, &num_ops, sizeof(uint32_t), cudaMemcpyHostToDevice);
    cudaDeviceSynchronize();
    std::cout << std::fixed << std::setprecision(2)
              << "  Prefix sum:       " << (omp_get_wtime() - t) * 1e3 << " ms" << std::endl;

    // Compute per-region instance counts from prefix boundary values
    t = omp_get_wtime();
    uint32_t h_boundary[3];
    cudaMemcpy(&h_boundary[0], d_prefix + N_ADDR_ROM,                sizeof(uint32_t), cudaMemcpyDeviceToHost);
    cudaMemcpy(&h_boundary[1], d_prefix + N_ADDR_ROM + N_ADDR_INPUT, sizeof(uint32_t), cudaMemcpyDeviceToHost);
    h_boundary[2] = num_ops;

    region_n_ops[REGION_ROM]   = h_boundary[0];
    region_n_ops[REGION_INPUT] = h_boundary[1] - h_boundary[0];
    region_n_ops[REGION_RAM]   = h_boundary[2] - h_boundary[1];

    num_instances = 0;
    for (uint8_t r = 0; r < 3; r++) {
        num_inst[r] = region_n_ops[r] ? (region_n_ops[r] + INSTANCE_SIZE - 1) / INSTANCE_SIZE : 0;
        if (num_inst[r] > MAX_INST[r]) {
            std::cerr << "ERROR: too many instances in region " << REGION_NAME[r]
                      << " (" << num_inst[r] << " > " << MAX_INST[r] << ")" << std::endl;
            exit(1);
        }
        num_instances += num_inst[r];
    }

    region_ops_start[REGION_ROM]   = 0;
    region_ops_start[REGION_INPUT] = h_boundary[0];
    region_ops_start[REGION_RAM]   = h_boundary[1];

    std::cout << std::fixed << std::setprecision(2)
              << "  Region counts:"
              << " ROM=" << region_n_ops[0] << " (" << num_inst[0] << " inst),"
              << " INPUT=" << region_n_ops[1] << " (" << num_inst[1] << " inst),"
              << " RAM=" << region_n_ops[2] << " (" << num_inst[2] << " inst),"
              << " total=" << num_instances << " inst" << std::endl;

    pick_active_instances();

    // Instance boundaries (per-region, cheap)
    cudaMemset(d_fml, 0, (size_t)num_active * num_chunks * 3 * sizeof(uint32_t));
    for (uint8_t r = 0; r < 3; r++) {
        if (num_active_per[r] == 0) continue;
        uint32_t na  = num_active_per[r];
        uint32_t off = active_offset[r];
        instance_boundaries_kernel<<<1, na>>>(
            d_prefix, REGION_ADDR_START[r], REGION_N_ADDR[r],
            region_n_ops[r],
            d_active_ids + off, d_active_first + off, d_active_last + off,
            d_offset_starts + off, na);
    }

    // Chunk FML counts — single pass over all ops for all active instances
    int fml_block, fml_grid;
    cudaOccupancyMaxPotentialBlockSize(&fml_grid, &fml_block, chunk_fml_count_kernel, 0, 0);
    chunk_fml_count_kernel<<<fml_grid, fml_block>>>(
        d_ops, d_active_first, d_active_last, d_chunk_offsets,
        d_fml, num_active, num_chunks, num_ops);

    cudaDeviceSynchronize();

    // D2H: active boundaries
    cudaMemcpy(h_active_first.data(), d_active_first, num_active * sizeof(uint32_t), cudaMemcpyDeviceToHost);
    cudaMemcpy(h_active_last.data(),  d_active_last,  num_active * sizeof(uint32_t), cudaMemcpyDeviceToHost);
    std::cout << std::fixed << std::setprecision(2)
              << "  Boundaries + FML: " << (omp_get_wtime() - t) * 1e3 << " ms" << std::endl;

    // Build metas + addr_offsets (overlapped on two streams)
    t = omp_get_wtime();

    std::vector<uint32_t> h_offset_starts(num_active);
    uint32_t total_addrs = 0;
    for (uint32_t i = 0; i < num_active; i++) {
        h_offset_starts[i] = total_addrs;
        total_addrs += h_active_last[i] - h_active_first[i] + 1;
    }

    for (uint8_t r = 0; r < 3; r++) {
        if (num_active_per[r] == 0) continue;
        uint32_t na  = num_active_per[r];
        uint32_t off = active_offset[r];

        build_metas_kernel<<<na, 256, 0, meta_stream>>>(
            d_fml + (size_t)off * num_chunks * 3, d_prefix,
            REGION_ADDR_START[r], region_n_ops[r],
            d_active_ids + off, d_active_first + off, d_active_last + off,
            d_result_nops + (size_t)off * num_chunks,
            d_meta_scalars + off * 4, na, num_chunks);

        compute_addr_offsets_kernel<<<na, 1024, 0, d2h_stream>>>(
            d_prefix, REGION_ADDR_START[r], region_n_ops[r],
            d_active_ids + off, d_active_first + off, d_active_last + off,
            d_addr_offsets, d_offset_starts + off, na);
    }

    cudaMemcpyAsync(h_meta_scalars, d_meta_scalars,
                    num_active * 4 * sizeof(uint32_t), cudaMemcpyDeviceToHost, meta_stream);
    cudaMemcpyAsync(h_result_nops, d_result_nops,
                    (size_t)num_active * num_chunks * sizeof(uint32_t), cudaMemcpyDeviceToHost, meta_stream);
    cudaMemcpyAsync(h_offsets_buf, d_addr_offsets,
                    (size_t)total_addrs * sizeof(uint32_t), cudaMemcpyDeviceToHost, d2h_stream);

    cudaStreamSynchronize(meta_stream);
    cudaStreamSynchronize(d2h_stream);

    // Populate InstanceMeta structs
    uint32_t ai = 0;
    for (uint8_t r = 0; r < 3; r++) {
        for (uint32_t j = 0; j < num_active_per[r]; j++, ai++) {
            uint32_t* scalars      = h_meta_scalars + ai * 4;
            metas[ai].inst_id      = h_active_local_ids[active_offset[r] + j];
            metas[ai].type         = r;
            metas[ai].first_addr   = expand_addr(h_active_first[ai]);
            metas[ai].last_addr    = expand_addr(h_active_last[ai]);
            metas[ai].first_addr_chunk  = scalars[0];
            metas[ai].first_addr_skip   = scalars[1];
            metas[ai].last_addr_chunk   = scalars[2];
            metas[ai].last_addr_include = scalars[3];
            uint32_t num_addrs = h_active_last[ai] - h_active_first[ai] + 1;
            metas[ai].nops_per_chunk = {h_result_nops + (size_t)ai * num_chunks, num_chunks};
            metas[ai].addr_offsets   = {h_offsets_buf + h_offset_starts[ai], num_addrs};
        }
    }

    std::cout << std::fixed << std::setprecision(2)
              << "  Build metas:      " << (omp_get_wtime() - t) * 1e3 << " ms" << std::endl
              << "  TOTAL:            " << (omp_get_wtime() - t_total) * 1e3 << " ms" << std::endl
              << "  Last chunk to ready: " << (omp_get_wtime() - t_last_chunk) * 1e3 << " ms" << std::endl;
}

void PairSortGPU::cpu_fill() {
    std::cout << std::endl << "=== CPU Fill ===" << std::endl;
    double t = omp_get_wtime();

    #pragma omp parallel for schedule(dynamic, 1)
    for (size_t idx = 0; idx < num_active; idx++) {
        auto& m = metas[idx];
        bool single_addr = (m.first_addr == m.last_addr);

        uint32_t* o_ops;
        uint32_t* o_vals;
        if (m.type == REGION_ROM)        { o_ops = out_ops_rom;   o_vals = out_vals_rom;   }
        else if (m.type == REGION_INPUT) { o_ops = out_ops_input; o_vals = out_vals_input; }
        else                             { o_ops = out_ops_ram;   o_vals = out_vals_ram;   }

        uint32_t inst_size     = std::min(INSTANCE_SIZE, region_n_ops[m.type] - m.inst_id * INSTANCE_SIZE);
        uint32_t out_base      = m.inst_id * INSTANCE_SIZE;
        uint32_t total_written = 0;

        for (uint32_t chunk = 0; chunk < num_chunks; chunk++) {
            uint32_t expected = m.nops_per_chunk[chunk];
            if (expected == 0) continue;

            uint32_t chunk_start = chunk_offsets[chunk];
            uint32_t chunk_size  = chunk_offsets[chunk + 1] - chunk_start;
            uint32_t found       = 0;
            uint32_t first_found = 0;
            uint32_t last_found  = 0;

            for (uint32_t j = 0; j < chunk_size && found < expected; j++) {
                uint32_t raw  = h_ops[chunk_start + j];
                if (raw < m.first_addr || raw > m.last_addr) continue;
                uint32_t ind  = (raw - m.first_addr) >> 3;
                found++;

                bool skip = false;
                if (raw == m.first_addr) {
                    first_found++;
                    if (chunk < m.first_addr_chunk) skip = true;
                    else if (chunk == m.first_addr_chunk && first_found <= m.first_addr_skip) skip = true;
                    else if (single_addr) {
                        if (chunk > m.last_addr_chunk) skip = true;
                        else if (chunk == m.last_addr_chunk && first_found > m.last_addr_include) skip = true;
                    }
                } else if (raw == m.last_addr) {
                    last_found++;
                    if (chunk > m.last_addr_chunk) skip = true;
                    else if (chunk == m.last_addr_chunk && last_found > m.last_addr_include) skip = true;
                }
                if (skip) continue;

                uint32_t pos = m.addr_offsets[ind]++;
                if (pos == 0) continue;  // halo entry
                uint32_t out_pos = out_base + pos - 1;
                o_ops[out_pos]  = raw;
                o_vals[out_pos] = h_vals[chunk_start + j];
                total_written++;
                if (total_written >= inst_size) break;
            }
            if (total_written >= inst_size) break;
        }
    }

    std::cout << std::fixed << std::setprecision(2)
              << "  " << num_active << " instances in " << (omp_get_wtime() - t) * 1e3 << " ms" << std::endl;
}

void PairSortGPU::reference_sort() {
    std::cout << std::endl << "=== Verify ===" << std::endl;
    double t = omp_get_wtime();

    struct Triple { uint32_t compact, pos, val, raw; };
    std::vector<Triple> triples(num_ops);
    for (uint32_t i = 0; i < num_ops; i++)
        triples[i] = {compact_addr(h_ops[i]), i, h_vals[i], h_ops[i]};
    std::sort(triples.begin(), triples.end(), [](const Triple& a, const Triple& b) {
        return a.compact < b.compact || (a.compact == b.compact && a.pos < b.pos);
    });

    memset(ref_ops_rom,    0, (size_t)MAX_INST_ROM   * INSTANCE_SIZE * sizeof(uint32_t));
    memset(ref_vals_rom,   0, (size_t)MAX_INST_ROM   * INSTANCE_SIZE * sizeof(uint32_t));
    memset(ref_ops_input,  0, (size_t)MAX_INST_INPUT * INSTANCE_SIZE * sizeof(uint32_t));
    memset(ref_vals_input, 0, (size_t)MAX_INST_INPUT * INSTANCE_SIZE * sizeof(uint32_t));
    memset(ref_ops_ram,    0, (size_t)MAX_INST_RAM   * INSTANCE_SIZE * sizeof(uint32_t));
    memset(ref_vals_ram,   0, (size_t)MAX_INST_RAM   * INSTANCE_SIZE * sizeof(uint32_t));

    for (uint32_t p = 0; p < num_ops; p++) {
        uint32_t ca = triples[p].compact;
        uint32_t* r_ops;
        uint32_t* r_vals;
        uint32_t local_p;

        if (ca < N_ADDR_ROM) {
            r_ops   = ref_ops_rom;   r_vals = ref_vals_rom;
            local_p = p - region_ops_start[REGION_ROM];
        } else if (ca < N_ADDR_ROM + N_ADDR_INPUT) {
            r_ops   = ref_ops_input; r_vals = ref_vals_input;
            local_p = p - region_ops_start[REGION_INPUT];
        } else {
            r_ops   = ref_ops_ram;   r_vals = ref_vals_ram;
            local_p = p - region_ops_start[REGION_RAM];
        }

        uint32_t out_idx = local_p;  // inst_local * INSTANCE_SIZE + local_pos == local_p
        r_ops[out_idx]  = triples[p].raw;
        r_vals[out_idx] = triples[p].val;
    }

    std::cout << std::fixed << std::setprecision(2)
              << "  Reference sort:   " << (omp_get_wtime() - t) * 1e3 << " ms" << std::endl;
}

void PairSortGPU::verify() {
    double t = omp_get_wtime();
    uint32_t total_verified = 0;

    for (uint32_t idx = 0; idx < num_active; idx++) {
        auto& m = metas[idx];

        uint32_t* o_ops;  uint32_t* o_vals;
        uint32_t* r_ops;  uint32_t* r_vals;
        if (m.type == REGION_ROM) {
            o_ops = out_ops_rom;   o_vals = out_vals_rom;
            r_ops = ref_ops_rom;   r_vals = ref_vals_rom;
        } else if (m.type == REGION_INPUT) {
            o_ops = out_ops_input; o_vals = out_vals_input;
            r_ops = ref_ops_input; r_vals = ref_vals_input;
        } else {
            o_ops = out_ops_ram;   o_vals = out_vals_ram;
            r_ops = ref_ops_ram;   r_vals = ref_vals_ram;
        }

        uint32_t inst_size = std::min(INSTANCE_SIZE, region_n_ops[m.type] - m.inst_id * INSTANCE_SIZE);
        uint32_t start     = m.inst_id * INSTANCE_SIZE;

        for (uint32_t j = 0; j < inst_size; j++) {
            uint32_t ind = start + j;
            if (o_ops[ind] != r_ops[ind] || o_vals[ind] != r_vals[ind]) {
                std::cout << "MISMATCH " << REGION_NAME[m.type] << " inst " << m.inst_id
                          << " local " << j
                          << ": got (" << o_ops[ind] << "," << o_vals[ind] << ")"
                          << " expected (" << r_ops[ind] << "," << r_vals[ind] << ")" << std::endl;
                return;
            }
        }
        total_verified += inst_size;
    }

    std::cout << std::fixed << std::setprecision(2)
              << "  Verify:           " << (omp_get_wtime() - t) * 1e3
              << " ms -- " << total_verified << " entries, " << num_active << " instances OK" << std::endl;
}

// =====================================================================
// Main
// =====================================================================

int main(int argc, char** argv) {
    bool do_verify = false;
    uint32_t block_number = 0;
    bool have_block = false;

    for (int i = 1; i < argc; i++) {
        if (std::string(argv[i]) == "-v")
            do_verify = true;
        else {
            block_number = std::strtoul(argv[i], nullptr, 10);
            have_block = true;
        }
    }
    if (!have_block) {
        std::cerr << "Usage: " << argv[0] << " <block_number> [-v]" << std::endl;
        return 1;
    }

    PairSortGPU app;
    app.generate(block_number);
    app.gpu_metadata();
    app.cpu_fill();

    if (do_verify) {
        app.reference_sort();
        app.verify();
    }
}
