#include "mem_offsets.hpp"
#include <assert.h>
#include <ostream>
#include <algorithm>

MemOffsets::MemOffsets(uint32_t id, uint32_t mb_size)
    : id(id), first_offset_addr(0), last_offset_addr(0), is_preallocated(true),
      debug_mode(false), debug_absent_total(0) {
    
    num_offset_pages = MEM_OFFSETS_PAGES;
    num_offsets = MEM_OFFSETS;
    // number of pages was determined based on total address
    page_offsets = (uint32_t *)calloc(num_offset_pages, sizeof(uint32_t));
    page_values  = (uint32_t *)calloc(num_offset_pages, sizeof(uint32_t));
    offsets      = (uint32_t *)calloc(num_offsets, sizeof(uint32_t));
}

MemOffsets::MemOffsets(uint32_t id, uint32_t pages, uint32_t collapsible_pages)
    : id(id), first_offset_addr(0), last_offset_addr(0), is_preallocated(false),
      debug_mode(false), debug_absent_total(0) {
    inner_allocate(pages, collapsible_pages);
}

MemOffsets::MemOffsets(uint32_t id):id(id), first_offset_addr(0), last_offset_addr(0),
      is_preallocated(false), debug_mode(false), debug_absent_total(0) {
    page_offsets = nullptr;
    page_values  = nullptr;
    offsets      = nullptr;
    num_offset_pages = 0;
    num_offsets = 0;
}

MemOffsets::~MemOffsets() {
    // Safe to call free(nullptr), but check for clarity
    // After move_to_paged_offsets, these will be nullptr
    if (page_offsets) {
        free(page_offsets);
    }
    if (page_values) {
        free(page_values);
    }
    if (offsets) {
        free(offsets);
    }
}

void MemOffsets::realloc_pages(uint32_t page) {
    #ifdef MEM_WARNINGS
    uint32_t _num_offset_pages = num_offset_pages;
    #endif
    num_offset_pages = std::max(num_offset_pages, page + 1) + std::min(num_offset_pages, (uint32_t) MAX_OFFSET_PAGES_INCREMENT);
    #ifdef MEM_WARNINGS
    printf("MemOffsets::realloc_pages: reallocating from %u to %u pages (requested page %u)\n", _num_offset_pages, num_offset_pages, page);
    #endif
    page_values = (uint32_t *)realloc(page_values, num_offset_pages * sizeof(uint32_t));
    page_offsets = (uint32_t *)realloc(page_offsets, num_offset_pages * sizeof(uint32_t));
}

void MemOffsets::allocate(uint32_t pages, uint32_t collapsible_pages) {
    assert(page_values == nullptr && page_offsets == nullptr && offsets == nullptr);
    inner_allocate(pages, collapsible_pages);
}
void MemOffsets::inner_allocate(uint32_t pages, uint32_t collapsible_pages) {
    first_offset_addr = 0;
    last_offset_addr = 0;
    debug_absent_total = 0;
    num_offset_pages = pages;
    num_offsets = (pages - collapsible_pages) * OFFSET_PAGE_SIZE; // worst case: all pages are non-collapsible
    page_values = (uint32_t *)calloc(pages, sizeof(uint32_t));
    page_offsets = (uint32_t *)calloc(pages, sizeof(uint32_t));
    offsets = (uint32_t *)calloc(num_offsets, sizeof(uint32_t));
}
#define MIN_OFFSET_PAGES_INCREMENT 16

