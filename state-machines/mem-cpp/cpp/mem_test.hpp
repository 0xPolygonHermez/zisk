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

#include "api.hpp"
#include "mem_types.hpp"
#include "mem_config.hpp"
#include "mem_context.hpp"
#include "tools.hpp"
#include "mem_count_and_plan.hpp"

class MemTestChunk {
public:
    std::shared_ptr<MemCountersBusData> chunk_data;
    uint32_t chunk_size;
    MemTestChunk(MemCountersBusData *data, uint32_t size) 
        : chunk_data(data, [](MemCountersBusData* p) { free(p); }), chunk_size(size) {}
    ~MemTestChunk() {
        // Memory is automatically freed by shared_ptr with custom deleter
    }
};

class MemTest {
private:
    std::vector<MemTestChunk> chunks;
public:
    MemTest() {
        chunks.reserve(MAX_CHUNKS);
    }
    void load(const char *path) {
        printf("Loading compact data...\n");
        uint32_t tot_chunks = 0;
        uint32_t tot_ops = 0;
        uint32_t chunk_id;
        int32_t chunk_size;
        MemCountersBusData *chunk_data;
        // A chunk file holds the chunk's stream words (mops_format.hpp); chunk_size counts words.
        while ((chunk_id = chunks.size()) < MAX_CHUNKS && (chunk_size = load_from_compact_file(path, chunk_id, &chunk_data)) >=0) {
            chunks.emplace_back(chunk_data, chunk_size);
            tot_ops += count_operations(chunk_data, chunk_size);
            tot_chunks += chunk_size;
            #ifdef DEBUG_INFO
            // if (chunk_id == 999999) {
                const uint64_t *words = (const uint64_t *)chunk_data;
                for (int32_t i = 0; i < chunk_size; i += mops_record_len(words[i])) {
                    const MemCountersBusData rec = mops_decode_record(words + i);
                    const uint32_t addr = rec.addr;
                    if (addr < INPUT_ADDR || addr >= ROM_ADDR) continue;
                    const uint8_t bytes = rec.flags & 0x0F;
                    const uint8_t is_write = (rec.flags & MOPS_WRITE_FLAG) != 0;
                    if ((addr & 0x7) == 0 && bytes == 8) {
                        printf("MEM_OP 0x%08X %04d:%07d:0\n", addr, chunk_id, i);
                    } else if ((addr & 0x7) != 0 || bytes != 8) {
                        uint32_t aligned_addr = addr & 0xFFFFFFF8;
                        if ((addr & 0x7) + bytes > 8) {
                            printf("MEM_OP 0x%08X %04d:%07d:0 offset %d bytes %d RD1\n", aligned_addr, chunk_id, i, addr & 0x7, bytes);
                            if (is_write) {
                                printf("MEM_OP 0x%08X %04d:%07d:1 offset %d bytes %d WR1\n", aligned_addr, chunk_id, i ,addr & 0x7, bytes);
                            }
                            aligned_addr += 8;
                            printf("MEM_OP 0x%08X %04d:%07d:0 offset %d bytes %d RD2\n", aligned_addr, chunk_id, i, addr & 0x7, bytes);
                            if (is_write) {
                                printf("MEM_OP 0x%08X %04d:%07d:1 offset %d bytes %d WR2\n", aligned_addr, chunk_id, i, addr & 0x7, bytes);
                            }
                        } else {
                            printf("MEM_OP 0x%08X %04d:%07d:0 offset %d bytes %d RD\n", aligned_addr, chunk_id, i, addr & 0x7, bytes);
                            if (is_write) printf("MEM_ALIGN_OP 0x%08X %04d:%07d:1 offset %d bytes %d WR\n", aligned_addr, chunk_id, i, addr & 0x7, bytes);
                        }
                    }

                }
            // }
            #endif
            if (chunk_id % 100 == 0) printf("Loaded chunk %d with size %d\n", chunk_id, chunk_size);
        }
        printf("chunks: %ld  tot_chunks: %d tot_ops: %d tot_time:%ld (ms) Speed(Mhz): %04.2f\n", chunks.size(), tot_chunks, tot_ops, (chunks.size() * TIME_US_BY_CHUNK)/1000, (double)(CHUNK_SIZE) / TIME_US_BY_CHUNK);
    }
    void execute(void) {
        printf("Starting...\n");
        auto cp = create_mem_count_and_plan(1u << 21, 1u << 21, 1u << 22);
        printf("Executing...\n");
        execute_mem_count_and_plan(cp);
        uint64_t init = get_usec();
        uint32_t chunk_id = 0;
        for (auto& chunk : chunks) {
            uint64_t chunk_ready = init + (uint64_t)(chunk_id+1) * TIME_US_BY_CHUNK;
            uint64_t current = get_usec();
            if (current < chunk_ready) {
                uint64_t wait_time = chunk_ready - current;
                // Optimization: busy wait for short delays
                if (wait_time < 100) {
                    // Busy wait for < 100μs (more accurate but consumes CPU)
                    while (get_usec() < chunk_ready) {
                        // Spin wait
                    }
                } else {
                    // usleep for long delays (saves CPU)
                    usleep(wait_time);
                }
            }
            MemCountersBusData *data = chunk.chunk_data.get();
            uint32_t chunk_size = chunk.chunk_size;
//            uint32_t j = chunk_size - 1;
//            printf("CHUNK[%4d] 0:[%08X %d %c] ... %d:[%08X %d %c]\n", chunk_id,
//                data[0].addr, data[0].flags & 0xFFFF, data[0].flags & MEM_WRITE_FLAG ? 'R':'W', j,
//                data[j].addr, data[j].flags & 0xFFFF, data[j].flags & MEM_WRITE_FLAG ? 'R':'W');
            add_chunk_mem_count_and_plan(cp, data, chunk_size);
            ++chunk_id;
        }
        set_completed_mem_count_and_plan(cp);
        wait_mem_count_and_plan(cp);
        stats_mem_count_and_plan(cp);
        destroy_mem_count_and_plan(cp);
    }
};
#endif
