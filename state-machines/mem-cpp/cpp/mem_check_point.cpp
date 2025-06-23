#include <stdint.h>
#include "mem_check_point.hpp"

void MemCheckPoint::set(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count) {
    this->chunk_id = chunk_id;
    this->from_addr = from_addr;
    this->from_skip = skip;
    this->to_addr = from_addr;
    this->to_count = count;
    this->count = count;
}

void MemCheckPoint::add_rows(uint32_t addr, uint32_t count) {
    this->count += count;
    if (addr == to_addr) {
        to_count += count;
    } else {
        to_addr = addr;
        to_count = count;
    }
}
