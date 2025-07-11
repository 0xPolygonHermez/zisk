#ifndef __MEM_SEGMENT_HPP__
#define __MEM_SEGMENT_HPP__
#include <string.h>
#include <map>
#include <vector>
#include <unordered_map>
#include <stdexcept>
#include "mem_config.hpp"
#include "mem_segment_hash_table.hpp"
#include "mem_check_point.hpp"
class MemSegment {
    std::unordered_map<uint32_t, uint32_t> mapping;
    uint32_t chunks_count = 0;
    MemCheckPoint *chunks;
public:
    bool is_last_segment;

    MemSegment(const MemSegment&) = delete;
    MemSegment& operator=(const MemSegment&) = delete;
    MemSegment(MemSegment&&) noexcept = delete;

    MemSegment() : is_last_segment(false) {
        chunks = nullptr;
        init();
    }
    MemSegment(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count): is_last_segment(false) {
        chunks = nullptr;
        init();
        push(chunk_id, from_addr, skip, count);
    }
    ~MemSegment() {
        if (chunks != nullptr) {
            free(chunks);
            chunks = nullptr;
        }
    }
    void init() {
        chunks_count = 0;
        mapping.reserve(MAX_CHUNKS);
        if (chunks != nullptr) {
            free(chunks);
        }
        chunks = (MemCheckPoint *)malloc(sizeof(MemCheckPoint) * MAX_CHUNKS);
    }
    void push(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count) {
        uint32_t next_index = chunks_count++;
        mapping.emplace(chunk_id, next_index);
        chunks[next_index].set(chunk_id, from_addr, skip, count);
    }

    void add_or_update(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count ) {
        auto it = mapping.find(chunk_id);
        if (it != mapping.end()) {
            chunks[it->second].add_rows(from_addr, count);
        } else {
            push(chunk_id, from_addr, skip, count);
        }
    }
    uint32_t size() const {
        return chunks_count;
    }
    const MemCheckPoint *get_chunks() const {
        return chunks;
    }
    void debug(uint32_t segment_id = 0) {
        for (const auto &[chunk_id, index] : mapping) {
            printf("#%d@%d [0x%08X s:%d] [0x%08X C:%d] C:%d\n", segment_id, chunk_id, chunks[index].from_addr, chunks[index].from_skip,
                chunks[index].to_addr, chunks[index].to_count, chunks[index].count);
        }
    }
};

#endif