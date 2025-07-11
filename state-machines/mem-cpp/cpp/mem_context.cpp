#include "mem_context.hpp"
#include "mem_types.hpp"
#include "mem_config.hpp"
#include "mem_locators.hpp"

void MemContext::clear () {
    chunks_count.store(0, std::memory_order_release);
    chunks_completed.store(false, std::memory_order_release);
}
const MemChunk *MemContext::get_chunk(uint32_t chunk_id, uint64_t &elapsed_us) {
    if (chunk_id < chunks_count.load(std::memory_order_acquire)) {
        elapsed_us = 0;
        return &chunks[chunk_id];
    }
    uint64_t t_ini = get_usec();
    usleep(1);
    while (chunk_id >= chunks_count.load(std::memory_order_acquire)) {
        if (chunks_completed.load(std::memory_order_acquire)) {                
            return nullptr;
        }
        usleep(1);
    }
    elapsed_us = get_usec() - t_ini;
    return &chunks[chunk_id];
}

MemContext::MemContext() : chunks_count(0), chunks_completed(false) {
}
void MemContext::add_chunk(MemCountersBusData *data, uint32_t count) {
    uint32_t chunk_id = chunks_count.load(std::memory_order_relaxed);        
    chunks[chunk_id].data = data;
    chunks[chunk_id].count = count;
    chunks_count.store(chunk_id + 1, std::memory_order_release);
}
