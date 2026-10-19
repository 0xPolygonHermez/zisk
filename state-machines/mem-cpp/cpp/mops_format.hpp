#ifndef __MOPS_FORMAT_HPP__
#define __MOPS_FORMAT_HPP__

#include <stdint.h>

// The memory-ops stream: the memory accesses of a chunk as 8-byte words, written by the memory-ops
// emulator and read by the planner (CPU and GPU). Every record starts with a header word; the
// records of some kinds are followed by payload words. Bit 63 tells them apart: set on a header,
// clear on a payload.
//
// Header word:
//   bits 0-31   byte address
//   bits 32-37  mode, the 6-bit value the MOPS_* constants name: bits 32-35 are a nibble, the
//               width of a single access (1, 2, 4, 8) or a block kind (0x7, 0xA-0xF); bit 36
//               (0x10 in the mode) is the write flag, so WRITE_n = 0x10 | n; bit 37 (0x20) marks
//               the two special single accesses, CWRITE_1 (0x31, a byte store whose upper bits
//               were zero) and READ_8_VALUE (0x28, the read that carries its value)
//   bits 36-61  block kinds only: number of 8-byte words of the block (the count overlaps the top
//               mode bits, which block kinds leave at 0)
//   bits 38-57  single-access kinds only: the step field `(step_in_chunk << 2) | slot`
//   bit 58      MOPS_NO_VALUE_BIT: light form, a write or a READ_8_VALUE without its value
//   bit 62      MOPS_NO_PAYLOAD_BIT: light form, a block record without its payload;
//               heavy form, on a write: bit 63 of the value (the payload word keeps bit 63 clear)
//   bit 63      header tag
// step_in_chunk is the instruction's main step relative to the chunk the record sits in, 0 to
// chunk_size - 1 (the consumer adds chunk * chunk_size); slot orders the accesses of one
// instruction: 0 the a operand, 1 the b operand, 2 the store and the precompile reads, 3 the
// precompile writes. The memory step of the airs is 1 + main_step * 4 + slot.
//
// Light form (the emulator built with MOPS_LIGHT): one header word per record, no values and no
// steps. It carries what the planner needs to count and plan, nothing more.
//   read (READ_n, ALIGNED_READ)            header
//   write (WRITE_n, CWRITE_1, ALIGNED_WRITE) header | NO_VALUE_BIT
//   READ_8_VALUE                           header | NO_VALUE_BIT
//   block read, block write                header | NO_PAYLOAD_BIT (count at bit 36)
//
// Heavy form: the same records with the step field, and the value of every write, so that the
// device can build the memory rows from the stream alone.
//   read                                   header
//   write                                  header, value          (bit 63 of the value in header bit 62)
//   READ_8_VALUE                           header, value          (the free-input word's read, see below)
//   block read                             header, step field     (the payload word holds the step
//                                                                  field at bits 38-57, tag clear)
//   block write = BLOCK_VALUES (0x07)      header, mask, value[0..n)
//        header: aligned address | 0x07 at 32 | n at bits 36-41 (n <= 63) | step field at bits
//        42-61 (slot 3); mask: bit i = bit 63 of value[i]; the values with bit 63 cleared. A block
//        write wider than 63 words is several BLOCK_VALUES records. The heavy form never emits
//        BLOCK_WRITE or ALIGNED_BLOCK_WRITE.
//
// Both forms decode with mops_record_len; the planner counts both. The GPU planner sees a light
// record (mops_record_is_light) as "no values in this stream": the plan stands, the block gets no
// device witness.
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
