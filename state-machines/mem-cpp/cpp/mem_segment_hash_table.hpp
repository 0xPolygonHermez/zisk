#ifndef __MEM_SEGMENT_HASH_TABLE_HPP__
#define __MEM_SEGMENT_HASH_TABLE_HPP__
#include <string.h>
#include <map>
#include <vector>
#include <unordered_map>
#include <stdexcept>
#include "mem_config.hpp"
#include "mem_check_point.hpp"
#include <assert.h>

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
    MemSegmentHashTable(uint32_t key_size);
    ~MemSegmentHashTable();
    inline uint32_t get_new_hash_id();
    inline void set(uint32_t key, uint32_t pos);
    inline uint32_t get(uint32_t key);
    inline void full_reset();
    inline void fast_reset();
    inline void debug();
};

uint32_t MemSegmentHashTable::get_new_hash_id() {
    if (hash_id >= hash_count) {
        throw std::runtime_error("Error: MemSegmentHashTable::get_new_hash_id: hash_id out of bounds");
    }
    return hash_id++;
}
void MemSegmentHashTable::set(uint32_t key, uint32_t pos) {
    assert(key < hash_count);
    hash_table[key] = hash_id | pos;
}
uint32_t MemSegmentHashTable::get(uint32_t key) {
    uint32_t value = hash_table[key];
    if (value < hash_id) {
        return MEM_SEGMENT_HASH_TABLE_KEY_NOT_FOUND;
    }
    uint32_t result = value & hash_mask;
    return result;
}
void MemSegmentHashTable::full_reset() {
    hash_id = 1 << hash_bits;
    memset(hash_table, 0, hash_count * sizeof(uint32_t));
}
void MemSegmentHashTable::fast_reset() {
    hash_id = hash_id + (1 << hash_bits);
    if (hash_id == 0) {
        full_reset();
    }
}

void MemSegmentHashTable::debug() {
    printf("MEM_SEGMENT_HASH_TABLE DEBUG: %p (bits:%d count:%d this:%p)\n", hash_table, hash_bits, hash_count, this);
}

#endif
