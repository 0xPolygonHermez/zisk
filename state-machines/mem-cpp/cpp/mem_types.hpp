#ifndef __MEM_TYPES_HPP__
#define __MEM_TYPES_HPP__

#include <stdint.h>
#include "mem_config.hpp"
// One decoded memory-ops record. The stream itself is tagged 8-byte words: see
// `mops_record_len` / `mops_decode_record`.
struct MemCountersBusData {
    uint32_t addr;
    uint32_t flags;
} __attribute__((packed));

// Decodes the header of the record at `w` (the counters need no payload). A value block is
// reported as an aligned block write of its words.
static inline MemCountersBusData mops_decode_record(const uint64_t *w) {
    const uint64_t hdr = w[0];
    MemCountersBusData rec;
    rec.addr = (uint32_t)hdr;
    rec.flags = (uint32_t)(hdr >> 32) & 0x3FFFFFFF;
    if ((rec.flags & 0x0F) == MOPS_BLOCK_VALUES) {
        rec.flags = MOPS_ALIGNED_BLOCK_WRITE | ((uint32_t)((hdr >> 36) & 63) << MOPS_BLOCK_COUNT_SBITS);
    }
    return rec;
}

struct MemChunk {
    MemCountersBusData *data;   // the chunk's stream words
    uint32_t count;             // words
};

struct MemCountTrace {
    MemCountersBusData *chunk_data[MAX_CHUNKS];
    uint32_t chunk_size[MAX_CHUNKS];
    uint32_t chunks = 0;
};


#endif