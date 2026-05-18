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
//     uint32_t kind                          // 0=ROM, 1=INPUT, 2=RAM
//     uint32_t first_addr
//     uint32_t last_addr
//     uint32_t first_addr_chunk
//     uint32_t first_addr_skip
//     uint32_t last_addr_chunk
//     uint32_t last_addr_include
//     uint32_t n_chunks
//     uint32_t num_pages
//     uint32_t present_count
//     uint32_t addr_range_slots              // = (last_addr - first_addr)/8 + 1
//     uint32_t count_per_chunk[n_chunks]
//     uint32_t page_starts[num_pages]
//     uint32_t page_single_value[num_pages]
//     uint32_t pages_dense[present_count * MEM_OFFSETS_PAGE_SIZE]
//
// Wire-format version: paged v1 (incompatible with the dense
// `addr_offsets[addr_offsets_size]` and sparse-soa formats).
//
// `LoadedMetas` owns the backing storage; do not move/destroy it while any
// pointer inside `metas[i]` is in use.

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
    std::vector<uint32_t>     page_starts_storage;
    std::vector<uint32_t>     page_single_storage;
    std::vector<uint32_t>     pages_dense_storage;
    std::vector<std::size_t>  cnt_offsets;
    std::vector<std::size_t>  page_starts_offsets;
    std::vector<std::size_t>  page_single_offsets;
    std::vector<std::size_t>  pages_dense_offsets;
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
    out.page_starts_offsets.reserve(n);
    out.page_single_offsets.reserve(n);
    out.pages_dense_offsets.reserve(n);

    std::vector<uint32_t> tmp_cnt;
    std::vector<uint32_t> tmp_starts;
    std::vector<uint32_t> tmp_carry;
    std::vector<uint32_t> tmp_dense;
    InstanceMeta m{};
    for (uint32_t i = 0; i < n; i++) {
        rd(&m.inst_id,            sizeof(uint32_t));
        rd(&m.kind,               sizeof(uint32_t));
        rd(&m.first_addr,         sizeof(uint32_t));
        rd(&m.last_addr,          sizeof(uint32_t));
        rd(&m.first_addr_chunk,   sizeof(uint32_t));
        rd(&m.first_addr_skip,    sizeof(uint32_t));
        rd(&m.last_addr_chunk,    sizeof(uint32_t));
        rd(&m.last_addr_include,  sizeof(uint32_t));
        rd(&m.n_chunks,                 sizeof(uint32_t));
        rd(&m.offsets.num_pages,        sizeof(uint32_t));
        rd(&m.offsets.present_count,    sizeof(uint32_t));
        rd(&m.offsets.addr_range_slots, sizeof(uint32_t));
        const std::size_t dense_words =
            static_cast<std::size_t>(m.offsets.present_count) * MEM_OFFSETS_PAGE_SIZE;
        tmp_cnt.resize(m.n_chunks);
        tmp_starts.resize(m.offsets.num_pages);
        tmp_carry.resize(m.offsets.num_pages);
        tmp_dense.resize(dense_words);
        rd(tmp_cnt.data(),    m.n_chunks            * sizeof(uint32_t));
        rd(tmp_starts.data(), m.offsets.num_pages   * sizeof(uint32_t));
        rd(tmp_carry.data(),  m.offsets.num_pages   * sizeof(uint32_t));
        rd(tmp_dense.data(),  dense_words           * sizeof(uint32_t));

        out.cnt_offsets.push_back(out.count_storage.size());
        out.page_starts_offsets.push_back(out.page_starts_storage.size());
        out.page_single_offsets.push_back(out.page_single_storage.size());
        out.pages_dense_offsets.push_back(out.pages_dense_storage.size());
        out.count_storage.insert(out.count_storage.end(), tmp_cnt.begin(), tmp_cnt.end());
        out.page_starts_storage.insert(out.page_starts_storage.end(), tmp_starts.begin(), tmp_starts.end());
        out.page_single_storage.insert(out.page_single_storage.end(), tmp_carry.begin(), tmp_carry.end());
        out.pages_dense_storage.insert(out.pages_dense_storage.end(), tmp_dense.begin(), tmp_dense.end());
        out.metas.push_back(m);
    }
    std::fclose(f);

    // Bind pointers now that backing vectors are final.
    for (std::size_t i = 0; i < out.metas.size(); i++) {
        out.metas[i].count_per_chunk           = out.count_storage.data()        + out.cnt_offsets[i];
        out.metas[i].offsets.page_starts       = out.page_starts_storage.data()  + out.page_starts_offsets[i];
        out.metas[i].offsets.page_single_value = out.page_single_storage.data()  + out.page_single_offsets[i];
        out.metas[i].offsets.pages_dense       = out.pages_dense_storage.data()  + out.pages_dense_offsets[i];
    }
    return out;
}

#endif  // INSTANCE_META_LOADER_HPP
