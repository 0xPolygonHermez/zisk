#ifndef __MEM_OFFSETS_HPP__
#define __MEM_OFFSETS_HPP__

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <string>
#include "mem_config.hpp"
#include "instance_meta.hpp"

class MemOffsets {
private:
    uint32_t id;
    uint32_t first_offset_addr;
    uint32_t last_offset_addr;
    uint32_t *page_offsets;
    uint32_t *page_values;
    uint32_t *offsets;
    uint32_t num_offset_pages;
    uint32_t num_offsets;
    uint32_t num_absent_pages;
    bool is_empty;
    bool is_preallocated;

public:
    MemOffsets(uint32_t id, uint32_t pages, uint32_t collapsible_pages);
    MemOffsets(uint32_t id);
    ~MemOffsets();
    
    void add_addr_offset(uint32_t addr, uint32_t offset_value);
    void save_offsets_to_file(const std::string &filename, bool compact = true);
    uint32_t get_first_offset_addr() const { return first_offset_addr; }
    uint32_t get_last_offset_addr() const { return last_offset_addr; }
    
    // Prevent copying
    MemOffsets(const MemOffsets&) = delete;
    MemOffsets& operator=(const MemOffsets&) = delete;
    inline void update(uint32_t addr, uint32_t offset) {
        if (addr > last_offset_addr) {
            add_addr_offset(addr, offset);
        }        
    }
    inline void reset() {
        first_offset_addr = 0;
        last_offset_addr = 0;
        num_absent_pages = 0;
        is_empty = true;
    }   
    void realloc_pages(uint32_t page = 0);
    void realloc_offsets(uint32_t min_size);
    void preallocate(uint32_t first_addr, uint32_t last_addr, uint32_t num_addrs);
    void move_to_paged_offsets(PagedOffsets &paged_offsets, uint32_t &offsets_base_addr);
    void allocate(uint32_t pages, uint32_t collapsible_pages);
    void fill_padding(void);
protected:
    void inner_allocate(uint32_t pages, uint32_t collapsible_pages);
};

#endif
