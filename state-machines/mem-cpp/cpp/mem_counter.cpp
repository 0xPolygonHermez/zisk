#include "mem_counter.hpp"
#include <cstdio>
#include <cassert>
#include <assert.h>
#include <string.h>
#include <ostream>

#define ST_INI 0
#define ST_READ 1
#define ST_WRITE 2
#define ST_INI_TO_READ (ST_READ << ST_BITS_OFFSET)
#define ST_INI_TO_WRITE (ST_WRITE << ST_BITS_OFFSET)
#define ST_READ_TO_WRITE ((ST_WRITE - ST_READ) << ST_BITS_OFFSET)
#define ST_X_TO_INI_MASK (0xFFFFFFFF >> (32 - ST_BITS_OFFSET))

#define ALIGN_MASK 0xFFFF'FFFF'FFFF'FFF8ULL 

MemCounter::MemCounter(uint32_t id, std::shared_ptr<MemContext> context)
:id(id), context(context), addr_mask(id * 8) {
    count = 0;
    queue_full = 0;
    first_chunk_us = 0;
    tot_wait_us = 0;
    addr_count_table = (AddrCount *)malloc(ADDR_TABLE_SIZE * sizeof(AddrCount));
    explicit_bzero(addr_count_table, ADDR_TABLE_SIZE * sizeof(AddrCount));

    // no memset because informations is overrided.
    addr_slots = (uint32_t *)std::aligned_alloc(64, ADDR_SLOTS_SIZE * sizeof(uint32_t));

    memset(first_offset, 0xFF, sizeof(first_offset));
    explicit_bzero(last_offset, sizeof(last_offset));

    free_slot = 0;
    addr_count = 0;
}

MemCounter::~MemCounter() {
    free(addr_count_table);
    free(addr_slots);
}

void MemCounter::execute() {
    uint64_t init_us = get_usec();
    
    int64_t elapsed_us = 0;

    const MemChunk *chunk =
#ifdef MEM_CONTEXT_SEM
        context->get_chunk(id, 0, elapsed_us);
#else
        context->get_chunk(0, elapsed_us);
#endif
    #ifdef COUNT_CHUNK_STATS
    wait_chunks_us[0] = elapsed_us;
    auto start_execute_us = get_usec();
    #endif
    if (chunk != nullptr) {
        execute_chunk(0, chunk->data, chunk->count);
        #ifdef COUNT_CHUNK_STATS
        chunks_us[0] = get_usec() - start_execute_us;
        tot_wait_us += elapsed_us > 0 ? elapsed_us : 0;
        #else
        tot_wait_us += elapsed_us;
        #endif
        first_chunk_us = get_usec() - init_us;

        uint32_t chunk_id = 1;
#ifdef MEM_CONTEXT_SEM
        while ((chunk = context->get_chunk(id, chunk_id, elapsed_us)) != nullptr)
#else
        while ((chunk = context->get_chunk(chunk_id, elapsed_us)) != nullptr)
#endif
        {
            #ifdef COUNT_CHUNK_STATS
            wait_chunks_us[chunk_id] = elapsed_us;
            auto start_execute_us = get_usec();
            #endif
            execute_chunk(chunk_id, chunk->data, chunk->count);
            #ifdef COUNT_CHUNK_STATS
            chunks_us[chunk_id] = get_usec() - start_execute_us;
            tot_wait_us += elapsed_us > 0 ? elapsed_us : 0;
            #else
            tot_wait_us += elapsed_us;
            #endif
            ++chunk_id;
        }
        #ifdef COUNT_CHUNK_STATS
        wait_chunks_us[chunk_id] = elapsed_us;
        #endif
    }
    elapsed_ms = ((get_usec() - init_us) / 1000);
}

