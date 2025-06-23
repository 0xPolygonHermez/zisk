#ifndef __MEM_ALIGN_COUNTER_HPP__
#define __MEM_ALIGN_COUNTER_HPP__

#include "mem_config.hpp"
#include "mem_types.hpp"
#include "mem_context.hpp"
#include "tools.hpp"
#include <vector>
#include <assert.h>
#include <memory>   

struct MemAlignCheckPoint {
    uint32_t segment_id;
    uint32_t chunk_id;
    uint32_t skip; // skip specified as unaligned ops
    uint32_t count; // count as specified unaligned ops
    uint32_t rows;  // rows
    uint32_t offset; // row offset
};

class MemAlignCounter {
private:
    std::shared_ptr<MemContext> context;
    std::vector<MemAlignCheckPoint> checkpoints;
    uint32_t count;
    int32_t segment_id;
    uint32_t available_rows;
    uint32_t skip;
    uint32_t rows;
    uint32_t elapsed_ms;
public:
    uint64_t total_usleep;
    MemAlignCounter (uint32_t rows, std::shared_ptr<MemContext> context);
    void execute (void);
    void execute_chunk (uint32_t chunk_id, const MemCountersBusData *chunk_data, uint32_t chunk_size);
    void add_mem_align_op(uint32_t chunk_id, uint32_t ops);
    void open_chunk(uint32_t chunk_id, uint32_t ops = 0);
    void open_segment(uint32_t chunk_id, uint32_t ops = 0);
    uint32_t size() {
        return checkpoints.size();
    }
    const MemAlignCheckPoint *get_checkpoints() {
        return checkpoints.data();
    }
    uint32_t get_elapsed_ms() {
        return elapsed_ms;
    }
    void debug (void);
};


#endif // __MEM_ALIGN_COUNTER_HPP__
