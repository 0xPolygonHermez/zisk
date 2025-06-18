#ifndef __MEM_ALIGN_COUNTER_HPP__
#define __MEM_ALIGN_COUNTER_HPP__

#include "mem_config.hpp"
#include "mem_types.hpp"
#include "mem_context.hpp"
#include "tools.hpp"
#include <vector>
#include <assert.h>


struct MemAlignCheckPoint {
    uint32_t segment_id;
    uint32_t chunk_id;
    uint32_t skip; // unaligned ops
    uint32_t count; // unaligned ops
    uint32_t rows;  // rows
    uint32_t offset; // row offset
};

class MemAlignCounter {
private:
    MemContext *context;
    std::vector<MemAlignCheckPoint> checkpoints;
    uint32_t count;
    int32_t segment_id;
    uint32_t available_rows;
    uint32_t skip;
    uint32_t rows;
    uint32_t elapsed_ms;
public:
    MemAlignCounter(uint32_t rows, MemContext *context) :context(context), rows(rows) {
        count = 0;
        available_rows = 0;
        segment_id = -1;
        skip = 0;
    }
    ~MemAlignCounter() {
    }
    void execute() {
        uint64_t init = get_usec();
        const MemChunk *chunk;
        uint32_t chunk_id = 0;
        while ((chunk = context->get_chunk(chunk_id)) != nullptr) {
            execute_chunk(chunk_id, chunk->data, chunk->count);
            ++chunk_id;
        }
        elapsed_ms = ((get_usec() - init) / 1000);
        }
    void execute_chunk(uint32_t chunk_id, const MemCountersBusData *chunk_data, uint32_t chunk_size) {
        skip = 0;
        for (uint32_t i = 0; i < chunk_size; i++) {
            const uint8_t bytes = chunk_data[i].flags & 0xFF;
            const uint32_t addr = chunk_data[i].addr;
            assert(bytes == 1 || bytes == 2 || bytes == 4 || bytes == 8);
            if (bytes != 8 || (addr & 0x07) != 0) {
                uint32_t addr_count = (bytes + (addr & 0x07)) > 8 ? 2:1;
                uint32_t ops_by_addr = (chunk_data[i].flags & 0x10000) ? 2:1;
                uint32_t ops = addr_count * ops_by_addr + 1;
                add_mem_align_op(chunk_id, ops);
                skip = skip + 1;
    }
        }
    }
    void add_mem_align_op(uint32_t chunk_id, uint32_t ops) {
        if (available_rows < ops) {
            open_segment(chunk_id, ops);
        } else {
            MemAlignCheckPoint &lcp = checkpoints.back();
            if (lcp.chunk_id != chunk_id) {
                open_chunk(chunk_id, ops);
            } else {
                lcp.count += 1;
                lcp.rows += ops;
            }
        }
        available_rows -= ops;
    }
    void open_chunk(uint32_t chunk_id, uint32_t ops = 0) {
        uint32_t count = ops ? 1 : 0;
        checkpoints.emplace_back(MemAlignCheckPoint{(uint32_t)segment_id, chunk_id, 0, count, ops, rows - available_rows});
    }
    void open_segment(uint32_t chunk_id, uint32_t ops = 0) {
        uint32_t count = ops ? 1 : 0;
        ++segment_id;
        checkpoints.emplace_back(MemAlignCheckPoint{(uint32_t)segment_id, chunk_id, skip, count, ops, 0});
        available_rows = rows;
    }
    uint32_t size() {
        return checkpoints.size();
    }
    const MemAlignCheckPoint *get_checkpoints() {
        return checkpoints.data();
    }
    uint32_t get_elapsed_ms() {
        return elapsed_ms;
    }
    void debug (void) {
        uint32_t index = 0;
        uint32_t last_segment_id = 0;
        for (auto &cp: checkpoints) {
            if (cp.segment_id != last_segment_id) {
                index = 0;
                last_segment_id = cp.segment_id;
            }
            printf("MEM_ALIGN %d:%d #%d S:%d C:%d R:%d O:%d\n", cp.segment_id, cp.chunk_id, index++, cp.skip, cp.count, cp.rows, cp.offset);
}
    }
};


#endif // __MEM_ALIGN_COUNTER_HPP__