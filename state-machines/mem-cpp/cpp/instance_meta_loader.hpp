// Loader for the binary `metas.bin` format produced by the standalone GPU
// runner. Produces a `LoadedMetas` whose `metas[]` use the same
// `InstanceMeta` layout the in-process GPU pipeline does — pointer + size
// instead of std::span — so downstream consumers can be written once.
//
// On-disk format (little-endian, packed, in order):
//
//   uint32_t num_metas
//   for each meta:
//     uint32_t inst_id
//     uint32_t kind                       // 0=ROM, 1=INPUT, 2=RAM
//     uint32_t first_addr
//     uint32_t last_addr
//     uint32_t first_addr_chunk
//     uint32_t first_addr_skip
//     uint32_t last_addr_chunk
//     uint32_t last_addr_include
//     uint32_t n_chunks
//     uint32_t addr_offsets_size
//     uint32_t count_per_chunk[n_chunks]
//     uint32_t addr_offsets[addr_offsets_size]
//
// `LoadedMetas` owns the backing storage; do not move/destroy it while any
// pointer inside `metas[i].count_per_chunk` / `addr_offsets` is in use.

#ifndef INSTANCE_META_LOADER_HPP
#define INSTANCE_META_LOADER_HPP

#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <stdexcept>
#include <string>
#include <vector>

#include "instance_meta.hpp"

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

    // Append into bundle storage first; pointers are bound below so vector
    // re-allocations during inserts can't invalidate already-bound pointers.
    std::vector<uint32_t> tmp_cnt;
    std::vector<uint32_t> tmp_aos;
    InstanceMeta m{};
    for (uint32_t i = 0; i < n; i++) {
        rd(&m.inst_id,           sizeof(uint32_t));
        rd(&m.kind,              sizeof(uint32_t));
        rd(&m.first_addr,        sizeof(uint32_t));
        rd(&m.last_addr,         sizeof(uint32_t));
        rd(&m.first_addr_chunk,  sizeof(uint32_t));
        rd(&m.first_addr_skip,   sizeof(uint32_t));
        rd(&m.last_addr_chunk,   sizeof(uint32_t));
        rd(&m.last_addr_include, sizeof(uint32_t));
        rd(&m.n_chunks,          sizeof(uint32_t));
        rd(&m.addr_offsets_size, sizeof(uint32_t));
        tmp_cnt.resize(m.n_chunks);
        tmp_aos.resize(m.addr_offsets_size);
        rd(tmp_cnt.data(), m.n_chunks            * sizeof(uint32_t));
        rd(tmp_aos.data(), m.addr_offsets_size  * sizeof(uint32_t));

        out.cnt_offsets.push_back(out.count_storage.size());
        out.aos_offsets.push_back(out.offset_storage.size());
        out.count_storage.insert(out.count_storage.end(),
                                  tmp_cnt.begin(), tmp_cnt.end());
        out.offset_storage.insert(out.offset_storage.end(),
                                   tmp_aos.begin(), tmp_aos.end());
        out.metas.push_back(m);
    }
    std::fclose(f);

    // Bind pointers now that count_storage / offset_storage are final.
    for (std::size_t i = 0; i < out.metas.size(); i++) {
        out.metas[i].count_per_chunk = out.count_storage.data() + out.cnt_offsets[i];
        out.metas[i].addr_offsets    = out.offset_storage.data() + out.aos_offsets[i];
    }
    return out;
}

#endif  // INSTANCE_META_LOADER_HPP
