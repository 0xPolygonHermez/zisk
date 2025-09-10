#ifndef __MEM_ALIGN_COUNTER_HPP__
#define __MEM_ALIGN_COUNTER_HPP__

#include "mem_config.hpp"
#include "mem_types.hpp"
#include "mem_context.hpp"
#include "tools.hpp"
#include <vector>
#include <assert.h>
#include <memory>   

struct MemAlignChunkCounters {
    uint32_t chunk_id;
    uint32_t full_5;
    uint32_t full_3;
    uint32_t full_2;
    uint32_t read_byte;
    uint32_t write_byte;
};

class MemAlignCounter {
private:
    std::shared_ptr<MemContext> context;
    std::vector<MemAlignChunkCounters> counters;
    MemAlignChunkCounters total_counters;
    uint32_t rows;    
    uint32_t elapsed_ms;
public:
    uint64_t total_usleep;
    MemAlignCounter (uint32_t rows, std::shared_ptr<MemContext> context);
    void execute ();
    void execute_chunk (uint32_t chunk_id, const MemCountersBusData *chunk_data, uint32_t chunk_size);
    uint32_t size() {
        return counters.size();
    }
    const MemAlignChunkCounters *get_counters() {
        return counters.data();
    }
    const MemAlignChunkCounters *get_total_counters() {
        return &total_counters;
    }
    uint32_t get_elapsed_ms() {
        return elapsed_ms;
    }
    void debug (void);
};


#endif // __MEM_ALIGN_COUNTER_HPP__
