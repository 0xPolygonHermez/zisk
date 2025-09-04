#include "mem_context.hpp"
#include "mem_types.hpp"
#include "mem_config.hpp"
#include "mem_locators.hpp"
#include <condition_variable>
#include <mutex>
#include <assert.h>

// Add efficient synchronization variables

//#define MEM_CONTEXT_CV
#ifdef MEM_CONTEXT_CV
static std::mutex chunk_mutex;
static std::condition_variable chunk_cv;
#endif

void MemContext::clear () {
#ifdef MEM_CONTEXT_CV
    std::lock_guard<std::mutex> lock(chunk_mutex);
#endif
    chunks_count.store(0, std::memory_order_release);
    chunks_completed.store(false, std::memory_order_release);
}

#ifdef MEM_CONTEXT_SEM
const MemChunk *MemContext::get_chunk(uint32_t thread_id, uint32_t chunk_id, int64_t &elapsed_us) {
    uint64_t t_ini = get_usec();

    //printf("MemContext::get_chunk: thread_id=%d, chunk_id=%d\n", thread_id, chunk_id);
    sem_wait(&semaphores[thread_id]);

    if (chunk_id < chunks_count.load(std::memory_order_acquire)) {
        #ifdef COUNT_CHUNK_STATS
        #ifdef CHUNK_STATS
        elapsed_us = (int64_t)chunks_us[chunk_id] - (int64_t)get_usec();
        #else
        elapsed_us = 0;
        #endif
        #endif
        return &chunks[chunk_id];
    }

    if (chunks_completed.load(std::memory_order_acquire)) {                
        elapsed_us = get_usec() - t_ini;             
        return nullptr;
    }

    assert(false);
}
#else
const MemChunk *MemContext::get_chunk(uint32_t chunk_id, int64_t &elapsed_us) {
    if (chunk_id < chunks_count.load(std::memory_order_acquire)) {
        #ifdef COUNT_CHUNK_STATS
        #ifdef CHUNK_STATS
        elapsed_us = (int64_t)chunks_us[chunk_id] - (int64_t)get_usec();
        #else
        elapsed_us = 0;
        #endif
        #endif
        return &chunks[chunk_id];
    }
    
    uint64_t t_ini = get_usec();
    
    // Usar condition variable para evitar polling activo
#ifdef MEM_CONTEXT_CV
    std::unique_lock<std::mutex> lock(chunk_mutex);
#endif
    while (chunk_id >= chunks_count.load(std::memory_order_acquire)) {
        if (chunks_completed.load(std::memory_order_acquire)) {                
            elapsed_us = get_usec() - t_ini;             
            return nullptr;
        }

#ifdef MEM_CONTEXT_CV
        // Efficient wait: the thread blocks until it is notified
        chunk_cv.wait_for(lock, std::chrono::microseconds(1000));
#endif
        //usleep(1);
        //printf("MemContext::get_chunk: waiting for chunk_id=%d\n", chunk_id);
    }
    
    elapsed_us = get_usec() - t_ini;
    return &chunks[chunk_id];
}
#endif

MemContext::MemContext() : chunks_count(0), chunks_completed(false) {
#ifdef MEM_CONTEXT_SEM
    for (int i=0; i<(MAX_THREADS + 1); ++i) {
        sem_init(&semaphores[i], 0, 0);
    }
#endif
}

MemContext::~MemContext() {
#ifdef MEM_CONTEXT_SEM
    for (int i=0; i<(MAX_THREADS + 1); ++i) {
        sem_destroy(&semaphores[i]);
    }
#endif
}

void MemContext::add_chunk(MemCountersBusData *data, uint32_t count) {
    
    {
#ifdef MEM_CONTEXT_CV
        std::lock_guard<std::mutex> lock(chunk_mutex);
#endif
        uint32_t chunk_id = chunks_count.load(std::memory_order_relaxed);        
        chunks[chunk_id].data = data;
        chunks[chunk_id].count = count;
        #ifdef CHUNK_STATS
        chunks_us[chunk_id] = get_usec();
        #endif
        chunks_count.store(chunk_id + 1, std::memory_order_release);
    }

    // Notify ALL waiting threads
#ifdef MEM_CONTEXT_SEM
    for (int i=0; i<(MAX_THREADS + 1); ++i) {
        sem_post(&semaphores[i]);
    }
#elif defined(MEM_CONTEXT_CV)
    chunk_cv.notify_all();
#endif
}

void MemContext::stats() {
    #ifdef CHUNK_STATS
    uint32_t chunks_count = size();
    if (chunks_count > 0) {
        printf("chunks_us: %ld", chunks_us[0] - t_init_us);
        for (size_t j = 1; j < chunks_count; ++j) {
            printf(";%ld", chunks_us[j] - chunks_us[j-1]);
        }
        printf("\n");
    }    
    if (chunks_count > 0) {
        printf("chunks_count: %d", chunks[0].count);
        for (size_t j = 1; j < chunks_count; ++j) {
            printf(";%d", chunks[j].count);
        }
        printf("\n");
    }    
    #endif
}