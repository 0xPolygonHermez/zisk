// Standalone test main for the GPU port of MemCounterSingle::execute.
//
// Usage: ./pair_sort_preprocess_gpu <block_number> [--verify]
//
// Reference semantics: per-chunk duality collapsing, state CLEARED between
// chunks (matches mem_counter_single.cpp::execute's clear() call). The
// precomputed data/<block>_aligned/*.bin files were generated with cross-
// chunk state persistence and therefore do NOT match this semantics, so we
// use a local CPU oracle (run_cpu_chunk) for verification.

#include "mem_preprocess.cuh"

#include <cub/device/device_radix_sort.cuh>
#include <cub/device/device_run_length_encode.cuh>
#include <cub/device/device_scan.cuh>
#include <thrust/iterator/discard_iterator.h>

#include <algorithm>
#include <cctype>
#include <chrono>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <dirent.h>
#include <iostream>
#include <string>
#include <sys/stat.h>
#include <vector>

struct ChunkRef {
    uint32_t file_idx;
    uint32_t n_memops;
    uint32_t memop_offset;
};

static int parse_idx_from_filename(const char* prefix, const char* name) {
    size_t plen = strlen(prefix);
    if (strncmp(name, prefix, plen) != 0) return -1;
    const char* p = name + plen;
    if (!*p || !isdigit((unsigned char)*p)) return -1;
    char* end = nullptr;
    long v = strtol(p, &end, 10);
    if (!end || strcmp(end, ".bin") != 0) return -1;
    return (int)v;
}

static std::vector<uint32_t> list_indices(const std::string& dir, const char* prefix) {
    std::vector<uint32_t> idxs;
    DIR* d = opendir(dir.c_str());
    if (!d) { fprintf(stderr, "ERROR: cannot open %s\n", dir.c_str()); exit(1); }
    struct dirent* e;
    while ((e = readdir(d)) != nullptr) {
        int idx = parse_idx_from_filename(prefix, e->d_name);
        if (idx >= 0) idxs.push_back((uint32_t)idx);
    }
    closedir(d);
    std::sort(idxs.begin(), idxs.end());
    return idxs;
}

static size_t file_size(const std::string& path) {
    struct stat st;
    if (stat(path.c_str(), &st) != 0) { fprintf(stderr, "ERROR: stat %s\n", path.c_str()); exit(1); }
    return (size_t)st.st_size;
}

// ---------------------------------------------------------------------
// CPU oracle: per-chunk execution matching mem_counter_single.cpp.
// State table is passed in and cleared before each call (mirrors clear()).
// ---------------------------------------------------------------------

static void cpu_add_r(uint32_t addr, std::vector<bool>& free_r, std::vector<uint32_t>& out) {
    const bool ram = is_ram_addr(addr);
    if (ram) {
        uint32_t off = ram_compact(addr);
        if (free_r[off]) { free_r[off] = false; }
        else             { free_r[off] = true; out.push_back(addr); }
    } else {
        out.push_back(addr);
    }
}
static void cpu_add_w(uint32_t addr, std::vector<bool>& free_r, std::vector<uint32_t>& out) {
    if (is_ram_addr(addr)) free_r[ram_compact(addr)] = true;
    out.push_back(addr);
}
static void cpu_add_rw(uint32_t addr, std::vector<bool>& free_r, std::vector<uint32_t>& out) {
    const bool ram = is_ram_addr(addr);
    if (ram) {
        uint32_t off = ram_compact(addr);
        if (!free_r[off]) out.push_back(addr);   // read
        out.push_back(addr);                     // write
        free_r[off] = true;
    } else {
        out.push_back(addr);
        out.push_back(addr);
    }
}

