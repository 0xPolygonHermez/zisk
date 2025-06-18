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
    MemContext *context;
    MemPlanner *quick_mem_planner;
    ImmutableMemPlanner *rom_data_planner;
    ImmutableMemPlanner *input_data_planner;
    std::vector<MemPlanner> plan_workers;
    std::thread *parallel_execute;
    uint64_t t_init_us;
    uint64_t t_count_us;
    uint64_t t_prepare_us;
    uint64_t t_plan_us;

public:
    MemSegments segments[MEM_TYPES];
    MemAlignCounter *mem_align_counter;

    MemCountAndPlan() {
        context = new MemContext();
    }
    ~MemCountAndPlan() {
    }
    void clear() {
        // for (auto& chunk : chunks) {
        //     free(chunk.chunk_data);
        // }
        context->clear();
    }
    void prepare() {
        uint init = get_usec();
        printf("Preparing MemCountAndPlan (clear count_workers)...\n");
        count_workers.clear();
        printf("Preparing MemCountAndPlan (count_workers)...\n");
        for (size_t i = 0; i < MAX_THREADS; ++i) {
            printf("Preparing MemCountAndPlan (count_worker %ld)...\n", i);
            count_workers.push_back(new MemCounter(i, context));
        }
        printf("Preparing MemCountAndPlan (mem_align_counter)...\n");
        mem_align_counter = new MemAlignCounter(MEM_ALIGN_ROWS, context);
        plan_workers.clear();
        printf("Preparing MemCountAndPlan (rom_data_planner)...\n");
        rom_data_planner = new ImmutableMemPlanner(ROM_ROWS, 0x80000000, 128);
        printf("Preparing MemCountAndPlan (input_data_planner)...\n");
        input_data_planner = new ImmutableMemPlanner(INPUT_ROWS, 0x90000000, 128);
        printf("Preparing MemCountAndPlan (quick_mem_planner)...\n");
        quick_mem_planner = new MemPlanner(0, RAM_ROWS, 0xA0000000, 512);
        printf("Preparing MemCountAndPlan (planners)...\n");
        for (int i = 0; i < MAX_MEM_PLANNERS; ++i) {
            plan_workers.emplace_back(i+1, RAM_ROWS, 0xA0000000, 512);
        }
        printf("Prepared MemCountAndPlan\n");
        t_prepare_us = get_usec() - init;
    }
    void add_chunk(MemCountersBusData *chunk_data, uint32_t chunk_size) {
        context->add_chunk(chunk_data, chunk_size);
    }
    void detach_execute() {
        printf("MemCountAndPlan::count_phase\n");
        count_phase();
        printf("MemCountAndPlan::plan_phase\n");
        plan_phase();
    }
    void execute(void) {
        parallel_execute = new std::thread([this](){ this->detach_execute();});
        // parallel_execute.detach();
    }
    void count_phase() {
        uint64_t init = t_init_us = get_usec();
        std::vector<std::thread> threads;

        for (int i = 0; i < MAX_THREADS; ++i) {
            threads.emplace_back([this, i](){count_workers[i]->execute();});
        }
        threads.emplace_back([this](){ mem_align_counter->execute();});

        for (auto& t : threads) {
            t.join();
        }
        t_count_us = (uint32_t) (get_usec() - init);
    }

    void plan_phase() {
        uint64_t init = get_usec();
        std::vector<std::thread> threads;

    
        plan_threads.emplace_back([this](){ quick_mem_planner->generate_locators(count_workers, context->locators);});
        plan_threads.emplace_back([this](){ rom_data_planner->execute(count_workers);});
        plan_threads.emplace_back([this](){ input_data_planner->execute(count_workers);});
        segments[RAM_ID].clear();
        for (int i = 0; i < MAX_MEM_PLANNERS; ++i) {
            threads.emplace_back([this, i](){ plan_workers[i].execute_from_locators(count_workers, context->locators, segments[RAM_ID]);});
        }
        for (auto& t : threads) {
            t.join();
        }
        for (auto& t : plan_threads) {
            t.join();
        }
        t_plan_us = (uint32_t) (get_usec() - init);

        segments[ROM_ID].clear();
        rom_data_planner->collect_segments(segments[ROM_ID]);

        segments[INPUT_ID].clear();
        input_data_planner->collect_segments(segments[INPUT_ID]);
        segments[ROM_ID].debug();
        segments[INPUT_ID].debug();
        segments[RAM_ID].debug();
        mem_align_counter->debug();
    }

    void stats() {
        uint32_t tot_used_slots = 0;
        for (size_t i = 0; i < MAX_THREADS; ++i) {
            uint32_t used_slots = count_workers[i]->get_used_slots();
            tot_used_slots += used_slots;
            printf("Thread %ld: used slots %d/%d (%04.02f%%) T:%d ms S:%d ms Q:%d\n",
                i, used_slots, ADDR_SLOTS,
                ((double)used_slots*100.0)/(double)(ADDR_SLOTS), count_workers[i]->get_elapsed_ms(),
                count_workers[i]->get_tot_usleep()/1000,
                count_workers[i]->get_queue_full_times()/1000);
        }
        printf("\n> threads: %d\n", MAX_THREADS);
        printf("> address table: %ld MB\n", (ADDR_TABLE_SIZE * ADDR_TABLE_ELEMENT_SIZE * MAX_THREADS)>>20);
        printf("> memory slots: %ld MB (used: %ld MB)\n", (ADDR_SLOTS_SIZE * sizeof(uint32_t) * MAX_THREADS)>>20, (tot_used_slots * ADDR_SLOT_SIZE * sizeof(uint32_t))>> 20);
        printf("> page table: %ld MB\n\n", (ADDR_PAGE_SIZE * sizeof(uint32_t))>> 20);
        quick_mem_planner->stats();
        for (uint32_t i = 0; i < plan_workers.size(); ++i) {
            plan_workers[i].stats();
        }
        printf("execution: %04.2f ms\n", (TIME_US_BY_CHUNK * context->size()) / 1000.0);
        printf("count_phase: %04.2f ms\n", t_count_us / 1000.0);
        printf("plan_phase: %04.2f ms\n", t_plan_us / 1000.0);
    }
    void set_completed() {
        context->set_completed();
    }
    void wait() {
        parallel_execute->join();
        delete parallel_execute;
        parallel_execute = nullptr;
    }

};

