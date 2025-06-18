#ifndef __MEM_CHECK_POINT_HPP__
#define __MEM_CHECK_POINT_HPP__
#include <stdint.h>
#include "mem_config.hpp"

struct MemCheckPoint {
    public:
        uint32_t chunk_id;
        uint32_t from_addr;
        uint32_t from_skip;
        uint32_t to_addr;
        uint32_t to_count;
        uint32_t count;
    public:
        void set(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count) {
            this->chunk_id = chunk_id;
            this->from_addr = from_addr;
            this->from_skip = skip;
            this->to_addr = from_addr;
            this->to_count = count;
            this->count = count;
        }
        void add_rows(uint32_t addr, uint32_t count) {
            this->count += count;
            if (addr == to_addr) {
                to_count += count;
            } else {
                to_addr = addr;
                to_count = count;
            }
        }
};
#endif