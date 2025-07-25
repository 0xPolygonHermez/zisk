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
#include "tools.hpp"

class MemContext {
public:
    MemChunk chunks[MAX_CHUNKS];
    MemLocators locators;
    uint64_t t_init_us;    
    uint64_t t_first_us;
    uint64_t t_completed_us;
    std::atomic<uint32_t> chunks_count;
    std::atomic<bool> chunks_completed;
#ifdef CHUNK_STATS
    uint64_t chunks_us[MAX_CHUNKS];
#endif
    void clear ();
    const MemChunk *get_chunk(uint32_t chunk_id, int64_t &elapsed_us);
    MemContext();
    void add_chunk(MemCountersBusData *data, uint32_t count);
    void init() {
        t_init_us = get_usec();
    }
    uint64_t get_init_us() {
        return t_init_us;
    }
    void set_completed() {
        t_completed_us = get_usec();
        chunks_completed.store(true, std::memory_order_release);
    }
    uint64_t get_completed_us() {
        return t_completed_us - t_init_us;
    }
    uint32_t size() {
        return chunks_count.load(std::memory_order_acquire);
    }
    void stats();
};

#endif