void MemCounter::execute_chunk(uint32_t chunk_id, const MemCountersBusData *chunk_data, uint32_t chunk_size) {

#ifdef MEM_STATS_ACTIVE
    // Get start time for stats
    struct timespec start_time;
    clock_gettime(CLOCK_REALTIME, &start_time);
#endif // MEM_STATS_ACTIVE

    current_chunk = chunk_id;

    for (const MemCountersBusData *chunk_eod = chunk_data + chunk_size; chunk_eod != chunk_data; chunk_data++) {
        const uint8_t bytes = chunk_data->flags & 0x0F;
        const uint32_t addr = chunk_data->addr;
        switch (bytes) {
            // byte
            case 1:
                if ((addr & ADDR_MASK) != addr_mask) {
                    continue;
                }
                incr_counter(addr & ALIGN_MASK, chunk_id, false, chunk_data->flags & MOPS_WRITE_FLAG);
                break;

            // half word                
            case 2:
                if ((addr & ADDR_MASK) == addr_mask) {
                    incr_counter(addr & ALIGN_MASK, chunk_id, false, chunk_data->flags & MOPS_WRITE_FLAG);
                }
                else if (((addr + 1) & ADDR_MASK) == addr_mask) {
                    incr_counter((addr & ALIGN_MASK) + 8 , chunk_id, false, chunk_data->flags & MOPS_WRITE_FLAG);
                }
                break;

            // word                
            case 4:
                if ((addr & ADDR_MASK) == addr_mask) {
                    incr_counter(addr & ALIGN_MASK, chunk_id, false, chunk_data->flags & MOPS_WRITE_FLAG);
                }
                else if (((addr + 3) & ADDR_MASK) == addr_mask) {
                    incr_counter((addr & ALIGN_MASK) + 8, chunk_id, false, chunk_data->flags & MOPS_WRITE_FLAG);
                }
                break;

            // double word 
            case 8:
                if ((addr & 0x07) == 0) {
                    // aligned access
                    if ((addr & ADDR_MASK) != addr_mask) {
                        continue;
                    }
                    incr_counter(addr, chunk_id, true, chunk_data->flags & MOPS_WRITE_FLAG);
                } else {
                    const uint32_t aligned_addr = addr & ALIGN_MASK;

                    if ((aligned_addr & ADDR_MASK) == addr_mask) {
                        incr_counter(aligned_addr, chunk_id, false, chunk_data->flags & MOPS_WRITE_FLAG);
                    }
                    else if (((aligned_addr + 7) & ADDR_MASK) == addr_mask) {
                        incr_counter(aligned_addr + 8 , chunk_id, false, chunk_data->flags & MOPS_WRITE_FLAG);
                    }
                }
                break;
                
            case MOPS_ALIGNED_READ: {
                assert((addr & 0x07) == 0);
                if ((addr & ADDR_MASK) == addr_mask) {
                    incr_counter(addr , chunk_id, true, false);
                }
                break;
            }

            case MOPS_ALIGNED_WRITE: {
                assert((addr & 0x07) == 0);
                if ((addr & ADDR_MASK) == addr_mask) {
                    incr_counter(addr , chunk_id, true, true);
                }
                break;
            }

            case MOPS_BLOCK_READ: 
            case MOPS_BLOCK_WRITE: {
                bool write = bytes == MOPS_BLOCK_WRITE;
                const uint32_t count = chunk_data->flags >> MOPS_BLOCK_COUNT_SBITS;
                if ((addr & 0x07) == 0) {
                    uint32_t to_addr = addr + count * 8;
                    uint32_t c_addr = (addr & ~ADDR_MASK) + addr_mask;
                    if (c_addr < addr) {
                        c_addr += (MAX_THREADS * 8);
                    }
                    while (c_addr < to_addr) {                
                        incr_counter(c_addr , chunk_id, true, write);
                        c_addr += (MAX_THREADS * 8);
                    }
                } else {
                    // increase range, because if width = 8 and not aligned means
                    // each access is double, addr and addr + 8 
                    const uint32_t from_addr = (addr & ~0x07);
                    const uint32_t to_addr = from_addr + (count + 1) * 8;
                    uint32_t c_addr = (from_addr & ~ADDR_MASK) + addr_mask;
                    if (c_addr < from_addr) {
                        c_addr += (MAX_THREADS * 8);
                    }
                    while (c_addr < to_addr) {                
                        incr_counter(c_addr , chunk_id, false, write);
                        c_addr += (MAX_THREADS * 8);
                    }
                }
                break;
            }

            case MOPS_ALIGNED_BLOCK_READ:
            case MOPS_ALIGNED_BLOCK_WRITE: {
                assert((addr & 0x07) == 0);
                bool write = bytes == MOPS_ALIGNED_BLOCK_WRITE;
                uint32_t count = chunk_data->flags >> 4;
                uint32_t to_addr = addr + count * 8;
                uint32_t c_addr = (addr & ~ADDR_MASK) + addr_mask;
                if (c_addr < addr) {
                    c_addr += (MAX_THREADS * 8);
                }
                while (c_addr < to_addr) {
                    incr_counter(c_addr , chunk_id, true, write);
                    c_addr += (MAX_THREADS * 8);
                }
                break;
            }

            
            default:
                std::ostringstream msg;
                msg << "ERROR: MemCounter execute_chunk: invalid bytes size " << bytes << " at chunk_id " << chunk_id << " addr 0x" << std::hex << addr;
                throw std::runtime_error(msg.str());
        }
        // if (bytes == 8 && (addr & 0x07) == 0) {
        //     // aligned access
        //     if ((addr & ADDR_MASK) != addr_mask) {
        //         continue;
        //     }
        //     incr_counter(addr, chunk_id, true, chunk_data->flags & MEM_WRITE_FLAG);
        // } else {
        //     const uint32_t aligned_addr = addr & 0xFFFFFFF8;

        //     if ((aligned_addr & ADDR_MASK) == addr_mask) {
        //         incr_counter(aligned_addr, chunk_id, false, chunk_data->flags & MEM_WRITE_FLAG);
        //     }
        //     else if ((bytes + (addr & 0x07)) > 8 && ((aligned_addr + 8) & ADDR_MASK) == addr_mask) {
        //         incr_counter(aligned_addr + 8 , chunk_id, false, chunk_data->flags & MEM_WRITE_FLAG);
        //     }
        // }
    }

#ifdef MEM_STATS_ACTIVE
    // Add stats for this chunk execution
    struct timespec end_time;
    clock_gettime(CLOCK_REALTIME, &end_time);
    assert(mem_stats != nullptr);
    mem_stats->add_stat(
        MEM_STATS_EXECUTE_CHUNK_0 + ((id - MEM_STATS_EXECUTE_CHUNK_0) % std::min(8, MAX_THREADS)),
        start_time.tv_sec,
        start_time.tv_nsec, 
        (end_time.tv_sec - start_time.tv_sec) * 1000000000 + (end_time.tv_nsec - start_time.tv_nsec));
#endif // MEM_STATS_ACTIVE
}

