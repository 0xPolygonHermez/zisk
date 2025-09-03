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
#include <semaphore.h>

class MemContext;

#include "mem_types.hpp"
#include "mem_config.hpp"
#include "mem_locators.hpp"
#include "tools.hpp"

#define MEM_CONTEXT_SEM

class MemContext {
public:
    MemChunk chunks[MAX_CHUNKS];
    MemLocators locators;
    uint64_t t_init_us;    
    uint64_t t_first_us;
    uint64_t t_completed_us;
    std::atomic<uint32_t> chunks_count;
    std::atomic<bool> chunks_completed;
#ifdef MEM_CONTEXT_SEM
    sem_t semaphores[MAX_THREADS + 1];
#endif
#ifdef CHUNK_STATS
    uint64_t chunks_us[MAX_CHUNKS];
#endif
    void clear ();
#ifdef MEM_CONTEXT_SEM
    const MemChunk *get_chunk(uint32_t thread_id, uint32_t chunk_id, int64_t &elapsed_us);
#else
    const MemChunk *get_chunk(uint32_t chunk_id, int64_t &elapsed_us);
#endif
    MemContext();
    ~MemContext();
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
#ifdef MEM_CONTEXT_SEM
        // Wakeup counter threads
        for (int i=0; i<(MAX_THREADS + 1); ++i) {
            sem_post(&semaphores[i]);
        }
#endif
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