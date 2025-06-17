#ifndef __MEM_CHECK_POINT_HPP__
#define __MEM_CHECK_POINT_HPP__
#include <stdint.h>
#include "mem_config.hpp"
class MemCheckPoint {
    private:
        #ifndef MEM_CHECK_POINT_MAP
        uint32_t chunk_id;
        #endif
        uint32_t from_addr;
        uint32_t from_skip;
        uint32_t to_addr;
        uint32_t to_count;
        uint32_t count;
    public:
        #ifdef MEM_CHECK_POINT_MAP
        MemCheckPoint(uint32_t from_addr, uint32_t skip, uint32_t count) :
        #else
        MemCheckPoint(uint32_t chunk_id, uint32_t from_addr, uint32_t skip, uint32_t count) : chunk_id(chunk_id),
        #endif
            from_addr(from_addr),
            from_skip(skip),
            to_addr(from_addr),
            to_count(count),
            count(count) {
        }
        ~MemCheckPoint() {
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