static void run_cpu_chunk(const MemOp* chunk, uint32_t n,
                          std::vector<bool>& free_r,
                          std::vector<uint32_t>& out) {
    std::fill(free_r.begin(), free_r.end(), false);  // clear() per chunk
    out.clear();
    for (uint32_t k = 0; k < n; k++) {
        const MemOp& op = chunk[k];
        const uint32_t addr    = op.addr;
        const uint32_t aligned = addr & ZISK_ALIGN_MASK;
        const uint8_t  mode    = op.flags & 0x3F;
        const uint32_t off     = addr & 0x07;
        switch (mode) {
            case MOPS_READ_1:   cpu_add_r (aligned, free_r, out); break;
            case MOPS_CWRITE_1:
            case MOPS_WRITE_1:  cpu_add_rw(aligned, free_r, out); break;
            case MOPS_READ_2:   cpu_add_r (aligned, free_r, out);
                                if (off > 6) cpu_add_r (aligned + 8, free_r, out); break;
            case MOPS_WRITE_2:  cpu_add_rw(aligned, free_r, out);
                                if (off > 6) cpu_add_rw(aligned + 8, free_r, out); break;
            case MOPS_READ_4:   cpu_add_r (aligned, free_r, out);
                                if (off > 4) cpu_add_r (aligned + 8, free_r, out); break;
            case MOPS_WRITE_4:  cpu_add_rw(aligned, free_r, out);
                                if (off > 4) cpu_add_rw(aligned + 8, free_r, out); break;
            case MOPS_READ_8:   cpu_add_r (aligned, free_r, out);
                                if (off > 0) cpu_add_r (aligned + 8, free_r, out); break;
            case MOPS_WRITE_8:  if (addr == aligned) cpu_add_w(aligned, free_r, out);
                                else { cpu_add_rw(aligned, free_r, out);
                                       cpu_add_rw(aligned + 8, free_r, out); } break;
            case MOPS_ALIGNED_READ  + 0x00: case MOPS_ALIGNED_READ  + 0x10:
            case MOPS_ALIGNED_READ  + 0x20: case MOPS_ALIGNED_READ  + 0x30:
                cpu_add_r(addr, free_r, out); break;
            case MOPS_ALIGNED_WRITE + 0x00: case MOPS_ALIGNED_WRITE + 0x10:
            case MOPS_ALIGNED_WRITE + 0x20: case MOPS_ALIGNED_WRITE + 0x30:
                cpu_add_w(addr, free_r, out); break;
            case MOPS_BLOCK_READ        + 0x00: case MOPS_BLOCK_READ        + 0x10:
            case MOPS_BLOCK_READ        + 0x20: case MOPS_BLOCK_READ        + 0x30:
            case MOPS_ALIGNED_BLOCK_READ+ 0x00: case MOPS_ALIGNED_BLOCK_READ+ 0x10:
            case MOPS_ALIGNED_BLOCK_READ+ 0x20: case MOPS_ALIGNED_BLOCK_READ+ 0x30: {
                uint32_t cnt = op.flags >> MOPS_BLOCK_COUNT_SBITS;
                for (uint32_t i = 0; i < cnt; i++) cpu_add_r(addr + i * 8, free_r, out);
                break;
            }
            case MOPS_BLOCK_WRITE        + 0x00: case MOPS_BLOCK_WRITE        + 0x10:
            case MOPS_BLOCK_WRITE        + 0x20: case MOPS_BLOCK_WRITE        + 0x30:
            case MOPS_ALIGNED_BLOCK_WRITE+ 0x00: case MOPS_ALIGNED_BLOCK_WRITE+ 0x10:
            case MOPS_ALIGNED_BLOCK_WRITE+ 0x20: case MOPS_ALIGNED_BLOCK_WRITE+ 0x30: {
                uint32_t cnt = op.flags >> MOPS_BLOCK_COUNT_SBITS;
                for (uint32_t i = 0; i < cnt; i++) cpu_add_w(addr + i * 8, free_r, out);
                break;
            }
            default:
                // Match the CPU spec (zisk/mem_counter_single.cpp:153-157):
                // unknown opcodes are a fatal data error, not "skip silently".
                // Without this throw, --verify would silently agree with the
                // GPU's same default-break behaviour on bad data.
                fprintf(stderr,
                        "FATAL (CPU oracle): chunk memop %u has unknown mode 0x%02x "
                        "(addr=0x%08x flags=0x%08x)\n",
                        k, mode, addr, op.flags);
                std::exit(1);
        }
    }
}

// ---------------------------------------------------------------------
// Per-stream device + pinned buffers
// ---------------------------------------------------------------------

struct StreamBufs {
    cudaStream_t stream;

    MemOp*         d_memops;
    uint32_t*      d_counts;
    uint32_t*      d_potential_offsets;
    PotentialEmit* d_potentials;
    uint32_t*      d_emit_bits;
    uint32_t*      d_final_offsets;
    uint32_t*      d_out;
    uint64_t*      d_ram_keys;
    uint64_t*      d_ram_keys_sorted;
    uint32_t*      d_ram_vals_sorted;  // produced post-sort by extract_sorted_packed_kernel
    uint32_t*      d_ram_count;
    BlockOpSpill*  d_spill;
    uint32_t*      d_spill_count;
    uint8_t*       d_spill_status;       // [CHUNK_MAX_MEMOPS], 1=spilled / 0=inline

    uint32_t*      d_sorted_addr;        // per-slot compact_addr, input to RLE
    uint32_t*      d_run_lengths;        // RLE output: events per unique addr
    uint32_t*      d_run_offsets;        // exclusive sum of run_lengths (+sentinel)
    uint32_t*      d_num_unique;         // RLE output: count of unique addrs

    void*          d_scan_temp_counts;   size_t scan_temp_counts_bytes = 0;
    void*          d_scan_temp_emit;     size_t scan_temp_emit_bytes   = 0;
    void*          d_scan_temp_runs;     size_t scan_temp_runs_bytes   = 0;
    void*          d_sort_temp;          size_t sort_temp_bytes        = 0;
    void*          d_rle_temp;           size_t rle_temp_bytes         = 0;

    uint32_t*      h_n_potentials;
    uint32_t*      h_spill_count;
    uint32_t*      h_ram_count;
    uint32_t*      h_n_emits;
};

