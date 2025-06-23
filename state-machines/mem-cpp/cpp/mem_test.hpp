#ifndef __MEM_TEST_HPP__
#define __MEM_TEST_HPP__

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
#include "mem_context.hpp"
#include "tools.hpp"
#include "mem_count_and_plan.hpp"

class MemTestChunk {
public:
    MemCountersBusData *chunk_data;
    uint32_t chunk_size;
    MemTestChunk(MemCountersBusData *data, uint32_t size) : chunk_data(data), chunk_size(size) {}
    ~MemTestChunk() {
//        free(chunk_data);
    }
};

class MemTest {
private:
    std::vector<MemTestChunk> chunks;
public:
    MemTest() {
        chunks.reserve(4096);
    }
    ~MemTest() {
        for (auto& chunk : chunks) {
            free(chunk.chunk_data);
        }
    }
    void load() {
        printf("Loading compact data...\n");
        uint32_t tot_chunks = 0;
        uint32_t tot_ops = 0;
        uint32_t chunk_id;
        int32_t chunk_size;
        MemCountersBusData *chunk_data;
        while ((chunk_id = chunks.size()) < MAX_CHUNKS && (chunk_size = load_from_compact_file(chunk_id, &chunk_data)) >=0) {
            chunks.emplace_back(chunk_data, chunk_size);
            tot_ops += count_operations(chunk_data, chunk_size);
            tot_chunks += chunk_size;
            if (chunk_id % 100 == 0) printf("Loaded chunk %d with size %d\n", chunk_id, chunk_size);
        }
        printf("chunks: %ld  tot_chunks: %d tot_ops: %d tot_time:%ld (ms)\n", chunks.size(), tot_chunks, tot_ops, (chunks.size() * TIME_US_BY_CHUNK)/1000);
    }
    void execute(void) {
        printf("Starting...\n");
        auto cp = create_mem_count_and_plan();
        printf("Executing...\n");
        execute_mem_count_and_plan(cp);
        uint64_t init = get_usec();
        uint32_t chunk_id = 0;
        for (auto& chunk : chunks) {
            uint64_t chunk_ready = init + (uint64_t)(chunk_id+1) * TIME_US_BY_CHUNK;
            uint64_t current = get_usec();
            if (current < chunk_ready) {
                usleep(chunk_ready - current);
            }
            MemCountersBusData *data = chunk.chunk_data;
            uint32_t chunk_size = chunk.chunk_size;
//            uint32_t j = chunk_size - 1;
//            printf("CHUNK[%4d] 0:[%08X %d %c] ... %d:[%08X %d %c]\n", chunk_id,
//                data[0].addr, data[0].flags & 0xFFFF, data[0].flags & 0x10000 ? 'R':'W', j,
//                data[j].addr, data[j].flags & 0xFFFF, data[j].flags & 0x10000 ? 'R':'W');
            add_chunk_mem_count_and_plan(cp, data, chunk_size);
            ++chunk_id;
        }
        set_completed_mem_count_and_plan(cp);
        wait_mem_count_and_plan(cp);
        stats_mem_count_and_plan(cp);
    }
};
#endif