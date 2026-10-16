#ifndef __MEM_TYPES_HPP__
#define __MEM_TYPES_HPP__

#include <stdint.h>
#include "mem_config.hpp"
// One memory-ops record as the counters (mem_counter.cpp, mem_align_counter.cpp) consume it:
// the byte address and the header's bits 32-61 (mops_format.hpp). The counters switch on the low
// nibble of `flags` (the width or the block kind), test the write flag, and read a block's word
// count as `flags >> 4`; they never look at the step bits above.
struct MemCountersBusData {
    uint32_t addr;
    uint32_t flags;
} __attribute__((packed));

// The record whose header word is `w[0]`, for the counters. The payload words are not read: a
// count needs no value. The one rewrite is the heavy form's block write (MOPS_BLOCK_VALUES, 0x07),
// whose header holds the count in 6 bits at 36 and the write step above it: it is reported as an
// ALIGNED_BLOCK_WRITE of that count, the form the counters' block case already handles, so no
// counter needs a case for 0x07 and none reads the step bits as part of the count.
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

// One chunk of the stream as handed in by the runner: `count` 8-byte words starting at `data`
// (the pointer type is historical; the words are walked with mops_record_len and decoded with
// mops_decode_record, never indexed as records).
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