static void alloc_stream_bufs(StreamBufs& s) {
    CUDA_CHECK(cudaStreamCreate(&s.stream));

    CUDA_CHECK(cudaMalloc(&s.d_memops,            sizeof(MemOp) * CHUNK_MAX_MEMOPS));
    CUDA_CHECK(cudaMalloc(&s.d_counts,            sizeof(uint32_t) * (CHUNK_MAX_MEMOPS + 1)));
    CUDA_CHECK(cudaMalloc(&s.d_potential_offsets, sizeof(uint32_t) * (CHUNK_MAX_MEMOPS + 1)));
    CUDA_CHECK(cudaMalloc(&s.d_potentials,        sizeof(PotentialEmit) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_emit_bits,         sizeof(uint32_t) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_final_offsets,     sizeof(uint32_t) * (POTENTIAL_CAP_PER_CHUNK + 1)));
    CUDA_CHECK(cudaMalloc(&s.d_out,               sizeof(uint32_t) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_ram_keys,          sizeof(uint64_t) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_ram_keys_sorted,   sizeof(uint64_t) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_ram_vals_sorted,   sizeof(uint32_t) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_ram_count,         sizeof(uint32_t)));
    CUDA_CHECK(cudaMalloc(&s.d_spill,             sizeof(BlockOpSpill) * MAX_BLOCKOP_SPILL_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_spill_count,       sizeof(uint32_t)));
    CUDA_CHECK(cudaMalloc(&s.d_spill_status,      sizeof(uint8_t) * CHUNK_MAX_MEMOPS));

    CUDA_CHECK(cudaMalloc(&s.d_sorted_addr,       sizeof(uint32_t) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_run_lengths,       sizeof(uint32_t) * POTENTIAL_CAP_PER_CHUNK));
    CUDA_CHECK(cudaMalloc(&s.d_run_offsets,       sizeof(uint32_t) * (POTENTIAL_CAP_PER_CHUNK + 1)));
    CUDA_CHECK(cudaMalloc(&s.d_num_unique,        sizeof(uint32_t)));

    cub::DeviceScan::ExclusiveSum(nullptr, s.scan_temp_counts_bytes,
        (uint32_t*)nullptr, (uint32_t*)nullptr, CHUNK_MAX_MEMOPS + 1);
    CUDA_CHECK(cudaMalloc(&s.d_scan_temp_counts, s.scan_temp_counts_bytes));

    cub::DeviceScan::ExclusiveSum(nullptr, s.scan_temp_emit_bytes,
        (uint32_t*)nullptr, (uint32_t*)nullptr, POTENTIAL_CAP_PER_CHUNK + 1);
    CUDA_CHECK(cudaMalloc(&s.d_scan_temp_emit, s.scan_temp_emit_bytes));

    cub::DeviceScan::ExclusiveSum(nullptr, s.scan_temp_runs_bytes,
        (uint32_t*)nullptr, (uint32_t*)nullptr, POTENTIAL_CAP_PER_CHUNK + 1);
    CUDA_CHECK(cudaMalloc(&s.d_scan_temp_runs, s.scan_temp_runs_bytes));

    cub::DeviceRadixSort::SortKeys(nullptr, s.sort_temp_bytes,
        (uint64_t*)nullptr, (uint64_t*)nullptr, POTENTIAL_CAP_PER_CHUNK);
    CUDA_CHECK(cudaMalloc(&s.d_sort_temp, s.sort_temp_bytes));

    cub::DeviceRunLengthEncode::Encode(nullptr, s.rle_temp_bytes,
        (uint32_t*)nullptr, thrust::discard_iterator<>{},
        (uint32_t*)nullptr, (uint32_t*)nullptr, POTENTIAL_CAP_PER_CHUNK);
    CUDA_CHECK(cudaMalloc(&s.d_rle_temp, s.rle_temp_bytes));

    CUDA_CHECK(cudaMallocHost(&s.h_n_potentials, sizeof(uint32_t)));
    CUDA_CHECK(cudaMallocHost(&s.h_spill_count,  sizeof(uint32_t)));
    CUDA_CHECK(cudaMallocHost(&s.h_ram_count,    sizeof(uint32_t)));
    CUDA_CHECK(cudaMallocHost(&s.h_n_emits,      sizeof(uint32_t)));
}

static void free_stream_bufs(StreamBufs& s) {
    cudaFree(s.d_memops); cudaFree(s.d_counts); cudaFree(s.d_potential_offsets);
    cudaFree(s.d_potentials); cudaFree(s.d_emit_bits); cudaFree(s.d_final_offsets);
    cudaFree(s.d_out); cudaFree(s.d_ram_keys);
    cudaFree(s.d_ram_keys_sorted); cudaFree(s.d_ram_vals_sorted);
    cudaFree(s.d_ram_count); cudaFree(s.d_spill); cudaFree(s.d_spill_count);
    cudaFree(s.d_spill_status);
    cudaFree(s.d_sorted_addr); cudaFree(s.d_run_lengths); cudaFree(s.d_run_offsets);
    cudaFree(s.d_num_unique);
    cudaFree(s.d_scan_temp_counts); cudaFree(s.d_scan_temp_emit);
    cudaFree(s.d_scan_temp_runs); cudaFree(s.d_sort_temp); cudaFree(s.d_rle_temp);
    cudaFreeHost(s.h_n_potentials); cudaFreeHost(s.h_spill_count);
    cudaFreeHost(s.h_ram_count); cudaFreeHost(s.h_n_emits);
    cudaStreamDestroy(s.stream);
}