void MemCounter::incr_counter(uint32_t addr, uint32_t chunk_id, bool is_aligned, bool is_write) {
    uint32_t offset = addr_to_offset(addr, current_chunk);
    uint32_t pos = addr_count_table[offset].pos;    
    bool is_ram = (addr >= RAM_ADDR);
    if (pos == 0) {
        // It's the first time for this address
        uint32_t pos = get_next_slot_pos();
        addr_slots[pos] = 0;
        addr_slots[pos + 1] = pos;
        addr_slots[pos + 2] = chunk_id;
        addr_slots[pos + 3] = init_addr_count(is_aligned, is_write, is_ram);
        assert(offset < ADDR_TABLE_SIZE);
        addr_count_table[offset].pos = pos + 2;        

        uint32_t page = offset >> ADDR_PAGE_BITS;
        first_offset[page] = std::min(first_offset[page], offset);
        last_offset[page] = std::max(last_offset[page], offset);
        ++addr_count;
    } else {
        // check if we need to increase the counter of current active chunk

        if (addr_slots[pos] == chunk_id) {
            update_addr_count(addr_slots[pos + 1], is_aligned, is_write, is_ram);
            return;
        }
        // update addr_count_table because only the last pos remaining non update
        // for this reason when calculate total take account the last position and
        // its state.
        addr_count_table[offset].count += get_pos_count(pos + 1);

        if ((pos % ADDR_SLOT_SIZE) == (ADDR_SLOT_SIZE - 2)) {

            uint32_t npos = get_next_slot_pos();
            uint32_t tpos = pos - ADDR_SLOT_SIZE + 2;
            addr_slots[npos] = tpos;
            addr_slots[npos + 1] = addr_slots[tpos + 1];
            addr_slots[npos + 2] = chunk_id;
            addr_slots[npos + 3] = init_addr_count(is_aligned, is_write, is_ram);
            addr_slots[tpos + 1] = npos;
            addr_count_table[offset].pos = npos + 2;
            return;
        }
        addr_slots[pos + 2] = chunk_id;
        addr_slots[pos + 3] = init_addr_count(is_aligned, is_write, is_ram);
        addr_count_table[offset].pos = pos + 2;
    }
}

