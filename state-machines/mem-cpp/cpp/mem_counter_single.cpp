#include "mem_counter_single.hpp"
#include <cstdio>
#include <cassert>
#include <assert.h>
#include <string.h>
#include <ostream>

#define ALIGN_MASK 0xFFFF'FFFF'FFFF'FFF8ULL 

MemCounterSingle::MemCounterSingle(void){
    free_read_available = (bool *)malloc(TABLE_OFFSET_SIZE * sizeof(bool));
    clear();
}

MemCounterSingle::~MemCounterSingle() {
    free(free_read_available);
}
void MemCounterSingle::clear(void) {
    explicit_bzero(free_read_available, TABLE_OFFSET_SIZE * sizeof(bool));
    full_5 = 0;
    full_3 = 0;
    full_2 = 0;
    read_byte = 0;
    write_byte = 0;
    out.clear();        
}
void MemCounterSingle::execute(const MemCountersBusData *chunk_data, uint32_t chunk_size) {
    clear();
    for (const MemCountersBusData *chunk_eod = chunk_data + chunk_size; chunk_eod != chunk_data; chunk_data++) {
        const uint8_t bytes = chunk_data->flags & 0x0F;
        const uint32_t addr = chunk_data->addr;
        const bool write_flag = (chunk_data->flags & MOPS_WRITE_FLAG) != 0;
        const uint32_t aligned_addr = addr & ALIGN_MASK;
        const uint8_t mode = chunk_data->flags & 0x3F;
        switch (mode) {
            // 1 byte
            case MOPS_READ_1:
                read_byte += 1;
                add_aligned_read(aligned_addr);
                break;
            case MOPS_CWRITE_1:
                write_byte += 1;
                add_aligned_read_write(aligned_addr);
                break;
            case MOPS_WRITE_1:
                full_3 += 1;
                add_aligned_read_write(aligned_addr);
                break;                

            // 2 bytes
            case MOPS_READ_2: {
                add_aligned_read(aligned_addr);
                if ((addr & 0x07) > 6) {
                    add_aligned_read(aligned_addr + 8);
                    full_3 += 1;
                } else {
                    full_2 += 1;
                }
                break;
            }
            case MOPS_WRITE_2: {
                add_aligned_read_write(aligned_addr);
                if ((addr & 0x07) > 6) {
                    add_aligned_read_write(aligned_addr + 8);
                    full_5 += 1;
                } else {
                    full_3 += 1;
                }
                break;
            }

            // 4 bytes
            case MOPS_READ_4: {
                add_aligned_read(aligned_addr);
                if ((addr & 0x07) > 4) {
                    full_3 += 1;
                    add_aligned_read(aligned_addr + 8);
                } else {
                    full_2 += 1;
                }
                break;
            }
            case MOPS_WRITE_4: {
                add_aligned_read_write(aligned_addr);
                if ((addr & 0x07) > 4) {
                    full_5 += 1;
                    add_aligned_read_write(aligned_addr + 8);
                } else {
                    full_3 += 1;
                }
                break;
            }
            // 8 bytes 
            case MOPS_READ_8:
                add_aligned_read(aligned_addr);
                if ((addr & 0x07) > 0) {
                    full_3 += 1;
                    add_aligned_read(aligned_addr + 8);
                }
                break;
            case MOPS_WRITE_8:
                if (addr == aligned_addr) {
                    add_aligned_write(aligned_addr);
                } else {
                    full_5 += 1;
                    add_aligned_read_write(aligned_addr);
                    add_aligned_read_write(aligned_addr + 8);
                }
                break;   
            case MOPS_ALIGNED_READ: {
                add_aligned_read(addr);
                break;
            }
            case MOPS_ALIGNED_WRITE: {
                add_aligned_write(addr);
                break;
            }

            case MOPS_BLOCK_READ + 0x00:
            case MOPS_BLOCK_READ + 0x10:
            case MOPS_BLOCK_READ + 0x20:
            case MOPS_BLOCK_READ + 0x30:
            case MOPS_ALIGNED_BLOCK_READ + 0x00:
            case MOPS_ALIGNED_BLOCK_READ + 0x10:
            case MOPS_ALIGNED_BLOCK_READ + 0x20:
            case MOPS_ALIGNED_BLOCK_READ + 0x30: {
                const uint32_t count = chunk_data->flags >> MOPS_BLOCK_COUNT_SBITS;
                for (uint32_t i = 0; i < count; i++) {
                    add_aligned_read(addr + i * 8);
                }
                break;
            }
            case MOPS_ALIGNED_BLOCK_WRITE + 0x00:
            case MOPS_ALIGNED_BLOCK_WRITE + 0x10:
            case MOPS_ALIGNED_BLOCK_WRITE + 0x20:
            case MOPS_ALIGNED_BLOCK_WRITE + 0x30:
            case MOPS_BLOCK_WRITE + 0x00:
            case MOPS_BLOCK_WRITE + 0x10:
            case MOPS_BLOCK_WRITE + 0x20:
            case MOPS_BLOCK_WRITE + 0x30: {
                const uint32_t count = chunk_data->flags >> MOPS_BLOCK_COUNT_SBITS;
                for (uint32_t i = 0; i < count; i++) {
                    add_aligned_write(addr + i * 8);
                }
                break;
            }            
            default: {
                std::ostringstream msg;
                msg << "ERROR invalid bytes size " << bytes << " addr 0x" << std::hex << addr;
                throw std::runtime_error(msg.str());
            }
        }
    }
}

void MemCounterSingle::add_aligned_read_write(uint32_t addr) {
    bool is_ram = (addr >= RAM_ADDR);
    if (is_ram) {
        uint32_t offset = addr_to_offset(addr);    
        if (!free_read_available[offset]) {
            out.push_back(addr); // read
        }
        out.push_back(addr); // write
        free_read_available[offset] = true;
        return;
    } else {
        out.push_back(addr);
        out.push_back(addr);
    }
}


void MemCounterSingle::add_aligned_read(uint32_t addr) {
    bool is_ram = (addr >= RAM_ADDR);
    if (is_ram) {
        uint32_t offset = addr_to_offset(addr);    
        if (free_read_available[offset]) {
            free_read_available[offset] = false;
        } else {
            free_read_available[offset] = true;
            out.push_back(addr);    
        }
    } else {
        out.push_back(addr);
    }
}

void MemCounterSingle::add_aligned_write(uint32_t addr) {
    bool is_ram = (addr >= RAM_ADDR);
    if (is_ram) {
        uint32_t offset = addr_to_offset(addr);        
        free_read_available[offset] = true;
    }
    out.push_back(addr);
}
