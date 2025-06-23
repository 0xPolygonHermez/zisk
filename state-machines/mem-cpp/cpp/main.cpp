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
#include "immutable_mem_planner.hpp"
#include "mem_count_and_plan.hpp"
#include "mem_test.hpp"

// TODO: shared memory slots to balance in a worst scenario
// TODO: incremental memory slots on worst scenario (consolidate full memory slots? to avoid increase).


class LockFreeRingBuffer {
    std::vector<uint32_t> buffer;
    std::atomic<size_t> read_pos{0};
    std::atomic<size_t> write_pos{0};
    size_t capacity;

public:
    LockFreeRingBuffer(size_t size) : buffer(size), capacity(size) {}

    bool try_push(uint32_t value) {
        size_t current_write = write_pos.load(std::memory_order_relaxed);
        size_t next_write = (current_write + 1) % capacity;

        if(next_write == read_pos.load(std::memory_order_acquire)) {
            return false; // full buffer
        }

        buffer[current_write] = value;
        write_pos.store(next_write, std::memory_order_release);
        return true;
    }

    bool try_pop(uint32_t& value) {
        size_t current_read = read_pos.load(std::memory_order_relaxed);

        if(current_read == write_pos.load(std::memory_order_acquire)) {
            return false; // empty buffer
        }

        value = buffer[current_read];
        read_pos.store((current_read + 1) % capacity, std::memory_order_release);
        return true;
    }
};

<<<<<<< HEAD
int main(int argc, const char *argv[]) {

    MemTest mem_test;
    mem_test.load(argc > 1 ? argv[1] : "../bus_data.org/mem_count_data");
    mem_test.execute();
}

=======
int main() {
    // printf("Starting...\n");
    // auto cp = create_mem_count_and_plan();
    // printf("Executing...\n");
    // execute(cp);
    // while (true) {
    //     printf("Waiting...\n");
    //     sleep(5);
    // }
    MemTest mem_test;
    mem_test.load();
    mem_test.execute();
    printf("END\n");
}

#ifdef false

struct MemAlignCount {
    uint32_t chunk_id;
    uint32_t count[3];
    MemAlignCount(uint32_t chunk_id, uint32_t count[3]) : chunk_id(chunk_id), count{count[0], count[1], count[2]} {}
};

typedef struct {
    int thread_index;
    const MemCountTrace *mcp;
    int count;
} MemCountAndPlanThread;