void MemCounter::update_addr_count(uint32_t &count, bool is_aligned, bool is_write, bool is_ram) {
    if (!is_ram) {
        count += (is_aligned || !is_write) ? 1 : 2;
    } else if (is_aligned) {
        count = incr_st_counter_aligned(count, is_write);
    } else {
        count = incr_st_counter_unaligned(count, is_write);
    }
}

uint32_t MemCounter::init_addr_count(bool is_aligned, bool is_write, bool is_ram) {
    if (!is_ram) {
        return (is_aligned || !is_write) ? 1 : 2;
    } else if (is_aligned) {
        return is_write ? ST_INI_TO_WRITE : ST_INI_TO_READ;
    }
    return is_write ? 1 + ST_INI_TO_WRITE : ST_INI_TO_READ;
}


uint32_t MemCounter::incr_st_counter_aligned(uint32_t count, bool is_write) {
    switch ((uint8_t)(count >> ST_BITS_OFFSET)) {
        case ST_INI:
            if (is_write) {
                // this write could be compacted on dual operation write-read
                // don't increase the count, just change the state
                return count + ST_INI_TO_WRITE;
            }
            // this read could be compacted on dual operation read-read
            // don't increase the count, just change the state
            return count + ST_INI_TO_READ;
        case ST_READ:
            if (is_write) {
                // this write means that the previous read cannot be compacted, increase
                // the counter by this previous read and change state to write with
                // hope that this write could be compacted on dual operation write-read
                return (count & ST_X_TO_INI_MASK) + 1 + ST_INI_TO_WRITE;
            }
            // this read was compacted on dual operation read-read
            // increase the count for this dual operation and reset the state
            return (count & ST_X_TO_INI_MASK) + 1;
        case ST_WRITE:
            if (is_write) {
                // this write means that the previous write cannot be compacted, increase
                // the counter by this previous write and continue in same state
                // hope that this write could be compacted on dual operation write-read
                return count + 1;
            }
            // this read was compacted on dual operation read-read
            // increase the count for this dual operation and reset the state
            return (count & ST_X_TO_INI_MASK) + 1;
        default: 
            assert(false && "Invalid count state");
    }
}
uint32_t MemCounter::incr_st_counter_unaligned(uint32_t count, bool is_write) {
    // in this case the operation are:
    //  - is_write = false => READ
    //  - is_write = true  => READ + WRITE

    switch ((uint8_t)(count >> ST_BITS_OFFSET)) {
        case ST_INI:
            if (is_write) {
                // [read + write], the first read operation cannot be compacted, increase the
                // counter by first read, and change state to write by second write.
                return count + 1 + ST_INI_TO_WRITE;
            } 
            // this read could be compacted on dual operation read-read
            // don't increase the count, just change the state
            return count + ST_INI_TO_READ;
        case ST_READ:
            if (is_write) {
                // [read + write], means that the previous read could be compacted, increase
                // the counter by this read-read operation, change state to write by second write.
                return count + 1 + ST_READ_TO_WRITE;
            }
            // this read was compacted on dual operation read-read
            // increase the count for this dual operation and reset the state
            return (count & ST_X_TO_INI_MASK) + 1;
        case ST_WRITE:
            if (is_write) {
                // [read + write], this write means that the previous write cannot be compacted,
                // increase the counter by this previous write and continue in same state
                // hope that this write could be compacted on dual operation write-read
                return count + 1;
            }
            // this read was compacted on dual operation read-read
            // increase the count for this dual operation and reset the state
            return (count & ST_X_TO_INI_MASK) + 1;
        default: 
            assert(false && "Invalid count state");        
    }
}

void MemCounter::stats() {
    #ifdef COUNT_CHUNK_STATS
    uint32_t chunks_count = context->size();
    if (chunks_count > 0) {
        printf("counter[%d].chunk_us: %ld", id, chunks_us[0]);
        for (size_t j = 1; j < chunks_count; ++j) {
            printf(";%ld", chunks_us[j]);
        }
        printf("\ncounter[%d].wait_chunks_us: %ld", id, wait_chunks_us[0]);
        for (size_t j = 1; j < chunks_count; ++j) {
            printf(";%ld", wait_chunks_us[j]);
        }
        printf("\n");
    }
    #endif
}