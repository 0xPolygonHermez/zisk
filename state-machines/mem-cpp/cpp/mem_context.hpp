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

class MemContext;

#include "mem_types.hpp"
#include "mem_config.hpp"
#include "mem_locators.hpp"

class MemContext {
public:
    MemChunk chunks[MAX_CHUNKS];
    MemLocators locators;
    std::atomic<uint32_t> chunks_count;
    std::atomic<bool> chunks_completed;
    void clear ();
    const MemChunk *get_chunk(uint32_t chunk_id, uint64_t &elapsed_us);
    MemContext();
    void add_chunk(MemCountersBusData *data, uint32_t count);
    void set_completed() {
        chunks_completed.store(true, std::memory_order_release);
    }
    uint32_t size() {
        return chunks_count.load(std::memory_order_acquire);
    }
};

#endif