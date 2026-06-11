// Layout of an InstanceMeta as produced by the GPU `CountAndPlan` pipeline.

#pragma once

#include <cstdint>

// Paged-dense offsets layout constants.
//
// The values below MUST match with:
//   mem-common::MEM_OFFSETS_PAGE_SIZE
//   mem-common::MEM_OFFSETS_PAGE_ABSENT
//
// There is no compile-time cross-language check today; if you change one, change the other.
#define MEM_OFFSETS_PAGE_SIZE 1024u
#define MEM_OFFSETS_PAGE_ABSENT 0xFFFFFFFFu

// Paged-dense cumulative-offset table. POD; shared by the CUDA pipeline,
// host C++, the on-disk loader, and (mirrored) the Rust FFI.
//
// WIRE/FFI-significant: field order & types must match
// `mem-cpp/src/gpu_bindings.rs::PagedOffsets` and the per-field
// serialisation in `instance_meta_loader.hpp` exactly. Pointers come
// first so the struct has no interior padding (3×8 + 3×4 + 4 tail pad
// = 40 bytes, 8-aligned). The size_of assert on both sides catches an
// accidental add/remove of a field.
//
//   num_pages          — ceil(addr_range_slots / MEM_OFFSETS_PAGE_SIZE)
//   present_count      — number of non-absent pages
//   addr_range_slots   — (last_addr - first_addr)/8 + 1, dense slot count
//   page_starts[p]     — MEM_OFFSETS_PAGE_ABSENT iff page p is absent
//                        (uniform value = page_single_value[p]); otherwise
//                        the present-page index into `pages_dense`
//   page_single_value[p] — the value held by every slot in page p
//                          (the only value for absent pages, ignore if present)
//   pages_dense        — concatenated present-page slot data; the slice for
//                        a present page p is at
//                        pages_dense[page_starts[p] * MEM_OFFSETS_PAGE_SIZE
//                                   .. (page_starts[p]+1) * MEM_OFFSETS_PAGE_SIZE].
//                        Length = present_count * MEM_OFFSETS_PAGE_SIZE; the
//                        last partial page is padded with its carry value.
struct PagedOffsets {
    const uint32_t* page_starts;
    const uint32_t* page_single_value;
    const uint32_t* pages_dense;
    uint32_t        num_pages;
    uint32_t        present_count;
    uint32_t        addr_range_slots;
};
static_assert(sizeof(PagedOffsets) == 40,
              "PagedOffsets layout changed — update gpu_bindings.rs::PagedOffsets "
              "and the instance_meta_loader.hpp serialisation to match");

// One InstanceMeta describes a single planner instance produced by the GPU
// `CountAndPlan` pipeline. The struct is POD and laid out for the C ABI;
// pointer fields reference pinned host buffers owned by the planner.
//
//   inst_id            — instance index
//   kind               — 0 = ROM, 1 = INPUT, 2 = RAM
//   first_addr         — fist byte address in the instance
//   last_addr          — last byte address in the instance
//   count_per_chunk[c] — instance rows filled from chunk c (length n_chunks)
//   n_chunks           — total number of chunks
//   offsets            — paged cumulative-offset table (see PagedOffsets)
//   first_addr_chunk   — chunk that contains first_addr
//   first_addr_skip    — skip count within that chunk
//   last_addr_chunk    — chunk that contains last_addr
//   last_addr_include  — count to include from the last chunk
struct InstanceMeta {
    uint32_t inst_id;
    uint32_t kind;
    uint32_t first_addr;
    uint32_t last_addr;
    const uint32_t* count_per_chunk;
    uint32_t        n_chunks;
    PagedOffsets    offsets;
    uint32_t first_addr_chunk;
    uint32_t first_addr_skip;
    uint32_t last_addr_chunk;
    uint32_t last_addr_include;
};
