#ifndef __MEM_CONFIG_HPP__
#define __MEM_CONFIG_HPP__

#define ROM_ADDR 0x80000000
#define INPUT_ADDR 0x90000000
#define RAM_ADDR 0xA0000000

#define CHUNK_SIZE_BITS 18
#define CHUNK_SIZE (1 << CHUNK_SIZE_BITS)
#define MAX_LOCATORS 2048
#define MAX_MEM_PLANNERS 8
#define USE_ADDR_COUNT_TABLE
#define MAX_SEGMENTS 512
// #define MEM_PLANNER_STATS

#define MEM_CHECK_POINT_MAP
// #define SEGMENT_STATS
// #define CHUNK_STATS
// #define COUNT_CHUNK_STATS
#define SEGMENT_LARGE_CHUNKS 512

#define MEM_TYPES 3
#define ROM_ID 0
#define INPUT_ID 1
#define RAM_ID 2

#define RAM_ROWS (1 << 22)
#define ROM_ROWS (1 << 21)
#define INPUT_ROWS (1 << 21)
#define MEM_ROWS (1 << 22)
#define MAX_CHUNKS (1 << 18)     // 2^36 / 2^18 = 2^18

// THREAD_BITS >= 1
#define THREAD_BITS 2
#define ADDR_LOW_BITS (THREAD_BITS + 3)
#define MAX_THREADS (1 << THREAD_BITS)
#define ADDR_MASK ((MAX_THREADS - 1) * 8)

#define MAX_PAGES 12
#define ADDR_PAGE_BITS (23 - THREAD_BITS)
#define ADDR_PAGE_SIZE (1 << ADDR_PAGE_BITS)
#define RELATIVE_OFFSET_MASK (ADDR_PAGE_SIZE - 1)
#define ADDR_TABLE_SIZE (ADDR_PAGE_SIZE * MAX_PAGES)
#define OFFSET_BITS (25 + 4 - THREAD_BITS) // 4 bits (3 bits for 6 pages, 1 bit security)
#define OFFSET_PAGE_SHIFT_BITS (OFFSET_BITS - 3)

#define ADDR_SLOT_BITS 5
#define ADDR_SLOT_SIZE (1 << ADDR_SLOT_BITS)
#define ADDR_SLOT_MASK (0xFFFFFFFF << ADDR_SLOT_BITS)
#define ADDR_SLOTS ((1024 * 1024 * 64) / MAX_THREADS)

#define ADDR_SLOTS_SIZE (ADDR_SLOT_SIZE * ADDR_SLOTS)
#define TIME_US_BY_CHUNK 173

#define NO_CHUNK_ID 0xFFFFFFFF
#define EMPTY_PAGE 0xFFFFFFFF


// SINGLE WRITE FLAGS
//                bits
// bytes(4)        0-3   (values 1,2,4,8)
// write_flag (1)    4
// clear_flag (1)    8

// ALIGNED WRITE BLOCKS FLAGS
//                bits
// bytes(4)        0-3   (14 read block/15 write block)
// word_count(28) 4-31   2^28 * 2^3 = 2^31 bytes = 2GB MAX_MEMCPY_SIZE


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

#define MOPS_BLOCK_READ 0x0A
#define MOPS_BLOCK_WRITE 0x0B
#define MOPS_ALIGNED_READ 0x0C
#define MOPS_ALIGNED_WRITE 0x0D
#define MOPS_ALIGNED_BLOCK_READ 0x0E
#define MOPS_ALIGNED_BLOCK_WRITE 0x0F

#define MOPS_BLOCK_COUNT_SBITS      4

#endif
