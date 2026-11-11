#include <stdint.h>
#include "mem_check_point.hpp"

void MemCheckPoint::set(uint32_t chunk_id, uint64_t from_addr, uint32_t skip, uint32_t count) {
    set(chunk_id, from_addr, skip, from_addr, count, count);
}

void MemCheckPoint::set(uint32_t chunk_id, uint64_t from_addr, uint32_t skip, uint64_t to_addr, uint32_t to_count, uint32_t count) {
    this->chunk_id = chunk_id;
    this->from_addr = (uint32_t)(from_addr >> 3);
    this->from_skip = skip;
    this->to_addr = (uint32_t)(to_addr >> 3);
    this->to_count = to_count;
    this->count = count;
}

void MemCheckPoint::add_rows(uint64_t addr_bytes, uint32_t count) {
    const uint32_t addr = (uint32_t)(addr_bytes >> 3);
    this->count += count;
    if (addr == to_addr) {
        to_count += count;
    } else {
        to_addr = addr;
        to_count = count;
    }
}
