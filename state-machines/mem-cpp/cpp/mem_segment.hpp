#ifndef __MEM_SEGMENT_HPP__
#define __MEM_SEGMENT_HPP__
#include <string.h>
#include <map>
#include <vector>
#include <unordered_map>
#include <stdexcept>
#include "mem_config.hpp"
#include "mem_check_point.hpp"

#define MEM_SEGMENT_HASH_TABLE_KEY_NOT_FOUND 0xFFFFFFFF
class MemSegmentHashTable {
private:
    uint32_t hash_id;
    uint32_t *hash_table;
    uint32_t hash_count;
    uint32_t hash_bits;
    uint32_t hash_mask;
    uint32_t get_hash_bits(uint32_t key_size) {
        uint32_t bits = 0;
        while (key_size > 1) {
            key_size >>= 1;
            ++bits;
        }
        return bits;
    }
public:
    MemSegmentHashTable(uint32_t key_size) : hash_count(key_size) {
        hash_bits = get_hash_bits(hash_count);
        hash_mask = (1 << hash_bits) - 1;
        hash_table = (uint32_t *)malloc(hash_count * sizeof(uint32_t));
        full_reset();
    }
    ~MemSegmentHashTable() {
        free(hash_table);
    }
    uint32_t get_new_hash_id() {
        if (hash_id >= hash_count) {
            throw std::runtime_error("Error: MemSegmentHashTable::get_new_hash_id: hash_id out of bounds");
        }
        return hash_id++;
    }
    void set(uint32_t key, uint32_t pos) {
        hash_table[key] = hash_id | pos;
    }
    uint32_t get(uint32_t key) {
        uint32_t value = hash_table[key];
        if (value < hash_id) {
            return MEM_SEGMENT_HASH_TABLE_KEY_NOT_FOUND;
        }
        uint32_t result = value & hash_mask;
        return result;
    }
    void full_reset() {
        hash_id = 1 << hash_bits;
        memset(hash_table, 0, hash_count * sizeof(uint32_t));
    }
    void fast_reset() {
        hash_id = hash_id + (1 << hash_bits);
        if (hash_id == 0) {
            full_reset();
        }
    }
};

class MemSegment {
    std::unordered_map<uint32_t, uint32_t> mapping;
    uint32_t chunks_count = 0;
    MemCheckPoint *chunks;
public:
    bool is_last_segment;
    MemSegment() : is_last_segment(false) {
        init();
    }
    MemSegment(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count): is_last_segment(false) {
        init();
        push(chunk_id, from_addr, skip, count);
    }
    void init() {
        chunks_count = 0;
        mapping.reserve(MAX_CHUNKS);
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