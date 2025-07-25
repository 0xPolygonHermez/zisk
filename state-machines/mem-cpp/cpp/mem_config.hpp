#ifndef __MEM_CONFIG_HPP__
#define __MEM_CONFIG_HPP__

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
#define MEM_ALIGN_ROWS (1 << 22)
#define MAX_CHUNKS 8192     // 2^13 * 2^18 = 2^31

#define THREAD_BITS 2
#define ADDR_LOW_BITS (THREAD_BITS + 3)
#define MAX_THREADS (1 << THREAD_BITS)
#define ADDR_MASK ((MAX_THREADS - 1) * 8)

#define MAX_PAGES 20
#define ADDR_PAGE_BITS (23 - THREAD_BITS)
#define ADDR_PAGE_SIZE (1 << ADDR_PAGE_BITS)
#define RELATIVE_OFFSET_MASK (ADDR_PAGE_SIZE - 1)
#define ADDR_TABLE_SIZE (ADDR_PAGE_SIZE * MAX_PAGES)
#define OFFSET_BITS (25 + 4 - THREAD_BITS) // 4 bits (3 bits for 6 pages, 1 bit security)
#define OFFSET_PAGE_SHIFT_BITS (OFFSET_BITS - 3)

#define ADDR_SLOT_BITS 5
#define ADDR_SLOT_SIZE (1 << ADDR_SLOT_BITS)
#define ADDR_SLOT_MASK (0xFFFFFFFF << ADDR_SLOT_BITS)
#define ADDR_SLOTS ((1024 * 1024 * 32) / MAX_THREADS)

#define ADDR_SLOTS_SIZE (ADDR_SLOT_SIZE * ADDR_SLOTS)
#define TIME_US_BY_CHUNK 173

#define NO_CHUNK_ID 0xFFFFFFFF
#define EMPTY_PAGE 0xFFFFFFFF

#endif
