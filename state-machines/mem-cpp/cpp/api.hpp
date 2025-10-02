#pragma once

#include <stdint.h>

// To regenerate the bindings, run the following command on state-machines/mem-cpp:
// bindgen cpp/api.hpp -o src/bindings.rs

#ifdef __cplusplus
extern "C"
{
#endif

    // 👇 This makes `MemCountAndPlan*` a valid pointer type for C ABI consumers
    typedef struct MemCountAndPlan MemCountAndPlan;
    typedef struct MemCountersBusData MemCountersBusData;
    typedef struct MemCheckPoint MemCheckPoint;
    typedef struct MemAlignChunkCounters MemAlignChunkCounters;

    // C-compatible API (opaque to Rust)
    MemCountAndPlan *create_mem_count_and_plan(void);
    void destroy_mem_count_and_plan(MemCountAndPlan *mcp);
    void execute_mem_count_and_plan(MemCountAndPlan *mcp);
    void save_chunk(uint32_t chunk_id, MemCountersBusData *chunk_data, uint32_t chunk_size);
    void add_chunk_mem_count_and_plan(MemCountAndPlan *mcp, MemCountersBusData *chunk_data, uint32_t chunk_size);
    void stats_mem_count_and_plan(MemCountAndPlan *mcp);
    void set_completed_mem_count_and_plan(MemCountAndPlan *mcp);
    void wait_mem_count_and_plan(MemCountAndPlan *mcp);
    void wait_mem_align_counters(MemCountAndPlan *mcp);

    uint32_t get_mem_segment_count(MemCountAndPlan *mcp, uint32_t mem_id);
    const MemCheckPoint *get_mem_segment_check_points(MemCountAndPlan *mcp, uint32_t mem_id, uint32_t segment_id, uint32_t &count);
    const MemAlignChunkCounters *get_mem_align_counters(MemCountAndPlan *mcp, uint32_t &count);
    const MemAlignChunkCounters *get_mem_align_total_counters(MemCountAndPlan *mcp);

    // Additional functions for memory statistics
    uint64_t get_mem_stats_len(MemCountAndPlan * mcp);
    uint64_t get_mem_stats_ptr(MemCountAndPlan * mcp);

#ifdef __cplusplus
}
#endif
