// instance_meta_loader.hpp
//
// Self-contained loader for InstanceMeta files produced by main_real.cu's
// --save-metas option.
//
// File format (binary, little-endian):
//
//   uint32_t num_metas
//   for each meta:
//     uint32_t inst_id
//     uint8_t  type             (0=ROM, 1=INPUT, 2=RAM)
//     uint8_t  pad[3]
//     uint32_t first_addr
//     uint32_t last_addr
//     uint32_t first_addr_chunk
//     uint32_t first_addr_skip
//     uint32_t last_addr_chunk
//     uint32_t last_addr_include
//     uint32_t count_per_chunk_size
//     uint32_t addr_offsets_size
//     uint32_t count_per_chunk[count_per_chunk_size]
//     uint32_t addr_offsets[addr_offsets_size]
//
// Usage:
//   #include "instance_meta_loader.hpp"
//   LoadedMetas L = load_instance_metas("metas.bin");
//   for (const InstanceMeta& m : L.metas) {
//       // ... use m.inst_id, m.type, m.count_per_chunk, m.addr_offsets, ...
//   }
//   // L owns the backing storage for the spans; do not move/destroy L while
//   // any meta is still in use.
//
// Requires C++20 (std::span). Header-only, no external dependencies.

#ifndef INSTANCE_META_LOADER_HPP
#define INSTANCE_META_LOADER_HPP

#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <span>
#include <stdexcept>
#include <string>
#include <vector>

// Mirror of the InstanceMeta struct produced by main_real.cu. The two spans
// view contiguous slices of LoadedMetas::count_storage / offset_storage; the
// caller must keep the LoadedMetas alive while iterating the metas.
struct InstanceMeta {
    uint32_t inst_id;
    uint8_t  type;                                 // 0=ROM, 1=INPUT, 2=RAM
    uint32_t first_addr;
    uint32_t last_addr;
    std::span<const uint32_t> count_per_chunk;
    std::span<uint32_t> addr_offsets;
    uint32_t first_addr_chunk;
    uint32_t first_addr_skip;
    uint32_t last_addr_chunk;
    uint32_t last_addr_include;
};

// Owns the backing storage that the spans inside `metas` point into.
// `metas[i].count_per_chunk` is a view into `count_storage` starting at
// `cnt_offsets[i]`; same for `addr_offsets` / `offset_storage` / `aos_offsets`.
struct LoadedMetas {
    std::vector<InstanceMeta> metas;
    std::vector<uint32_t>     count_storage;
    std::vector<uint32_t>     offset_storage;
    std::vector<std::size_t>  cnt_offsets;
    std::vector<std::size_t>  aos_offsets;
};

inline LoadedMetas load_instance_metas(const std::string& path) {
    std::FILE* f = std::fopen(path.c_str(), "rb");
    if (!f) throw std::runtime_error("cannot open " + path);

    auto rd = [&](void* p, std::size_t bytes) {
        if (std::fread(p, 1, bytes, f) != bytes) {
            std::fclose(f);
            throw std::runtime_error("short read on " + path);
        }
    };

    uint32_t n;
    rd(&n, sizeof(uint32_t));

    LoadedMetas out;
    out.metas.reserve(n);
    out.cnt_offsets.reserve(n);
    out.aos_offsets.reserve(n);

    // Single pass: read into temporaries, append to bundle storage, record
    // offsets. Span binding is deferred to a final pass below so that vector
    // re-allocations during inserts don't invalidate already-bound spans.
    std::vector<uint32_t> tmp_cnt;
    std::vector<uint32_t> tmp_aos;
    InstanceMeta m{};
    for (uint32_t i = 0; i < n; i++) {
        uint8_t type, pad[3];
        rd(&m.inst_id,           sizeof(uint32_t));
        rd(&type, 1);
        rd(pad, 3);
        m.type = type;
        rd(&m.first_addr,        sizeof(uint32_t));
        rd(&m.last_addr,         sizeof(uint32_t));
        rd(&m.first_addr_chunk,  sizeof(uint32_t));
        rd(&m.first_addr_skip,   sizeof(uint32_t));
        rd(&m.last_addr_chunk,   sizeof(uint32_t));
        rd(&m.last_addr_include, sizeof(uint32_t));
        uint32_t cps, aos;
        rd(&cps, sizeof(uint32_t));
        rd(&aos, sizeof(uint32_t));
        tmp_cnt.resize(cps);
        tmp_aos.resize(aos);
        rd(tmp_cnt.data(), cps * sizeof(uint32_t));
        rd(tmp_aos.data(), aos * sizeof(uint32_t));

        out.cnt_offsets.push_back(out.count_storage.size());
        out.aos_offsets.push_back(out.offset_storage.size());
        out.count_storage.insert(out.count_storage.end(),
                                  tmp_cnt.begin(), tmp_cnt.end());
        out.offset_storage.insert(out.offset_storage.end(),
                                   tmp_aos.begin(), tmp_aos.end());
        out.metas.push_back(m);
    }
    std::fclose(f);

    // Bind spans now that count_storage / offset_storage are final.
    for (std::size_t i = 0; i < out.metas.size(); i++) {
        std::size_t cnt_size = (i + 1 < out.cnt_offsets.size())
            ? out.cnt_offsets[i + 1] - out.cnt_offsets[i]
            : out.count_storage.size() - out.cnt_offsets[i];
        std::size_t aos_size = (i + 1 < out.aos_offsets.size())
            ? out.aos_offsets[i + 1] - out.aos_offsets[i]
            : out.offset_storage.size() - out.aos_offsets[i];
        out.metas[i].count_per_chunk = std::span<const uint32_t>(
            out.count_storage.data() + out.cnt_offsets[i], cnt_size);
        out.metas[i].addr_offsets = std::span<uint32_t>(
            out.offset_storage.data() + out.aos_offsets[i], aos_size);
    }
    return out;
}

#endif  // INSTANCE_META_LOADER_HPP