// ---------------------------------------------------------------------
// Run one chunk — fully async on sb.stream, NO host syncs.
//
// n_potentials is an exact count of potential emissions, pre-computed on
// CPU during load (matches decode_potential_count). We use it as:
//   - launch grid size for gather, state-machine, compact
//   - num_items for both CUB scans and the sort
//   - fixed D2H length for d_out (over-copy past actual n_emits is harmless;
//     host reads the actual emit count from h_n_emits_slot after final sync)
//
// n_spills is kept device-side; kernels that depend on it take *d_spill_count
// and early-exit. This removes the three per-chunk syncs entirely and lets
// all 4 streams overlap on the GPU.
// ---------------------------------------------------------------------

static void run_chunk(StreamBufs& sb,
                      const MemOp* h_memops_chunk,
                      uint32_t n_memops,
                      uint32_t n_potentials,
                      uint32_t n_ram,
                      ChunkCounters* d_chunk_counters_entry,
                      uint32_t* d_invalid_mode_flag,  // sticky global, OR'd into
                      uint32_t* h_out_chunk,
                      uint32_t* h_n_emits_slot) {
    // Defensive guard: an empty chunk has nothing to do, and many of the
    // launches below would otherwise be issued with grid=0 — accepted by
    // modern CUDA but fragile and not worth depending on. Also: h_n_emits_slot
    // must reflect 0 emits so verify code doesn't read uninitialised pinned.
    if (n_memops == 0) {
        *h_n_emits_slot = 0;
        return;
    }

    cudaStream_t st = sb.stream;
    constexpr int BLOCK = 256;
    const int g_memops = (n_memops + BLOCK - 1) / BLOCK;
    const int g_pot    = (n_potentials + BLOCK - 1) / BLOCK;
    const int g_ram    = n_ram == 0 ? 0 : (int)((n_ram + BLOCK - 1) / BLOCK);

    CUDA_CHECK(cudaMemcpyAsync(sb.d_memops, h_memops_chunk,
                               sizeof(MemOp) * n_memops, cudaMemcpyHostToDevice, st));

    CUDA_CHECK(cudaMemsetAsync(sb.d_ram_count,   0, sizeof(uint32_t), st));
    CUDA_CHECK(cudaMemsetAsync(sb.d_spill_count, 0, sizeof(uint32_t), st));
    // d_spill_status[i] defaults to 0 (= "handle inline"). decode_count_kernel
    // sets it to 1 only on a successful spill claim. Memset only the active
    // memop range — the rest of the buffer is never read this chunk.
    CUDA_CHECK(cudaMemsetAsync(sb.d_spill_status, 0, sizeof(uint8_t) * n_memops, st));
    // d_ram_keys doesn't need sentinel init now — we sort exactly n_ram
    // entries, which matches the count gather will atomically compact.
    // d_potentials doesn't need clearing either: decode_emit + blockop_emit
    // fully populate [0, n_potentials) and no kernel reads past that bound.

    decode_count_kernel<<<g_memops, BLOCK, 0, st>>>(
        sb.d_memops, n_memops, sb.d_counts, sb.d_spill_status,
        d_chunk_counters_entry, sb.d_spill, sb.d_spill_count,
        d_invalid_mode_flag);

    {
        size_t bytes = sb.scan_temp_counts_bytes;
        cub::DeviceScan::ExclusiveSum(sb.d_scan_temp_counts, bytes,
            sb.d_counts, sb.d_potential_offsets, n_memops + 1, st);
    }

    decode_emit_kernel<<<g_memops, BLOCK, 0, st>>>(
        sb.d_memops, n_memops, sb.d_potential_offsets, sb.d_spill_status, sb.d_potentials);

    // blockop_emit with fixed grid; CTAs early-exit past *d_spill_count.
    // Slot bases are looked up directly from d_potential_offsets per spill —
    // no separate fill_spill_bases pass needed.
    blockop_emit_kernel<<<MAX_BLOCKOP_SPILL_PER_CHUNK, 256, 0, st>>>(
        sb.d_spill, sb.d_spill_count, sb.d_potential_offsets, sb.d_potentials);

    gather_ram_events_kernel<<<g_pot, BLOCK, 0, st>>>(
        sb.d_potentials, n_potentials,
        sb.d_ram_keys, sb.d_ram_count, sb.d_emit_bits);

    // Sort exactly n_ram RAM events (pre-computed on CPU during load).
    // Tight key layout: 21 bits orig_pos + 26 bits compact_addr = 47 bits total.
    if (n_ram > 0) {
        size_t bytes_sort = sb.sort_temp_bytes;
        cub::DeviceRadixSort::SortKeys(sb.d_sort_temp, bytes_sort,
            sb.d_ram_keys, sb.d_ram_keys_sorted,
            n_ram, 0, RAM_KEY_END_BIT, st);

        // Extract the compact_addr part of each sorted key for RLE input.
        extract_sorted_addr_kernel<<<g_ram, BLOCK, 0, st>>>(
            sb.d_ram_keys_sorted, n_ram, sb.d_sorted_addr);

        // Extract the per-event (kind_w, orig_pos) packed value into a
        // 32-bit array. state_machine_by_run_kernel reads this (4 bytes per
        // event) instead of the 8-byte sorted keys — keeps state_machine's
        // bandwidth identical to when we used SortPairs.
        extract_sorted_packed_kernel<<<g_ram, BLOCK, 0, st>>>(
            sb.d_ram_keys_sorted, n_ram, sb.d_ram_vals_sorted);

        // RLE: group consecutive equal addresses, get run lengths + count.
        {
            size_t bytes_rle = sb.rle_temp_bytes;
            cub::DeviceRunLengthEncode::Encode(sb.d_rle_temp, bytes_rle,
                sb.d_sorted_addr,
                thrust::discard_iterator<>{},
                sb.d_run_lengths,
                sb.d_num_unique,
                n_ram, st);
        }
        // Exclusive sum over run_lengths → run_offsets (we include n_ram itself
        // as the end sentinel so state_machine_by_run only needs one read per
        // boundary). Over-scan by n_ram+1 — slots past n_unique are garbage but
        // unused because the kernel checks t < *d_num_unique.
        {
            size_t bytes_scan = sb.scan_temp_runs_bytes;
            cub::DeviceScan::ExclusiveSum(sb.d_scan_temp_runs, bytes_scan,
                sb.d_run_lengths, sb.d_run_offsets, n_ram + 1, st);
        }
        // Per-segment state machine: one thread per unique RAM address.
        // Use g_ram as a conservative upper bound (n_unique <= n_ram); kernel
        // early-exits via *d_num_unique.
        state_machine_by_run_kernel<<<g_ram, BLOCK, 0, st>>>(
            sb.d_run_offsets, sb.d_num_unique, sb.d_ram_vals_sorted, sb.d_emit_bits);
    }

    {
        size_t bytes = sb.scan_temp_emit_bytes;
        cub::DeviceScan::ExclusiveSum(sb.d_scan_temp_emit, bytes,
            sb.d_emit_bits, sb.d_final_offsets, n_potentials + 1, st);
    }

    // Record actual emit count (async) into this chunk's own pinned slot.
    CUDA_CHECK(cudaMemcpyAsync(h_n_emits_slot,
        sb.d_final_offsets + n_potentials, sizeof(uint32_t),
        cudaMemcpyDeviceToHost, st));

    compact_kernel<<<g_pot, BLOCK, 0, st>>>(
        sb.d_potentials, sb.d_emit_bits, sb.d_final_offsets, n_potentials, sb.d_out);

    if (h_out_chunk != nullptr) {
        // Over-copy up to n_potentials entries. First *h_n_emits_slot of
        // them are valid; the rest are uninitialised slots in d_out.
        CUDA_CHECK(cudaMemcpyAsync(h_out_chunk, sb.d_out,
            sizeof(uint32_t) * (size_t)n_potentials, cudaMemcpyDeviceToHost, st));
    }
}

