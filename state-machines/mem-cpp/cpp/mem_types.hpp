#ifndef __MEM_TYPES_HPP__
#define __MEM_TYPES_HPP__

#include <stdint.h>
#include "mem_config.hpp"

struct MemCountersBusData {
    uint32_t addr;
    uint32_t flags;
};

struct MemChunk {
    MemCountersBusData *data;
    uint32_t count;
};

struct MemCountTrace {
    MemCountersBusData *chunk_data[MAX_CHUNKS];
    uint32_t chunk_size[MAX_CHUNKS];
    uint32_t chunks = 0;
};


#endif