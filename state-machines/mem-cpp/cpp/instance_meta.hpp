// Layout of an InstanceMeta as produced by the GPU `CountAndPlan` pipeline.

#pragma once

#include <cstdint>

struct InstanceMeta {
    uint32_t inst_id;
    uint32_t kind;             // 0=ROM, 1=INPUT, 2=RAM
    uint32_t first_addr;
    uint32_t last_addr;
    const uint32_t* count_per_chunk;
    uint32_t        n_chunks;
    // Sparse offsets: two parallel arrays, each of length `offset_changes_count`.
    // `offset_change_slots[]` is strictly increasing slot indices (slot 0 of the
    // address range is always present, so `offset_change_slots[0] == 0`);
    // `offset_change_values[]` is the value that holds from each change point
    // up to (but not including) the next change point. Total slot count is
    // `addr_range_slots = (last_addr - first_addr)/8 + 1`.
    const uint32_t* offset_change_slots;
    const uint32_t* offset_change_values;
    uint32_t        offset_changes_count;
    uint32_t        addr_range_slots;
    uint32_t first_addr_chunk;
    uint32_t first_addr_skip;
    uint32_t last_addr_chunk;
    uint32_t last_addr_include;
};
