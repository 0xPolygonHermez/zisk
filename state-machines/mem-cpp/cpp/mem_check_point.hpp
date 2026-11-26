#ifndef __MEM_CHECK_POINT_HPP__
#define __MEM_CHECK_POINT_HPP__
#include <stdint.h>
#include "mem_config.hpp"

// A checkpoint as shared with Rust (mem_checkpoints.rs::CppMemCheckPoint): the addresses are
// WORD addresses (byte address / 8), which keep the 33-bit address space inside a uint32_t.
struct MemCheckPoint {
    public:
        uint32_t chunk_id;
        uint32_t from_addr;   // word address
        uint32_t from_skip;
        uint32_t to_addr;     // word address
        uint32_t to_count;
        uint32_t count;
    public:
        // the setters take byte addresses
        void set(uint32_t chunk_id, uint64_t from_addr, uint32_t skip, uint32_t count);
        void set(uint32_t chunk_id, uint64_t from_addr, uint32_t skip, uint64_t to_addr, uint32_t to_count, uint32_t count);
        void add_rows(uint64_t addr, uint32_t count);
};
#endif