int main2() {
    printf("Starting...\n");

    MemCountTrace mcp;
    int chunks = 0;
    int tot_chunks = 0;
    uint32_t tot_ops = 0;
    printf("Loading compact data...\n");
    int32_t items_read;
    while (chunks < MAX_CHUNKS && (items_read = load_from_compact_file(chunks, &(mcp.chunk_data[chunks]))) >=0) {
        mcp.chunk_size[chunks] = items_read;
        tot_ops += count_operations(mcp.chunk_data[chunks], mcp.chunk_size[chunks]);
        chunks++;
        tot_chunks += mcp.chunk_size[chunks - 1];
        if (chunks % 100 == 0) printf("Loaded chunk %d with size %d\n", chunks, mcp.chunk_size[chunks - 1]);
    }
    printf("chunks: %d  tot_chunks: %d tot_ops: %d tot_time:%d (ms)\n", chunks, tot_chunks, tot_ops, (chunks * TIME_US_BY_CHUNK)/1000);
    mcp.chunks = chunks;

    printf("Initialization...\n");
    auto start = std::chrono::high_resolution_clock::now();
    std::vector<std::thread> threads;
    std::vector<MemCounter *> workers;

    uint64_t init = get_usec();
    for (size_t i = 0; i < MAX_THREADS; ++i) {
        MemCounter *th = new MemCounter(i, &mcp, mcp.chunk_size[i], init);
        workers.push_back(th);
    }
    auto mem_align_counter = new MemAlignCounter(MEM_ALIGN_ROWS, &mcp, init);
    auto end = std::chrono::high_resolution_clock::now();
    auto duration = std::chrono::duration_cast<std::chrono::milliseconds>(end-start);

    std::cout << "Duration initialization " << duration.count() << " ms" << std::endl;

    start = std::chrono::high_resolution_clock::now();
    for (int i = 0; i < MAX_THREADS; ++i) {
        threads.emplace_back([workers, i](){ workers[i]->execute();});
    }
    threads.emplace_back([mem_align_counter](){ mem_align_counter->execute();});

    for (auto& t : threads) {
        t.join();
    }

    end = std::chrono::high_resolution_clock::now();
    duration = std::chrono::duration_cast<std::chrono::milliseconds>(end-start);
    std::cout << "Mem count " << duration.count() << " ms" << std::endl;

    std::cout << "MemAlign " << mem_align_counter->get_instances_count() << " instances, on " << mem_align_counter->get_elapsed_ms() << " ms" << std::endl;

    uint32_t tot_used_slots = 0;
    for (size_t i = 0; i < MAX_THREADS; ++i) {
        uint32_t used_slots = workers[i]->get_used_slots();
        tot_used_slots += used_slots;
        printf("Thread %ld: used slots %d/%d (%04.02f%%) T:%d ms S:%d ms Q:%d\n", i, used_slots, ADDR_SLOTS, ((double)used_slots*100.0)/(double)(ADDR_SLOTS), workers[i]->get_elapsed_ms(), workers[i]->get_tot_usleep()/1000, workers[i]->get_queue_full_times()/1000);
    }
    printf("\n> threads: %d\n", MAX_THREADS);
    printf("> address table: %ld MB\n", (ADDR_TABLE_SIZE * ADDR_TABLE_ELEMENT_SIZE * MAX_THREADS)>>20);
    printf("> memory slots: %ld MB (used: %ld MB)\n", (ADDR_SLOTS_SIZE * sizeof(uint32_t) * MAX_THREADS)>>20, (tot_used_slots * ADDR_SLOT_SIZE * sizeof(uint32_t))>> 20);
    printf("> page table: %ld MB\n\n", (ADDR_PAGE_SIZE * sizeof(uint32_t))>> 20);


    // std::vector<MemPlanner *> planners;

    auto rom_data_planner = new ImmutableMemPlanner(MEM_ROWS, 0x80000000, 128);
    auto input_data_planner = new ImmutableMemPlanner(MEM_ROWS, 0x90000000, 128);
    std::vector<MemPlanner> mem_planners;
    auto mem_planner = new MemPlanner(0, MEM_ROWS, 0xA0000000, 512);
    for (int i = 0; i < MAX_MEM_PLANNERS; ++i) {
        mem_planners.emplace_back(i+1, MEM_ROWS, 0xA0000000, 512);
    }

    MemLocators locators;
    std::vector<std::thread> planner_threads;

    start = std::chrono::high_resolution_clock::now();
    planner_threads.emplace_back([mem_planner, workers, &locators](){ mem_planner->generate_locators(workers, locators);});
    // planner_threads.emplace_back([rom_data_planner, workers, &locators](){ rom_data_planner->execute(workers);});
    // planner_threads.emplace_back([input_data_planner, workers, &locators](){ input_data_planner->execute(workers);});
    for (size_t i = 0; i < mem_planners.size(); ++i) {
        planner_threads.emplace_back([&mem_planners, i, workers, &locators]{ mem_planners[i].execute_from_locators(workers, locators);});
    }

    for (auto& t : planner_threads) {
        t.join();
    }
    end = std::chrono::high_resolution_clock::now();
    duration = std::chrono::duration_cast<std::chrono::milliseconds>(end-start);
    std::cout << "Mem plan " << duration.count() << " ms" << std::endl;
    mem_planner->stats();

    for (uint32_t i = 0; i < mem_planners.size(); ++i) {
        mem_planners[i].stats();
    }

    return 0;
}
#endif
>>>>>>> origin/pre-develop-0.9.0
