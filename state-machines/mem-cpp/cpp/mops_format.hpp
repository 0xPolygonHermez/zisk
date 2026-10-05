#ifndef __MOPS_FORMAT_HPP__
#define __MOPS_FORMAT_HPP__

#include <stdint.h>

// Memory-ops stream: tagged 8-byte words, bit 63 set on header words and clear on payload words.
// Header: address (bits 0-31), mode (32-37), step field `(step_in_chunk << 2) | slot` (38-57).
// Two forms, chosen when the emulator is generated:
//  - heavy: a read is the header alone; a write adds the value with its bit 63 moved to header
//    bit 62; a block read (word count from bit 36) adds the step field as payload; a block write
//    is a value block: aligned address | 0x07 | words (6 bits at 36) | write step field (20 bits
//    at 42), then a word of the values' bits 63 and the values with bit 63 cleared.
//  - light: every record is one word, no values and no steps. A write sets header bit
//    MOPS_NO_VALUE_BIT; a block read or block write sets bit MOPS_NO_PAYLOAD_BIT.
// The planner counts both; a block in the light form gets no device witness.

#define MOPS_WRITE_FLAG 0x10
#define MOPS_WRITE_BYTE_CLEAR_FLAG 0x20

#define MOPS_READ_8   0x08
#define MOPS_READ_4   0x04
#define MOPS_READ_2   0x02
#define MOPS_READ_1   0x01

#define MOPS_WRITE_8  0x18
#define MOPS_WRITE_4  0x14
#define MOPS_WRITE_2  0x12
#define MOPS_WRITE_1  0x11

#define MOPS_CWRITE_1 0x31
// An aligned 8-byte read that carries the value read: the free-input word, whose reads return
// successive fcall results that no image holds.
#define MOPS_READ_8_VALUE 0x28

#define MOPS_BLOCK_READ 0x0A
#define MOPS_BLOCK_WRITE 0x0B
#define MOPS_ALIGNED_READ 0x0C
#define MOPS_ALIGNED_WRITE 0x0D
#define MOPS_ALIGNED_BLOCK_READ 0x0E
#define MOPS_ALIGNED_BLOCK_WRITE 0x0F
#define MOPS_BLOCK_VALUES 0x07
#define MOPS_VALUES_STEP_SHIFT 42
#define MOPS_BLOCK_COUNT_SBITS 4
#define MOPS_NO_VALUE_BIT 58
#define MOPS_NO_PAYLOAD_BIT 62

#ifdef __CUDACC__
#define MOPS_HD __host__ __device__ __forceinline__
#else
#define MOPS_HD static inline
#endif

// Words of the record whose header word is `hdr`.
MOPS_HD uint32_t mops_record_len(uint64_t hdr) {
    const uint32_t mode = (uint32_t)(hdr >> 32) & 0x3F;
    const uint32_t low  = mode & 0x0F;
    const int single = (low == 1 || low == 2 || low == 4 || low == 8);
    if (low == MOPS_ALIGNED_READ) return 1;
    if (mode == MOPS_READ_8_VALUE) return ((hdr >> MOPS_NO_VALUE_BIT) & 1) ? 1 : 2;
    if (single && (mode & MOPS_WRITE_FLAG) == 0) return 1;
    if (single || low == MOPS_ALIGNED_WRITE) return ((hdr >> MOPS_NO_VALUE_BIT) & 1) ? 1 : 2;
    if (low == MOPS_BLOCK_VALUES) return 2 + (uint32_t)((hdr >> 36) & 63);
    return ((hdr >> MOPS_NO_PAYLOAD_BIT) & 1) ? 1 : 2;   // block read or block write
}
// A record of the light form: a write without its value or a block record without its payload.
MOPS_HD int mops_record_is_light(uint64_t hdr) {
    const uint32_t low = (uint32_t)(hdr >> 32) & 0x0F;
    if (((uint32_t)(hdr >> 32) & 0x3F) == MOPS_READ_8_VALUE) return (int)((hdr >> MOPS_NO_VALUE_BIT) & 1);
    if (low == MOPS_BLOCK_VALUES || low == MOPS_ALIGNED_READ) return 0;
    if (low == 1 || low == 2 || low == 4 || low == 8 || low == MOPS_ALIGNED_WRITE) return (int)((hdr >> MOPS_NO_VALUE_BIT) & 1);
    return (int)((hdr >> MOPS_NO_PAYLOAD_BIT) & 1);
}

#endif