void MemOffsets::preallocate(uint32_t first_addr, uint32_t last_addr, uint32_t num_addrs) {
    is_preallocated = true;
    if (first_addr == 0 || last_addr == 0 || num_addrs == 0) {
        return;
    }
    
    // Calculate required pages based on address range
    uint32_t addr_range = (last_addr - first_addr) / 8;
    uint32_t required_offset_pages = (addr_range / OFFSET_PAGE_SIZE) + 1;
    
    // Pre-allocate page arrays if needed
    if (required_offset_pages > num_offset_pages) {
        realloc_pages(required_offset_pages - 1);
    }
    
    // Estimate required offsets:
    // Worst case: all addresses are in different pages = num_addrs * OFFSET_PAGE_SIZE
    // But realistically, they're more spread out, so use a heuristic
    uint32_t estimated_offsets = std::min(
        num_addrs * 2,  // Conservative estimate
        required_offset_pages * OFFSET_PAGE_SIZE
    );
    
    // Pre-allocate offsets array with some margin
    if (estimated_offsets > num_offsets) {
        uint32_t target_size = estimated_offsets + (MIN_OFFSET_PAGES_INCREMENT * OFFSET_PAGE_SIZE);
        num_offsets = std::min(target_size, (uint32_t)(MAX_OFFSET_INCREMENT * 2));
        offsets = (uint32_t *)realloc(offsets, num_offsets * sizeof(uint32_t));
        #ifdef MEM_WARNINGS
        printf("MemOffsets::preallocate: allocated %u offsets (estimated %u from %u addrs)\n",
               num_offsets, estimated_offsets, num_addrs);
        #endif
    }
}

void MemOffsets::realloc_offsets(uint32_t min_size) {
    #ifdef MEM_WARNINGS
    uint32_t _num_offsets = num_offsets;
    #endif
    
    // Calculate how many offsets we need at minimum
    uint32_t needed_increment = min_size - num_offsets + 1;
    uint32_t min_increment = MIN_OFFSET_PAGES_INCREMENT * OFFSET_PAGE_SIZE;
    
    // Use at least MIN_OFFSET_PAGES_INCREMENT pages or what's needed, whichever is larger
    // But cap at MAX_OFFSET_INCREMENT
    uint32_t increment = std::min((uint32_t)MAX_OFFSET_INCREMENT, 
                                   std::max(needed_increment, min_increment));
    num_offsets += increment;
                           
    #ifdef MEM_WARNINGS
    printf("MemOffsets::realloc_offsets: reallocating from %u to %u offsets (min_size=%u)\n", 
           _num_offsets, num_offsets, min_size);
    #endif
    offsets = (uint32_t *)realloc(offsets, num_offsets * sizeof(uint32_t));
}

