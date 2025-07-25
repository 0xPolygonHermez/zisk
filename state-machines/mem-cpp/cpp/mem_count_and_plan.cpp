#include <memory>
#include "api.hpp"
#include "tools.hpp"
#include "mem_count_and_plan.hpp"

MemCountAndPlan::MemCountAndPlan() {
    context = std::make_shared<MemContext>();
}

MemCountAndPlan::~MemCountAndPlan() {

    // Call clear
    clear();
}

void MemCountAndPlan::clear() {
    // Wait for and clean up any background threads
    if (parallel_execute && parallel_execute->joinable()) {
        parallel_execute->join();
    }

    // Clean up count_workers raw pointers
    for (auto* worker : count_workers) {
        delete worker;
    }
    count_workers.clear();
    
    // Clean up plan_workers
    plan_threads.clear();
    
    // Clear segments (they have their own cleanup)
    for (int i = 0; i < MEM_TYPES; ++i) {
        segments[i].clear();
    }
    
    context->clear();
}
void MemCountAndPlan::prepare() {
    uint64_t init = get_usec();
    
    // Clear existing workers to avoid memory leaks if prepare() called multiple times
    for (auto* worker : count_workers) {
        delete worker;
    }
    count_workers.clear();
    
    for (size_t i = 0; i < MAX_THREADS; ++i) {
        count_workers.push_back(new MemCounter(i, context));
    }
    mem_align_counter = std::make_unique<MemAlignCounter>(MEM_ALIGN_ROWS, context);
    plan_workers.clear();
    plan_workers.reserve(MAX_MEM_PLANNERS);
    rom_data_planner = std::make_unique<ImmutableMemPlanner>(ROM_ROWS, 0x80000000, 128);
    input_data_planner = std::make_unique<ImmutableMemPlanner>(INPUT_ROWS, 0x90000000, 128);
    quick_mem_planner = std::make_unique<MemPlanner>(0, RAM_ROWS, 0xA0000000, 512);
    for (int i = 0; i < MAX_MEM_PLANNERS; ++i) {
        plan_workers.emplace_back(i+1, RAM_ROWS, 0xA0000000, 512);
    }
    t_prepare_us = get_usec() - init;
}

void MemCountAndPlan::add_chunk(MemCountersBusData *chunk_data, uint32_t chunk_size) {
    context->add_chunk(chunk_data, chunk_size);
}

void MemCountAndPlan::execute(void) {
    parallel_execute = std::make_unique<std::thread>(&MemCountAndPlan::detach_execute, this);
}

void MemCountAndPlan::count_phase() {
    uint64_t init = t_init_us = get_usec();
    std::vector<std::thread> threads;
    context->init();

    for (int i = 0; i < MAX_THREADS; ++i) {
        threads.emplace_back([this, i](){count_workers[i]->execute();});
    }
    threads.emplace_back([this](){ mem_align_counter->execute();});

    for (auto& t : threads) {
        t.join();
    }
    uint64_t max_tot_wait_us = 0;
    uint64_t tot_wait_us = 0;
    uint32_t max_used_slots = 0;
    for (uint32_t index = 0; index < count_workers.size(); ++index) {
        if (count_workers[index]->tot_wait_us > max_tot_wait_us) {
            max_tot_wait_us = count_workers[index]->tot_wait_us;
        }
        tot_wait_us += count_workers[index]->tot_wait_us;        
        if (count_workers[index]->get_used_slots() > max_used_slots) {
            max_used_slots = count_workers[index]->get_used_slots();
        }
    }
    // printf("MemCountAndPlan wait_avg(ms): %ld max_wait(ms): %ld ms threads: %d max_used_slots: %04.2f%%\n", 
    //         (tot_wait_us >> THREAD_BITS)/1000, 
    //         max_tot_wait_us/1000, 
    //         1 << THREAD_BITS,
    //         max_used_slots * 100.0 / ADDR_SLOTS);
    t_count_us = (uint32_t) (get_usec() - init);    
}

void MemCountAndPlan::plan_phase() {
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

    // printf("RAM segments: %ld\n", segments[RAM_ID].size());
    // segments[RAM_ID].debug();

    segments[ROM_ID].clear();
    rom_data_planner->collect_segments(segments[ROM_ID]);
    // printf("ROM segments: %ld\n", segments[ROM_ID].size());
    // segments[ROM_ID].debug();
    // printf("ROM segments END\n");

    segments[INPUT_ID].clear();
    input_data_planner->collect_segments(segments[INPUT_ID]);
    // printf("INPUT segments: %ld\n", segments[INPUT_ID].size());
    // segments[INPUT_ID].debug();
    
    // printf("MEM_ALIGN segments\n");
    // mem_align_counter->debug();
}