MemCountAndPlan *create_mem_count_and_plan(void) {
    MemCountAndPlan *mcp = new MemCountAndPlan();
    printf("MemCountAndPlan created. Preparing ....\n");
    mcp->prepare();
    printf("MemCountAndPlan prepared\n");
    return mcp;
}

void destroy_mem_count_and_plan(MemCountAndPlan *mcp)
{
    if (mcp)
    {
        mcp->clear();
        delete mcp;
    }
}

void execute_mem_count_and_plan(MemCountAndPlan *mcp)
{
    mcp->execute();
}

void add_chunk_mem_count_and_plan(MemCountAndPlan *mcp, MemCountersBusData *chunk_data, uint32_t chunk_size)
{
    mcp->add_chunk(chunk_data, chunk_size);
}

void stats_mem_count_and_plan(MemCountAndPlan *mcp)
{
    mcp->stats();
}

void set_completed_mem_count_and_plan(MemCountAndPlan *mcp)
{
    mcp->set_completed();
}

void wait_mem_count_and_plan(MemCountAndPlan *mcp)
{
    mcp->wait();
}

uint32_t get_segment_count(MemCountAndPlan *mcp, uint32_t mem_id)
{
    return mcp->segments[mem_id].size();
}

const MemCheckPoint *get_segment_check_point(MemCountAndPlan *mcp, uint32_t mem_id, uint32_t segment_id, uint32_t &count)
{
    auto segment = mcp->segments[mem_id].get(segment_id);
    count = segment ? segment->size() : 0;
    return segment->get_chunks();
}

const MemAlignCheckPoint *get_mem_align_check_point(MemCountAndPlan *mcp, uint32_t &count)
{
    count = mcp->mem_align_counter->size();
    if (count == 0) {
        return nullptr;
    }
    return mcp->mem_align_counter->get_checkpoints();
}

#endif