void MemOffsets::add_addr_offset(uint32_t addr, uint32_t offset_value) {
    if (first_offset_addr == 0) {
        // First call: addr is the origin of the table, always at index 0
        first_offset_addr = addr;
        page_offsets[0] = 0;
        offsets[0] = offset_value;
        last_offset_addr = addr;
        return;
    }

    uint32_t index = ((addr - first_offset_addr) / 8);
    if (addr == last_offset_addr + 8) {
        // FAST PATH
        uint32_t page = index / OFFSET_PAGE_SIZE;
        if (is_preallocated && page >= num_offset_pages) {
            // only one page of increment
            realloc_pages(page);
        }
        if ((index % OFFSET_PAGE_SIZE) == 0) {  
            uint32_t page_index = page == 0 ? 0 : page_offsets[page - 1] + 1;
            uint32_t offsets_index = page_index * OFFSET_PAGE_SIZE;
            // new page        
            assert(page < num_offset_pages);    
            page_offsets[page] = page_index;
            if (is_preallocated) {
                if (offsets_index >= num_offsets) {
                    realloc_offsets(offsets_index);
                }
            } else if (offsets_index >= num_offsets) {
                assert(offsets_index < num_offsets);    
                return;
            }
            offsets[offsets_index] = offset_value;
        } else {
            uint32_t offsets_index = page_offsets[page] * OFFSET_PAGE_SIZE + (index % OFFSET_PAGE_SIZE);
            if (is_preallocated) {
                if (offsets_index >= num_offsets) {
                    realloc_offsets(offsets_index);
                }
            } else if (offsets_index >= num_offsets) {
                assert(offsets_index < num_offsets);    
            }
            offsets[offsets_index] = offset_value;
        }           
        last_offset_addr = addr;             
        return;
    }


    // GENERIC PATH
    uint32_t last_page = index / OFFSET_PAGE_SIZE;
    uint32_t in_page_index = index % OFFSET_PAGE_SIZE;
    uint32_t last_offset_index = ((last_offset_addr - first_offset_addr) / 8);
    uint32_t last_offset_page  = last_offset_index / OFFSET_PAGE_SIZE;

    if (is_preallocated) {
        if (last_page >= num_offset_pages) {
            realloc_pages(last_page);
        }
        
        // Ensure we have enough space in offsets array for the worst case:
        // - If same page: need space up to base + in_page_index
        // - If different page: need space up to page_offsets[last_offset_page] + OFFSET_PAGE_SIZE + in_page_index
        uint32_t max_offset_needed = page_offsets[last_offset_page] * OFFSET_PAGE_SIZE + OFFSET_PAGE_SIZE + in_page_index;
        if (max_offset_needed >= num_offsets) {
            realloc_offsets(max_offset_needed);
        }
    }
    if (last_offset_page == last_page) {
        // Same page: fill gap entries with offset_value, then write offset_value at target
        uint32_t base = page_offsets[last_offset_page] * OFFSET_PAGE_SIZE;
        uint32_t from_pos = (last_offset_index % OFFSET_PAGE_SIZE) + 1;
        for (uint32_t i = from_pos; i < in_page_index; ++i) {
            offsets[base + i] = offset_value;
        }
        offsets[base + in_page_index] = offset_value;
    } else {
        // 1. Fill the remainder of the last written page with offset_value
        {
            uint32_t base = page_offsets[last_offset_page] * OFFSET_PAGE_SIZE;
            uint32_t from_pos = (last_offset_index % OFFSET_PAGE_SIZE) + 1;
            for (uint32_t i = from_pos; i < OFFSET_PAGE_SIZE; ++i) {
                offsets[base + i] = offset_value;
            }
        }
        // 2. Compress entirely skipped pages.
        // Also compress last_page itself when addr is its last slot (all slots = offset_value).
        bool compress_last_page = (in_page_index == OFFSET_PAGE_SIZE - 1);
        uint32_t compress_end = compress_last_page ? last_page + 1 : last_page;
        for (uint32_t p = last_offset_page + 1; p < compress_end; ++p) {
            page_offsets[p] = MEM_OFFSETS_PAGE_ABSENT;
            page_values[p]  = offset_value;
        }
        if (debug_mode && compress_end > last_offset_page + 1) {
            uint32_t first_absent  = last_offset_page + 1;
            uint32_t last_absent   = compress_end - 1;
            uint32_t absent_count  = compress_end - last_offset_page - 1;
            uint32_t addr_start    = first_offset_addr + first_absent * OFFSET_PAGE_SIZE * 8;
            uint32_t addr_end      = first_offset_addr + compress_end * OFFSET_PAGE_SIZE * 8 - 8;
            debug_absent_total    += absent_count;
        }
        // 3. Allocate current page right after the last allocated one
        // (skipped when last_page was already compressed above)
        if (!compress_last_page) {
            uint32_t new_offsets_index = page_offsets[last_offset_page] * OFFSET_PAGE_SIZE + OFFSET_PAGE_SIZE;
            page_offsets[last_page] = new_offsets_index / OFFSET_PAGE_SIZE;
            for (uint32_t i = 0; i < in_page_index; ++i) {
                offsets[new_offsets_index + i] = offset_value;
            }
            offsets[new_offsets_index + in_page_index] = offset_value;
        }
    }
    last_offset_addr = addr;
}