void MemCountAndPlan::stats() {
    uint32_t tot_used_slots = 0;
    for (size_t i = 0; i < MAX_THREADS; ++i) {
        uint32_t used_slots = count_workers[i]->get_used_slots();
        tot_used_slots += used_slots;
        printf("Thread %ld: used slots %d/%d (%04.02f%%) T(ms):%d S(ms):%ld C0(us):%ld Q:%d\n",
            i, used_slots, ADDR_SLOTS,
            ((double)used_slots*100.0)/(double)(ADDR_SLOTS), count_workers[i]->get_elapsed_ms(),
            count_workers[i]->tot_wait_us/1000,
            count_workers[i]->get_first_chunk_us(),
            count_workers[i]->get_queue_full_times()/1000);
    }
    #ifdef CHUNK_STATS
    context->stats();
    for (size_t i = 0; i < MAX_THREADS; ++i) {
        count_workers[i]->stats();
    }
    #endif
    printf("\n> threads: %d\n", MAX_THREADS);
    printf("> address table: %ld MB\n", (ADDR_TABLE_SIZE * ADDR_TABLE_ELEMENT_SIZE * MAX_THREADS)>>20);
    printf("> memory slots: %ld MB (used: %ld MB)\n", (ADDR_SLOTS_SIZE * sizeof(uint32_t) * MAX_THREADS)>>20, (tot_used_slots * ADDR_SLOT_SIZE * sizeof(uint32_t))>> 20);
    printf("> page table: %ld MB\n\n", (ADDR_PAGE_SIZE * sizeof(uint32_t))>> 20);
    quick_mem_planner->stats();
    for (uint32_t i = 0; i < plan_workers.size(); ++i) {
        plan_workers[i].stats();
    }
    printf("prepare: %04.2f ms\n", t_prepare_us / 1000.0);
    printf("execution: %04.2f ms\n", (TIME_US_BY_CHUNK * context->size()) / 1000.0);
    printf("completed: %04.2f ms\n", context->get_completed_us() / 1000.0);
    printf("count_phase: %04.2f ms\n", t_count_us / 1000.0);
    printf("plan_phase: %04.2f ms\n", t_plan_us / 1000.0);
}

MemCountAndPlan *create_mem_count_and_plan(void) {
    MemCountAndPlan *mcp = new MemCountAndPlan();
    mcp->prepare();
    return mcp;
}

void destroy_mem_count_and_plan(MemCountAndPlan *mcp) {
    if (mcp) {
        mcp->clear();
        delete mcp;
        mcp = nullptr;
    }
}

void execute_mem_count_and_plan(MemCountAndPlan *mcp)
{
    mcp->execute();
}

void save_chunk(uint32_t chunk_id, MemCountersBusData *chunk_data, uint32_t chunk_size)
{
    char filename[200];
    snprintf(filename, sizeof(filename), "tmp/bus_data_asm/mem_count_data_%d.bin", chunk_id);
    int fd = open(filename, O_WRONLY | O_CREAT | O_TRUNC, S_IRUSR | S_IWUSR);
    
    ssize_t bytes_written = write(fd, chunk_data, sizeof(MemCountersBusData) * chunk_size);
    if (bytes_written < 0) {
        perror("Error writing to file");
    } else if (static_cast<size_t>(bytes_written) != sizeof(MemCountersBusData) * chunk_size) {
        fprintf(stderr, "Partial write: expected %zu bytes, but wrote %zd bytes\n",
                sizeof(MemCountersBusData) * chunk_size, bytes_written);
    }
    
    close(fd);
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

uint32_t get_mem_segment_count(MemCountAndPlan *mcp, uint32_t mem_id)
{
    return mcp->segments[mem_id].size();
}

const MemCheckPoint *get_mem_segment_check_points(MemCountAndPlan *mcp, uint32_t mem_id, uint32_t segment_id, uint32_t &count)
{
    auto segment = mcp->segments[mem_id].get(segment_id);
    count = segment ? segment->size() : 0;
    return segment->get_chunks();
}

const MemAlignCheckPoint *get_mem_align_check_points(MemCountAndPlan *mcp, uint32_t &count)
{
    count = mcp->mem_align_counter->size();
    if (count == 0) {
        return nullptr;
    }
    return mcp->mem_align_counter->get_checkpoints();
}

void MemCountAndPlan::wait() {
    try {
        parallel_execute->join();
        // delete parallel_execute;
        // parallel_execute = nullptr;
    } catch (const std::exception &e) {
        printf("Exception in wait: %s\n", e.what());
    }
}

void MemCountAndPlan::detach_execute() {
    count_phase();
    plan_phase();
    //stats();
    // printf("MemCountAndPlan count(ms):%ld plan(ms):%ld tot(ms):%ld\n", 
    //        t_count_us / 1000, t_plan_us / 1000, (t_count_us + t_plan_us) / 1000);
}
