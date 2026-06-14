#ifndef __MEM_SEGMENT_HPP__
#define __MEM_SEGMENT_HPP__
#include <string.h>
#include <map>
#include <vector>
#include <unordered_map>
#include <stdexcept>
#include <span>

#include "mem_config.hpp"
#include "mem_segment_hash_table.hpp"
#include "mem_check_point.hpp"
#include "instance_meta.hpp"
class MemSegment {
    std::unordered_map<uint32_t, uint32_t> mapping;
    uint32_t chunks_count = 0;
    MemCheckPoint *chunks;
public:
    bool is_last_segment;
    uint32_t offsets_base_addr;
    PagedOffsets offsets;

    MemSegment(const MemSegment&) = delete;
    MemSegment& operator=(const MemSegment&) = delete;
    MemSegment(MemSegment&&) noexcept = delete;

    MemSegment()
        : is_last_segment(false), offsets_base_addr(0),
          offsets{nullptr, nullptr, nullptr, 0, 0, 0} {
        chunks = nullptr;
        init();
    }
    MemSegment(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count)
        : is_last_segment(false), offsets_base_addr(0),
          offsets{nullptr, nullptr, nullptr, 0, 0, 0} {
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
        if (chunks_count >= MAX_CHUNKS) return;
        uint32_t next_index = chunks_count++;
        mapping.emplace(chunk_id, next_index);
        chunks[next_index].set(chunk_id, from_addr, skip, count);
    }

    void push(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t to_addr, uint32_t to_count, uint32_t count) {
        if (chunks_count >= MAX_CHUNKS) return;
        uint32_t next_index = chunks_count++;
        mapping.emplace(chunk_id, next_index);
        chunks[next_index].set(chunk_id, from_addr, skip, to_addr, to_count, count);
    }

    void swap_last_and_first() {
        if (chunks_count < 2) return; // No need to swap if less than 2 chunks
        std::swap(chunks[0], chunks[chunks_count - 1]);
        // Update mapping for the swapped chunk IDs
        mapping[chunks[0].chunk_id] = 0;
        mapping[chunks[chunks_count - 1].chunk_id] = chunks_count - 1;
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
    bool compare(const MemSegment &other, uint32_t segment_id = 0) const {
        bool equal = true;
        // Check chunks present in this but not in other, or with different values
        for (const auto &[chunk_id, index] : mapping) {
            auto it = other.mapping.find(chunk_id);
            if (it == other.mapping.end()) {
                printf("DIFF #%d@%d: only in A [0x%08X s:%d] [0x%08X C:%d] C:%d\n",
                    segment_id, chunk_id, chunks[index].from_addr, chunks[index].from_skip,
                    chunks[index].to_addr, chunks[index].to_count, chunks[index].count);
                equal = false;
            } else {
                const MemCheckPoint &a = chunks[index];
                const MemCheckPoint &b = other.chunks[it->second];
                if (a.from_addr != b.from_addr || a.from_skip != b.from_skip ||
                    a.to_addr != b.to_addr || a.to_count != b.to_count || a.count != b.count) {
                    printf("DIFF #%d@%d: A [0x%08X s:%d] [0x%08X C:%d] C:%d  vs  B [0x%08X s:%d] [0x%08X C:%d] C:%d\n",
                        segment_id, chunk_id,
                        a.from_addr, a.from_skip, a.to_addr, a.to_count, a.count,
                        b.from_addr, b.from_skip, b.to_addr, b.to_count, b.count);
                    equal = false;
                }
            }
        }
        // Check chunks present in other but not in this
        for (const auto &[chunk_id, index] : other.mapping) {
            if (mapping.find(chunk_id) == mapping.end()) {
                printf("DIFF #%d@%d: only in B [0x%08X s:%d] [0x%08X C:%d] C:%d\n",
                    segment_id, chunk_id, other.chunks[index].from_addr, other.chunks[index].from_skip,
                    other.chunks[index].to_addr, other.chunks[index].to_count, other.chunks[index].count);
                equal = false;
            }
        }
        return equal;
    }
};

#endif