#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C"
{
#endif

    // 👇 This makes `MemCountAndPlan*` a valid pointer type for C ABI consumers
    typedef struct MemCountAndPlan MemCountAndPlan;
    typedef struct MemCountersBusData MemCountersBusData;

    // C-compatible API (opaque to Rust)
    MemCountAndPlan *create_mem_count_and_plan(void);
    void destroy_mem_count_and_plan(MemCountAndPlan *mcp);
    void execute_mem_count_and_plan(MemCountAndPlan *mcp);
    void add_chunk_mem_count_and_plan(MemCountAndPlan *mcp, MemCountersBusData *chunk_data, uint32_t chunk_size);
    void stats_mem_count_and_plan(MemCountAndPlan *mcp);
    void set_completed_mem_count_and_plan(MemCountAndPlan *mcp);
    void wait_mem_count_and_plan(MemCountAndPlan *mcp);

#ifdef __cplusplus
}
#endif
