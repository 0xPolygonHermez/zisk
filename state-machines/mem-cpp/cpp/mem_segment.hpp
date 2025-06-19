#ifndef __MEM_SEGMENT_HPP__
#define __MEM_SEGMENT_HPP__
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

/*
class MemSegmentFast {
private:
    MemSegmentHashTable *hash_table;
    uint32_t push(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count) {
        uint32_t index = chunks.size();
        hash_table->set(chunk_id, index);
        chunks.emplace_back(chunk_id, from_addr, skip, count);
        return index;
    }
public:
    std::vector<MemCheckPoint> chunks;
    MemSegmentFast(MemSegmentHashTable *hash_table): hash_table(hash_table) {
        chunks.reserve(1024);
    }
    MemSegmentFast(MemSegmentHashTable *hash_table, uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count) : hash_table(hash_table) {
        chunks.reserve(1024);
    }

    #ifdef MEM_CHECK_POINT_MAP
    void add_or_update(uint32_t chunk_id, uint32_t from_addr, uint32_t count, uint32_t skip = 0) {
        uint32_t index = hash_table->get(chunk_id);
        if (index == MEM_SEGMENT_HASH_TABLE_KEY_NOT_FOUND) {
            push(chunk_id, from_addr, skip, count);
        } else {
            chunks[index].add_rows(from_addr, count);
        }
    }
    #endif
    uint32_t size() const {
        return chunks.size();
    }
};

*/

class MemSegment {
    #ifdef MEM_CHECK_POINT_MAP
    std::map<uint32_t, MemCheckPoint> chunks;
    // std::unordered_map<uint32_t, MemCheckPoint> chunks;
    #else
    std::vector<MemCheckPoint> chunks;
    #endif
public:
    bool is_last_segment;
    #ifdef MEM_CHECK_POINT_MAP
    MemSegment() : is_last_segment(false) {
        // chunks.reserve(4096);
    }
    MemSegment(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count) {
        // chunks.reserve(4096);
        add_or_update(chunk_id, from_addr, skip, count);
    }
    #else
    MemSegment(MemSegmentHashTable *hash_table) : is_last_segment(false) {
        chunks.reserve(4096);
        hash_table->fast_reset();
    }
    MemSegment(MemSegmentHashTable *hash_table, uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count) {
        chunks.reserve(4096);
        hash_table->fast_reset();
        add_or_update(hash_table, chunk_id, from_addr, skip, count);
    }
    #endif
    #ifdef MEM_CHECK_POINT_MAP
    void push(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count) {
        chunks.try_emplace(chunk_id, from_addr, skip, count);
    }
    #else
    void push(MemSegmentHashTable *hash_table, uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count) {
        uint32_t index = chunks.size();
        hash_table->set(chunk_id, index);
        chunks.emplace_back(from_addr, skip, count);
    }
    #endif

    #ifdef MEM_CHECK_POINT_MAP
    void add_or_update(uint32_t chunk_id, uint32_t from_addr, uint32_t count, uint32_t skip = 0) {
        auto result = chunks.try_emplace(chunk_id, std::move(MemCheckPoint(from_addr, 0, count)));
        if (!result.second) {
            result.first->second.add_rows(from_addr, count);
        }
    }
    #else
    void add_or_update(MemSegmentHashTable *hash_table, uint32_t chunk_id, uint32_t from_addr, uint32_t count, uint32_t skip = 0) {
        uint32_t index = hash_table->get(chunk_id);
        if (index == MEM_SEGMENT_HASH_TABLE_KEY_NOT_FOUND) {
            push(hash_table, chunk_id, from_addr, skip, count);
        } else {
            chunks[index].add_rows(from_addr, count);
        }
    }
    #endif
    uint32_t size() const {
        return chunks.size();
    }
};

// class MemSegment {
// public:
//     #ifdef MEM_CHECK_POINT_MAP
//     std::unordered_map<uint32_t, MemCheckPoint> chunks;
//     #else
//     std::vector<MemCheckPoint> chunks;
//     #endif
//     bool is_last_segment;
//     MemSegment() : is_last_segment(false) {
//         chunks.reserve(1024);
//     }
//     MemSegment(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count) : is_last_segment(false) {
//         chunks.reserve(1024);
//         #ifdef MEM_CHECK_POINT_MAP
//         chunks.try_emplace(chunk_id, std::move(MemCheckPoint(from_addr, skip, count)));
//         #else
//         chunks.emplace_back(chunk_id, from_addr, skip, count);
//         #endif
//     }
//     #ifdef MEM_CHECK_POINT_MAP
//     void add_or_update(uint32_t chunk_id, uint32_t from_addr, uint32_t count) {
//         auto result = chunks.try_emplace(chunk_id, std::move(MemCheckPoint(from_addr, 0, count)));
//         if (!result.second) {
//             result.first->second.add_rows(from_addr, count);
//         }
//     }
//     #endif
//     void close() {
//         #ifdef MEM_SEGMENT_SORT
//         std::sort(chunks.begin(), chunks.end(), [](const MemCheckPoint &a, const MemCheckPoint &b) {
//             return a.from_addr < b.from_addr;
//         });
//         #endif
//     }
// };
#endif