#include "mem_segment_hash_table.hpp"

MemSegmentHashTable::MemSegmentHashTable(uint32_t key_size) : hash_count(key_size) {
    hash_bits = get_hash_bits(key_size);
    hash_mask = (1 << hash_bits) - 1;
    hash_table = (uint32_t *)malloc(key_size * sizeof(uint32_t));
    full_reset();
}
MemSegmentHashTable::~MemSegmentHashTable() {
    if (hash_table) {
        free(hash_table);
        hash_table = nullptr;
    }
}

