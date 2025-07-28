#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C"
{
#endif

    // 👇 This makes `MemCountAndPlan*` a valid pointer type for C ABI consumers
    typedef struct MemCountAndPlan MemCountAndPlan;
    typedef struct MemCountersBusData MemCountersBusData;
    typedef struct MemCheckPoint MemCheckPoint;
    typedef struct MemAlignCheckPoint MemAlignCheckPoint;

    // C-compatible API (opaque to Rust)
    MemCountAndPlan *create_mem_count_and_plan(void);
    void destroy_mem_count_and_plan(MemCountAndPlan *mcp);
    void execute_mem_count_and_plan(MemCountAndPlan *mcp);
    void save_chunk(uint32_t chunk_id, MemCountersBusData *chunk_data, uint32_t chunk_size);
    void add_chunk_mem_count_and_plan(MemCountAndPlan *mcp, MemCountersBusData *chunk_data, uint32_t chunk_size);
    void stats_mem_count_and_plan(MemCountAndPlan *mcp);
    void set_completed_mem_count_and_plan(MemCountAndPlan *mcp);
    void wait_mem_count_and_plan(MemCountAndPlan *mcp);

    uint32_t get_mem_segment_count(MemCountAndPlan *mcp, uint32_t mem_id);
    const MemCheckPoint *get_mem_segment_check_points(MemCountAndPlan *mcp, uint32_t mem_id, uint32_t segment_id, uint32_t &count);
    const MemAlignCheckPoint *get_mem_align_check_points(MemCountAndPlan *mcp, uint32_t &count);

    // Additional functions for memory statistics
    uint64_t get_mem_stats_len(MemCountAndPlan * mcp);
    uint64_t get_mem_stats_ptr(MemCountAndPlan * mcp);

#ifdef __cplusplus
}
#endif
