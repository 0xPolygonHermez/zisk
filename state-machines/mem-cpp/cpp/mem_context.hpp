#ifndef __MEM_CONTEXT_HPP__
#define __MEM_CONTEXT_HPP__

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
#include "mem_locators.hpp"

class MemContext {
public:
    MemChunk chunks[MAX_CHUNKS];
    MemLocators locators;
    std::atomic<uint32_t> chunks_count;
    std::atomic<bool> chunks_completed;
    void clear () {
        chunks_count.store(0, std::memory_order_release);
        chunks_completed.store(false, std::memory_order_release);
    }
    const MemChunk *get_chunk(uint32_t chunk_id) {
        while (chunk_id >= chunks_count.load(std::memory_order_acquire)) {
            if (chunks_completed.load(std::memory_order_acquire)) {
                return nullptr;
            }
            usleep(1);
        }
        return &chunks[chunk_id];
    }

    MemContext() : chunks_count(0), chunks_completed(false) {
    }
    void add_chunk(MemCountersBusData *data, uint32_t count) {
        uint32_t chunk_id = chunks_count.load(std::memory_order_relaxed);
        chunks[chunk_id].data = data;
        chunks[chunk_id].count = count;
        chunks_count.store(chunk_id + 1, std::memory_order_release);
    }
    void set_completed() {
        chunks_completed.store(true, std::memory_order_release);
    }
    uint32_t size() {
        return chunks_count.load(std::memory_order_acquire);
    }
};

#endif