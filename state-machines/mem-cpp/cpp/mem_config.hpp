#ifndef __MEM_CONFIG_HPP__
#define __MEM_CONFIG_HPP__

// Must match core/src/mem.rs
#define ROM_ADDR 0x90000000
#define INPUT_ADDR 0x10000000
#define RAM_ADDR 0xA0000000

#define ROM_SIZE_MB 128
#define INPUT_SIZE_MB 2048
#define RAM_SIZE_MB 512

#define CHUNK_SIZE_BITS 18
#define CHUNK_SIZE (1 << CHUNK_SIZE_BITS)
#define MAX_LOCATORS 2048
#define MAX_MEM_PLANNERS 8
#define USE_ADDR_COUNT_TABLE
#define MAX_SEGMENTS 512
// #define MEM_PLANNER_STATS
// #define MEM_SAVE_CPP_OFFSETS

#define MEM_CHECK_POINT_MAP
// #define SEGMENT_STATS
// #define CHUNK_STATS
// #define COUNT_CHUNK_STATS
#define SEGMENT_LARGE_CHUNKS 512

#define MEM_TYPES 3
#define ROM_ID 0
#define INPUT_ID 1
#define RAM_ID 2



#define MAX_CHUNKS (1 << 18)     // 2^36 / 2^18 = 2^18

// THREAD_BITS >= 1
#define THREAD_BITS 2
#define ADDR_LOW_BITS (THREAD_BITS + 3)
#define MAX_THREADS (1 << THREAD_BITS)
#define ADDR_MASK ((MAX_THREADS - 1) * 8)

// Each page covers 64MB (2^ADDR_BLOCK_BITS bytes) of address space, the pages of each memory area
// are consecutive: ROM pages first, then INPUT pages and then RAM pages.
#define ADDR_BLOCK_BITS 26
#define ADDR_BLOCK_MB (1 << (ADDR_BLOCK_BITS - 20))
#define ROM_PAGES (ROM_SIZE_MB / ADDR_BLOCK_MB)
#define INPUT_PAGES (INPUT_SIZE_MB / ADDR_BLOCK_MB)
#define RAM_PAGES (RAM_SIZE_MB / ADDR_BLOCK_MB)
#define ROM_FIRST_PAGE 0
#define INPUT_FIRST_PAGE (ROM_FIRST_PAGE + ROM_PAGES)
#define RAM_FIRST_PAGE (INPUT_FIRST_PAGE + INPUT_PAGES)
#define MAX_PAGES (RAM_FIRST_PAGE + RAM_PAGES)
#define ADDR_PAGE_BITS (ADDR_BLOCK_BITS - ADDR_LOW_BITS)
#define ADDR_PAGE_SIZE (1 << ADDR_PAGE_BITS)
#define RELATIVE_OFFSET_MASK (ADDR_PAGE_SIZE - 1)
#define ADDR_TABLE_SIZE (ADDR_PAGE_SIZE * MAX_PAGES)
#define OFFSET_BITS (ADDR_PAGE_BITS + 6) // 6 bits for up to 64 pages
#define OFFSET_PAGE_SHIFT_BITS (OFFSET_BITS - 3)

#if (ROM_ADDR % (1 << ADDR_BLOCK_BITS)) || (INPUT_ADDR % (1 << ADDR_BLOCK_BITS)) || (RAM_ADDR % (1 << ADDR_BLOCK_BITS))
#error "memory areas must start at a page boundary"
#endif
#if (ROM_SIZE_MB % ADDR_BLOCK_MB) || (INPUT_SIZE_MB % ADDR_BLOCK_MB) || (RAM_SIZE_MB % ADDR_BLOCK_MB)
#error "memory area sizes must be a multiple of the page size"
#endif
#if MAX_PAGES > (1 << (OFFSET_BITS - ADDR_PAGE_BITS))
#error "too many pages for OFFSET_BITS"
#endif

#define MAX_SLOT_GB 6
#define ADDR_SLOT_BITS 5
#define ADDR_SLOT_BYTES 4
#define ADDR_SLOT_SIZE (1 << ADDR_SLOT_BITS)
#define ADDR_SLOT_MASK (0xFFFFFFFF << ADDR_SLOT_BITS)
#define ADDR_TOTAL_SLOTS ((((size_t)MAX_SLOT_GB) << 30) / (ADDR_SLOT_SIZE * ADDR_SLOT_BYTES))
#define ADDR_SLOTS (ADDR_TOTAL_SLOTS / MAX_THREADS)

#define ADDR_SLOTS_SIZE (ADDR_SLOT_SIZE * ADDR_SLOTS)
#define TIME_US_BY_CHUNK 173

#define NO_CHUNK_ID 0xFFFFFFFF
#define EMPTY_PAGE 0xFFFFFFFF

#define OFFSET_PAGE_SIZE 1024
#define MAX_OFFSETS (1 << 22)

#define MAX_OFFSET_PAGES_INCREMENT 8192
#define MAX_OFFSET_INCREMENT (1024 * OFFSET_PAGE_SIZE)
#define MEM_OFFSETS_PAGES 4096
#define MEM_OFFSETS MAX_OFFSET_INCREMENT

#define MAX_IMMUTABLE_ADDR_DISTANCE (1 << 24)

// SINGLE WRITE FLAGS
//                bits
// bytes(4)        0-3   (values 1,2,4,8)
// write_flag (1)    4
// clear_flag (1)    5

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