// ---------------------------------------------------------------------
// main
// ---------------------------------------------------------------------

int main(int argc, char** argv) {
    if (argc < 2) {
        fprintf(stderr,
                "Usage: %s <block_number> [--verify|--verify-files] [--no-d2h]"
                " [--save-counters <dir>]\n",
                argv[0]);
        return 1;
    }
    const std::string block = argv[1];
    bool do_verify = false, do_verify_files = false, no_d2h = false;
    std::string counters_dir;
    for (int i = 2; i < argc; i++) {
        if (strcmp(argv[i], "--verify") == 0) do_verify = true;
        else if (strcmp(argv[i], "--verify-files") == 0) do_verify_files = true;
        else if (strcmp(argv[i], "--no-d2h") == 0) no_d2h = true;
        else if (strcmp(argv[i], "--save-counters") == 0 && i + 1 < argc) {
            counters_dir = argv[++i];
        }
        else { fprintf(stderr, "unknown arg: %s\n", argv[i]); return 1; }
    }
    if (no_d2h && (do_verify || do_verify_files)) {
        fprintf(stderr, "--no-d2h skips the output copy; verify has nothing to compare\n");
        return 1;
    }
    const std::string raw_dir = "data/" + block + "_raw";

    auto raw_idxs = list_indices(raw_dir, "mem_count_data_");
    const uint32_t n_chunks = raw_idxs.size();
    std::cout << "Discovered " << n_chunks << " chunks for block " << block << std::endl;
    if (n_chunks == 0) {
        fprintf(stderr, "ERROR: no mem_count_data_*.bin files in %s\n", raw_dir.c_str());
        return 1;
    }

    std::vector<ChunkRef> chunks(n_chunks);
    size_t total_memops = 0;
    for (uint32_t c = 0; c < n_chunks; c++) {
        chunks[c].file_idx = raw_idxs[c];
        char rp[512]; snprintf(rp, sizeof(rp), "%s/mem_count_data_%u.bin", raw_dir.c_str(), raw_idxs[c]);
        size_t r_bytes = file_size(rp);
        if (r_bytes % sizeof(MemOp) != 0) { fprintf(stderr, "ERROR: bad file size\n"); return 1; }
        chunks[c].n_memops     = r_bytes / sizeof(MemOp);
        chunks[c].memop_offset = (uint32_t)total_memops;
        total_memops += chunks[c].n_memops;
        if (chunks[c].n_memops > CHUNK_MAX_MEMOPS) {
            fprintf(stderr, "ERROR: chunk %u has %u > CHUNK_MAX_MEMOPS\n", c, chunks[c].n_memops);
            return 1;
        }
    }
    std::cout << "Total memops: " << total_memops << std::endl;

    MemOp* h_memops = nullptr;
    CUDA_CHECK(cudaMallocHost(&h_memops, sizeof(MemOp) * total_memops));
    for (uint32_t c = 0; c < n_chunks; c++) {
        char rp[512]; snprintf(rp, sizeof(rp), "%s/mem_count_data_%u.bin", raw_dir.c_str(), raw_idxs[c]);
        FILE* f = fopen(rp, "rb");
        if (!f) { fprintf(stderr, "ERROR: cannot open %s\n", rp); return 1; }
        size_t got = fread(h_memops + chunks[c].memop_offset, sizeof(MemOp), chunks[c].n_memops, f);
        fclose(f);
        if (got != chunks[c].n_memops) { fprintf(stderr, "ERROR: short read %s\n", rp); return 1; }
    }
    std::cout << "Loaded raw memops" << std::endl;

    // Without persistence we don't know each chunk's actual emit count up
    // front. Per chunk, compute on CPU:
    //   n_potentials = total potential emissions (upper bound for out buffer,
    //                  sort and scan sizes)
    //   n_ram        = number of those that hit the RAM range and therefore
    //                  need to go through the sort + state_machine path
    // Both use the same decode table as the GPU decode kernels.
    std::vector<size_t>   out_offsets(n_chunks + 1, 0);
    std::vector<uint32_t> n_ram_per_chunk(n_chunks, 0);
    auto add_pot = [](uint32_t addr, uint32_t count, size_t& pot, uint32_t& ram) {
        pot += count;
        if (is_ram_addr(addr)) ram += count;
    };
    for (uint32_t c = 0; c < n_chunks; c++) {
        size_t   pot = 0;
        uint32_t ram = 0;
        const MemOp* ops = h_memops + chunks[c].memop_offset;
        for (uint32_t k = 0; k < chunks[c].n_memops; k++) {
            const MemOp& op = ops[k];
            const uint32_t addr    = op.addr;
            const uint32_t aligned = addr & ZISK_ALIGN_MASK;
            const uint8_t  mode    = op.flags & 0x3F;
            const uint32_t off     = addr & 0x07;
            switch (mode) {
                case MOPS_READ_1:
                    add_pot(aligned, 1, pot, ram); break;
                case MOPS_CWRITE_1: case MOPS_WRITE_1:
                    add_pot(aligned, 2, pot, ram); break;
                case MOPS_READ_2:
                    add_pot(aligned, 1, pot, ram);
                    if (off > 6) add_pot(aligned + 8, 1, pot, ram); break;
                case MOPS_WRITE_2:
                    add_pot(aligned, 2, pot, ram);
                    if (off > 6) add_pot(aligned + 8, 2, pot, ram); break;
                case MOPS_READ_4:
                    add_pot(aligned, 1, pot, ram);
                    if (off > 4) add_pot(aligned + 8, 1, pot, ram); break;
                case MOPS_WRITE_4:
                    add_pot(aligned, 2, pot, ram);
                    if (off > 4) add_pot(aligned + 8, 2, pot, ram); break;
                case MOPS_READ_8:
                    add_pot(aligned, 1, pot, ram);
                    if (off > 0) add_pot(aligned + 8, 1, pot, ram); break;
                case MOPS_WRITE_8:
                    if (addr == aligned) add_pot(aligned, 1, pot, ram);
                    else { add_pot(aligned, 2, pot, ram);
                           add_pot(aligned + 8, 2, pot, ram); } break;
                case MOPS_ALIGNED_READ  + 0x00: case MOPS_ALIGNED_READ  + 0x10:
                case MOPS_ALIGNED_READ  + 0x20: case MOPS_ALIGNED_READ  + 0x30:
                case MOPS_ALIGNED_WRITE + 0x00: case MOPS_ALIGNED_WRITE + 0x10:
                case MOPS_ALIGNED_WRITE + 0x20: case MOPS_ALIGNED_WRITE + 0x30:
                    add_pot(addr, 1, pot, ram); break;
                default: {
                    uint32_t cnt = op.flags >> MOPS_BLOCK_COUNT_SBITS;
                    // Block ops emit 'cnt' potentials at addr, addr+8, addr+16, ...
                    // They sit on contiguous 8-byte boundaries and never cross
                    // a region, so all cnt hits share is_ram_addr(addr).
                    add_pot(addr, cnt, pot, ram); break;
                }
            }
        }
        if (pot > POTENTIAL_CAP_PER_CHUNK) {
            fprintf(stderr, "ERROR: chunk %u needs %zu potentials > cap %u\n",
                    c, pot, POTENTIAL_CAP_PER_CHUNK);
            exit(1);
        }
        out_offsets[c + 1] = out_offsets[c] + pot;
        n_ram_per_chunk[c] = ram;
    }
    size_t total_bound = out_offsets[n_chunks];
    uint32_t* h_out_all = nullptr;
    CUDA_CHECK(cudaMallocHost(&h_out_all, sizeof(uint32_t) * total_bound));
    std::vector<uint32_t> h_out_n(n_chunks, 0);
    std::cout << "Pinned output bound: " << (total_bound * 4) / (1024*1024) << " MB" << std::endl;

    StreamBufs sb[ZISK_N_STREAMS];
    for (int i = 0; i < ZISK_N_STREAMS; i++) alloc_stream_bufs(sb[i]);

    ChunkCounters* d_chunk_counters = nullptr;
    CUDA_CHECK(cudaMalloc(&d_chunk_counters, sizeof(ChunkCounters) * n_chunks));
    // Reset is one-shot (matches the one-shot lifecycle of this binary).
    // If you wrap this in a worker-loop pattern (see main_real.cu's
    // --save-metas), reset before each pass instead.
    CUDA_CHECK(cudaMemset(d_chunk_counters, 0, sizeof(ChunkCounters) * n_chunks));

    // Sticky global flag: any chunk seeing an unrecognised opcode atomically
    // ORs 1 into here. We read it once at the end and abort hard if set.
    // mem_counter_single.cpp throws on unknown modes; without this flag, the
    // GPU and CPU oracle would silently agree on (wrong) zero emissions.
    uint32_t* d_invalid_mode_flag = nullptr;
    CUDA_CHECK(cudaMalloc(&d_invalid_mode_flag, sizeof(uint32_t)));
    CUDA_CHECK(cudaMemset(d_invalid_mode_flag, 0, sizeof(uint32_t)));

    // Per-chunk pinned size slots (one uint32_t each), populated async from
    // device to host during the pipeline. No per-chunk host sync.
    uint32_t* h_n_emits_all = nullptr;
    CUDA_CHECK(cudaMallocHost(&h_n_emits_all, sizeof(uint32_t) * n_chunks));

    auto t0 = std::chrono::steady_clock::now();
    for (uint32_t c = 0; c < n_chunks; c++) {
        int s = c % ZISK_N_STREAMS;
        uint32_t n_pot = (uint32_t)(out_offsets[c + 1] - out_offsets[c]);
        run_chunk(sb[s],
                  h_memops + chunks[c].memop_offset,
                  chunks[c].n_memops,
                  n_pot,
                  n_ram_per_chunk[c],
                  d_chunk_counters + c,
                  d_invalid_mode_flag,
                  no_d2h ? nullptr : (h_out_all + out_offsets[c]),
                  &h_n_emits_all[c]);
    }
    for (int i = 0; i < ZISK_N_STREAMS; i++)
        CUDA_CHECK(cudaStreamSynchronize(sb[i].stream));
    auto t1 = std::chrono::steady_clock::now();
    double ms = std::chrono::duration<double, std::milli>(t1 - t0).count();
    std::cout << "GPU pipeline: " << ms << " ms total ("
              << (ms / n_chunks) << " ms/chunk avg)" << std::endl;

    // Hard-fail if any chunk had a corrupted opcode. CPU reference throws on
    // unknown modes (see zisk/mem_counter_single.cpp:153-157); we mirror that
    // by aborting here. This catches silent-wrong-answer scenarios that
    // verify against the CPU oracle would otherwise miss.
    uint32_t h_invalid = 0;
    CUDA_CHECK(cudaMemcpy(&h_invalid, d_invalid_mode_flag, sizeof(uint32_t),
                          cudaMemcpyDeviceToHost));
    if (h_invalid != 0) {
        fprintf(stderr, "FATAL: at least one memop carried an unrecognised "
                        "opcode (flags & 0x3F not in MOPS_* set). "
                        "Refusing to produce silent-wrong output.\n");
        return 1;
    }

    // Publish per-chunk actual emit counts to h_out_n.
    for (uint32_t c = 0; c < n_chunks; c++) h_out_n[c] = h_n_emits_all[c];

    // Per-chunk counters: D2H and write one file per chunk only when the
    // caller asked for them via --save-counters. Each file is exactly 20 bytes
    // (5 little-endian uint32s: full_5, full_3, full_2, read_byte, write_byte).
    if (!counters_dir.empty()) {
        std::vector<ChunkCounters> h_chunk_counters(n_chunks);
        CUDA_CHECK(cudaMemcpy(h_chunk_counters.data(), d_chunk_counters,
                              sizeof(ChunkCounters) * n_chunks,
                              cudaMemcpyDeviceToHost));

        // Best-effort mkdir -p; fall through if it already exists.
        mkdir(counters_dir.c_str(), 0755);
        for (uint32_t c = 0; c < n_chunks; c++) {
            char p[1024];
            snprintf(p, sizeof(p), "%s/mem_counters_%u.bin",
                     counters_dir.c_str(), chunks[c].file_idx);
            FILE* f = fopen(p, "wb");
            if (!f) {
                fprintf(stderr, "ERROR: cannot open %s for write\n", p);
                return 1;
            }
            const ChunkCounters& cc = h_chunk_counters[c];
            uint32_t buf[5] = { cc.full_5, cc.full_3, cc.full_2,
                                cc.read_byte, cc.write_byte };
            if (fwrite(buf, sizeof(buf), 1, f) != 1) {
                fprintf(stderr, "ERROR: short write %s\n", p);
                fclose(f);
                return 1;
            }
            fclose(f);
        }
        std::cout << "Wrote " << n_chunks << " counter files → "
                  << counters_dir << "/mem_counters_*.bin\n";
    }

    if (do_verify) {
        std::vector<bool>     free_r(ZISK_RAM_SIZE_BYTES / 8, false);
        std::vector<uint32_t> cpu_out;
        uint32_t n_ok = 0, n_fail = 0;
        for (uint32_t c = 0; c < n_chunks; c++) {
            run_cpu_chunk(h_memops + chunks[c].memop_offset, chunks[c].n_memops,
                          free_r, cpu_out);
            const uint32_t* gpu_out = h_out_all + out_offsets[c];
            bool ok = (h_out_n[c] == cpu_out.size())
                   && (memcmp(gpu_out, cpu_out.data(), cpu_out.size() * sizeof(uint32_t)) == 0);
            if (ok) { n_ok++; }
            else {
                n_fail++;
                if (n_fail <= 3) {
                    fprintf(stderr, "MISMATCH chunk %u: gpu=%u cpu=%zu\n",
                            c, h_out_n[c], cpu_out.size());
                    size_t cmp_n = std::min((size_t)h_out_n[c], cpu_out.size());
                    for (size_t i = 0; i < cmp_n; i++) {
                        if (gpu_out[i] != cpu_out[i]) {
                            fprintf(stderr, "  first diff @ %zu: gpu=0x%08x cpu=0x%08x\n",
                                    i, gpu_out[i], cpu_out[i]);
                            break;
                        }
                    }
                }
            }
        }
        std::cout << "VERIFY: OK " << n_ok << "/" << n_chunks << " chunks match CPU oracle" << std::endl;
        if (n_fail > 0) std::cout << "FAIL: " << n_fail << std::endl;
    } else {
        std::cout << "(CPU-oracle verify skipped; pass --verify to enable)" << std::endl;
    }

    if (do_verify_files) {
        const std::string aligned_dir = "data/" + block + "_aligned";
        auto aligned_idxs = list_indices(aligned_dir, "mem_aligned_");
        if (aligned_idxs.size() != n_chunks) {
            fprintf(stderr, "ERROR: aligned file count %zu != chunks %u\n",
                    aligned_idxs.size(), n_chunks);
            return 1;
        }
        uint32_t n_ok = 0, n_fail = 0;
        std::vector<uint32_t> expected;
        for (uint32_t c = 0; c < n_chunks; c++) {
            char ap[512];
            snprintf(ap, sizeof(ap), "%s/mem_aligned_%u.bin", aligned_dir.c_str(), aligned_idxs[c]);
            size_t bytes = file_size(ap);
            if (bytes % sizeof(uint32_t) != 0) {
                fprintf(stderr, "ERROR: %s size %zu not multiple of 4\n", ap, bytes); return 1;
            }
            expected.resize(bytes / sizeof(uint32_t));
            FILE* f = fopen(ap, "rb");
            if (!f) { fprintf(stderr, "ERROR: open %s\n", ap); return 1; }
            size_t got = fread(expected.data(), sizeof(uint32_t), expected.size(), f);
            fclose(f);
            if (got != expected.size()) { fprintf(stderr, "ERROR: short read %s\n", ap); return 1; }
            const uint32_t* gpu_out = h_out_all + out_offsets[c];
            bool ok = (h_out_n[c] == expected.size())
                   && (memcmp(gpu_out, expected.data(), expected.size() * sizeof(uint32_t)) == 0);
            if (ok) { n_ok++; }
            else {
                n_fail++;
                if (n_fail <= 3) {
                    fprintf(stderr, "FILE MISMATCH chunk %u (file %u): gpu=%u file=%zu\n",
                            c, aligned_idxs[c], h_out_n[c], expected.size());
                    size_t cmp_n = std::min((size_t)h_out_n[c], expected.size());
                    for (size_t i = 0; i < cmp_n; i++) {
                        if (gpu_out[i] != expected[i]) {
                            fprintf(stderr, "  first diff @ %zu: gpu=0x%08x file=0x%08x\n",
                                    i, gpu_out[i], expected[i]);
                            break;
                        }
                    }
                }
            }
        }
        std::cout << "VERIFY (files): OK " << n_ok << "/" << n_chunks
                  << " chunks match data/" << block << "_aligned/" << std::endl;
        if (n_fail > 0) std::cout << "FAIL: " << n_fail << std::endl;
    }

    cudaFree(d_chunk_counters);
    cudaFree(d_invalid_mode_flag);
    cudaFreeHost(h_n_emits_all);
    cudaFreeHost(h_out_all);
    cudaFreeHost(h_memops);
    for (int i = 0; i < ZISK_N_STREAMS; i++) free_stream_bufs(sb[i]);
    return 0;
}
