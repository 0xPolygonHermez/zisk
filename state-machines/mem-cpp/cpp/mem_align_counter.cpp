#include "mem_align_counter.hpp"
#include "mem_config.hpp"
#include "mem_types.hpp"
#include "mem_context.hpp"
#include "tools.hpp"
#include <vector>
#include <assert.h>


MemAlignCounter::MemAlignCounter(uint32_t rows, std::shared_ptr<MemContext> context) :context(context), rows(rows) {
    count = 0;
    available_rows = 0;
    segment_id = -1;
    skip = 0;
}

void MemAlignCounter::execute() {
    uint64_t init = get_usec();
    const MemChunk *chunk;
    uint32_t chunk_id = 0;
    int64_t elapsed_us = 0;
#ifdef MEM_CONTEXT_SEM
    while ((chunk = context->get_chunk(MAX_THREADS, chunk_id, elapsed_us)) != nullptr) {
#else
    while ((chunk = context->get_chunk(chunk_id, elapsed_us)) != nullptr) {
#endif
        execute_chunk(chunk_id, chunk->data, chunk->count);
        #ifdef COUNT_CHUNK_STATS
        #ifdef CHUNK_STATS
        total_usleep += elapsed_us > 0 ? elapsed_us : 0;
        #else
        total_usleep += elapsed_us;
        #endif
        #endif
        ++chunk_id;
    }
    elapsed_ms = ((get_usec() - init) / 1000);
}

void MemAlignCounter::execute_chunk(uint32_t chunk_id, const MemCountersBusData *chunk_data, uint32_t chunk_size) {
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

void MemAlignCounter::add_mem_align_op(uint32_t chunk_id, uint32_t ops) {
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

void MemAlignCounter::open_chunk(uint32_t chunk_id, uint32_t ops) {
    uint32_t count = ops ? 1 : 0;
    checkpoints.emplace_back(MemAlignCheckPoint{(uint32_t)segment_id, chunk_id, 0, count, ops, rows - available_rows});
}

void MemAlignCounter::open_segment(uint32_t chunk_id, uint32_t ops) {
    uint32_t count = ops ? 1 : 0;
    ++segment_id;
    checkpoints.emplace_back(MemAlignCheckPoint{(uint32_t)segment_id, chunk_id, skip, count, ops, 0});
    available_rows = rows;
}

void MemAlignCounter::debug (void) {
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

