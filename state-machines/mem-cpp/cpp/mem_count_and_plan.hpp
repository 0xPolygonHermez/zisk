#ifndef __MEM_COUNT_AND_PLAN_HPP__
#define __MEM_COUNT_AND_PLAN_HPP__

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/stat.h>
#include <vector>
#include <thread>
#include <iostream>
#include <string.h>
#include <sys/time.h>
#include <cstdint>
#include <vector>
#include <map>
#include <unordered_map>
#include <stdexcept>
#include <mutex>
#include <atomic>
#include <memory>

#include "mem_types.hpp"
#include "mem_config.hpp"
#include "tools.hpp"
#include "mem_counter.hpp"
#include "mem_align_counter.hpp"
#include "mem_planner.hpp"
#include "mem_locator.hpp"
#include "mem_context.hpp"
#include "immutable_mem_planner.hpp"
#include "mem_segments.hpp"

typedef struct {
    int thread_index;
    const MemCountTrace *mcp;
    int count;
} MemCountAndPlanThread;

class MemCountAndPlan {
private:
    uint32_t max_chunks;
    std::vector<std::thread> plan_threads;
    std::vector<MemCounter *> count_workers;
    std::shared_ptr<MemContext> context;
    std::unique_ptr<MemPlanner> quick_mem_planner;
    std::unique_ptr<ImmutableMemPlanner> rom_data_planner;
    std::unique_ptr<ImmutableMemPlanner> input_data_planner;
    std::vector<MemPlanner> plan_workers;
    std::unique_ptr<std::thread> parallel_execute;
    uint64_t t_init_us;
    uint64_t t_count_us;
    uint64_t t_prepare_us;
    uint64_t t_plan_us;

#ifdef MEM_STATS_ACTIVE
public:
    MemStats *mem_stats;
#endif // MEM_STATS_ACTIVE

public:
    MemSegments segments[MEM_TYPES];
    std::unique_ptr<MemAlignCounter> mem_align_counter;

    MemCountAndPlan();
    ~MemCountAndPlan();
    void clear();
    void prepare();
    void add_chunk(MemCountersBusData *chunk_data, uint32_t chunk_size);
    void detach_execute();
    void execute(void);
    void count_phase();
    void plan_phase();
    void stats();
    void wait(); 

    void set_completed() {
        context->set_completed();
    }
    
};
/*
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
*/

#endif
