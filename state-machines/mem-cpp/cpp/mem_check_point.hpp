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
        void set(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count);
        void add_rows(uint32_t addr, uint32_t count);
};
#endif