void MemOffsets::save_offsets_to_file(const std::string &filename, bool compact) {
    FILE *file = fopen(filename.c_str(), "w");
    if (file == nullptr) {
        fprintf(stderr, "Error: Unable to open file %s for writing\n", filename.c_str());
        return;
    }

    if (first_offset_addr == 0) {
        fprintf(file, "# No offsets recorded yet\n");
        fclose(file);
        return;
    }

    uint32_t count = 0;
    uint32_t last_index = ((last_offset_addr - first_offset_addr) / 8);
    uint32_t last_page = last_index / OFFSET_PAGE_SIZE;

    // Iterate through all pages up to the last page with data
    uint32_t last_offset = 0xFFFFFFFE;
    for (uint32_t page = 0; page <= last_page; ++page) {
        if (page_offsets[page] == MEM_OFFSETS_PAGE_ABSENT) {
            // This is a compressed page - all entries have the same value
            uint32_t value = page_values[page];
            if (compact) {
                uint32_t next_value = (page < last_page) ? (page_offsets[page + 1] == MEM_OFFSETS_PAGE_ABSENT ? page_values[page + 1] : offsets[page_offsets[page + 1] * OFFSET_PAGE_SIZE]) : 0;
                if (value != next_value) {
                    uint32_t addr = (first_offset_addr >> 3) + (page + 1) * OFFSET_PAGE_SIZE - 1;
                    fprintf(file, "0x%X %d\n", addr * 8, value - 1);
                    ++count;
                    last_offset = value;
                }
            } else {
                uint32_t start_addr = first_offset_addr + (page * OFFSET_PAGE_SIZE * 8);
                uint32_t entries_in_page = (page == last_page) ? ((last_index % OFFSET_PAGE_SIZE) + 1) : OFFSET_PAGE_SIZE;
                
                for (uint32_t i = 0; i < entries_in_page; ++i) {
                    uint32_t addr = start_addr + (i * 8);
                    fprintf(file, "0x%X %u\n", addr, value);
                    ++count;
                }
            }
        } else {
            // Regular page with individual offset values
            uint32_t base = page_offsets[page] * OFFSET_PAGE_SIZE;
            uint32_t entries_in_page = (page == last_page) ? ((last_index % OFFSET_PAGE_SIZE) + 1) : OFFSET_PAGE_SIZE;
            
            for (uint32_t i = 0; i < entries_in_page; ++i) {
                uint32_t value = offsets[base + i]; 
                if (!compact || value != last_offset) {
                    uint32_t addr = (first_offset_addr >> 3) + (page * OFFSET_PAGE_SIZE + i);
                    fprintf(file, "0x%X %d\n", addr * 8, value - 1);
                    ++count;
                }
                last_offset = value;
            }
        }
    }

    fclose(file);
    printf("MemOffsets::save_offsets_to_file: Saved %u offsets to %s\n", count, filename.c_str());
}

void MemOffsets::move_to_paged_offsets(PagedOffsets &paged_offsets, uint32_t &offsets_base_addr) {
    // Move (transfer ownership) internal pointers to PagedOffsets structure
    // PagedOffsets uses different naming:
    //   page_starts = page_offsets
    //   page_single_value = page_values  
    //   pages_dense = offsets
    
    // Calculate the actual used range
    uint32_t last_index = first_offset_addr == 0 ? 0 : ((last_offset_addr - first_offset_addr) / 8);
    uint32_t last_page = last_index / OFFSET_PAGE_SIZE;
    
    // Count present (non-compressed) pages
    uint32_t present_count = 0;
    for (uint32_t p = 0; p <= last_page; ++p) {
        if (page_offsets[p] != MEM_OFFSETS_PAGE_ABSENT) {
            ++present_count;
        }
    }
    
    // Transfer ownership of the pointers
    paged_offsets.page_starts = page_offsets;
    paged_offsets.page_single_value = page_values;
    paged_offsets.pages_dense = offsets;
    #ifdef MEM_WARNINGS
    if (last_page >= num_offset_pages) {
        printf("\x1B[1;31mWARNING!! MemOffsets::move_to_paged_offsets: WARNING - last_page %u exceeds allocated num_offset_pages %u [0x%08X..=0x%08X]\x1B[0m\n", last_page, num_offset_pages,first_offset_addr, last_offset_addr);
    }
    #endif
    paged_offsets.num_pages = last_page + 1;
    paged_offsets.present_count = present_count;
    paged_offsets.addr_range_slots = last_index + 1;
    
    offsets_base_addr = first_offset_addr;
    
    // Clear internal pointers (ownership transferred)
    page_offsets = nullptr;
    page_values = nullptr;
    offsets = nullptr;
    num_offset_pages = 0;
    num_offsets = 0;
    first_offset_addr = 0;
    last_offset_addr = 0;
    
    #ifdef MEM_WARNINGS
    printf("MemOffsets::move_to_paged_offsets: Transferred %u pages (%u present, %u slots) from addr 0x%X\n",
           paged_offsets.num_pages, paged_offsets.present_count, 
           paged_offsets.addr_range_slots, offsets_base_addr);
    #endif
}
