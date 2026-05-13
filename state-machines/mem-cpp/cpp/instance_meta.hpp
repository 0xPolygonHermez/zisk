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
    const uint32_t* addr_offsets;
    uint32_t        addr_offsets_size;
    uint32_t first_addr_chunk;
    uint32_t first_addr_skip;
    uint32_t last_addr_chunk;
    uint32_t last_addr_include;
};
