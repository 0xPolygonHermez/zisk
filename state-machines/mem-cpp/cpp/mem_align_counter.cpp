#include "mem_align_counter.hpp"
#include "mem_config.hpp"
#include "mem_types.hpp"
#include "mem_context.hpp"
#include "tools.hpp"
#include <vector>
#include <assert.h>

MemAlignCounter::MemAlignCounter(std::shared_ptr<MemContext> context) :context(context) {
    total_counters.chunk_id = 0xFFFFFFFF;
    total_counters.full_5 = 0;
    total_counters.full_3 = 0;
    total_counters.full_2 = 0;
    total_counters.read_byte = 0;
    total_counters.write_byte = 0;
}

void MemAlignCounter::execute()
{
    uint64_t init = get_usec();
    const MemChunk *chunk;
    uint32_t chunk_id = 0;
    int64_t elapsed_us = 0;
    #ifdef MEM_CONTEXT_SEM
    while ((chunk = context->get_chunk(MAX_THREADS, chunk_id, elapsed_us)) != nullptr)
    #else
    while ((chunk = context->get_chunk(chunk_id, elapsed_us)) != nullptr) 
    #endif
    {
        execute_chunk(chunk_id, chunk->data, chunk->count);
        #ifdef COUNT_CHUNK_STATS
        #ifdef CHUNK_STATS
        total_usleep += elapsed_us > 0 ? elapsed_us : 0;
        #else
        total_usleep += elapsed_us;
        #endif
        #endif
        ++chunk_id;
    }
    elapsed_ms = ((get_usec() - init) / 1000);
}

void MemAlignCounter::execute_chunk(uint32_t chunk_id, const MemCountersBusData *chunk_data, uint32_t chunk_size) {
    uint32_t full_5 = 0;
    uint32_t full_3 = 0;
    uint32_t full_2 = 0;
    uint32_t read_byte = 0;
    uint32_t write_byte = 0;
    
    for (uint32_t i = 0; i < chunk_size; i++) {
        switch (chunk_data[i].flags & 0x3F) {
            // 1 byte read
            case MOPS_READ_1:
                read_byte += 1;
                break;        
            // 2 bytes read
            case MOPS_READ_2:
                if ((chunk_data[i].addr & 0x07) > 6) {
                    full_3 += 1;
                } else {
                    full_2 += 1;
                }
                break;
            // 4 bytes read
            case MOPS_READ_4: 
                if ((chunk_data[i].addr & 0x07) > 4) {
                    full_3 += 1;
                } else {
                    full_2 += 1;
                }
                break;
            // 8 bytes read
            case MOPS_READ_8: 
                if ((chunk_data[i].addr & 0x07) > 0) {
                    full_3 += 1;
                }
                // if chunk_data[i].addr & 0x07 == 0 ==> aligned read 
                break;
            // 1 byte write (clear)
            case MOPS_CWRITE_1:
                write_byte += 1;
                break;        
            // 1 byte write
            case MOPS_WRITE_1:
                full_3 += 1;
                break;
            // 2 bytes write
            case MOPS_WRITE_2:
                if ((chunk_data[i].addr & 0x07) > 6) {
                    full_5 += 1;
                } else {
                    full_3 += 1;
                }
                break;
            // 4 bytes write
            case MOPS_WRITE_4:
                if ((chunk_data[i].addr & 0x07) > 4) {
                    full_5 += 1;
                } else { 
                    full_3 += 1;
                }
                break;
            // 8 bytes write
            case MOPS_WRITE_8:
                if ((chunk_data[i].addr & 0x07) > 0) {
                    full_5 += 1;
                }
                // if chunk_data[i].addr & 0x07 == 0 ==> aligned write
                break;       
            case MOPS_BLOCK_READ + 0x00:
            case MOPS_BLOCK_READ + 0x10:
            case MOPS_BLOCK_READ + 0x20:
            case MOPS_BLOCK_READ + 0x30:
                if ((chunk_data[i].addr & 0x07) > 0) {
                    const uint32_t count = chunk_data[i].flags >> MOPS_BLOCK_COUNT_SBITS;
                    full_5 += count;
                }
                break;
            case MOPS_BLOCK_WRITE + 0x00:
            case MOPS_BLOCK_WRITE + 0x10:
            case MOPS_BLOCK_WRITE + 0x20:
            case MOPS_BLOCK_WRITE + 0x30:
                if ((chunk_data[i].addr & 0x07) > 0) {
                    const uint32_t count = chunk_data[i].flags >> MOPS_BLOCK_COUNT_SBITS;
                    full_5 += count;
                }
                break;

            case MOPS_ALIGNED_READ + 0x00:
            case MOPS_ALIGNED_READ + 0x10:
            case MOPS_ALIGNED_READ + 0x20:
            case MOPS_ALIGNED_READ + 0x30:
            case MOPS_ALIGNED_WRITE + 0x00:
            case MOPS_ALIGNED_WRITE + 0x10:
            case MOPS_ALIGNED_WRITE + 0x20:
            case MOPS_ALIGNED_WRITE + 0x30:
            case MOPS_ALIGNED_BLOCK_READ + 0x00:
            case MOPS_ALIGNED_BLOCK_READ + 0x10:
            case MOPS_ALIGNED_BLOCK_READ + 0x20:
            case MOPS_ALIGNED_BLOCK_READ + 0x30:
            case MOPS_ALIGNED_BLOCK_WRITE + 0x00:
            case MOPS_ALIGNED_BLOCK_WRITE + 0x10:
            case MOPS_ALIGNED_BLOCK_WRITE + 0x20:
            case MOPS_ALIGNED_BLOCK_WRITE + 0x30:
                break;
            default:
                printf("MemAlignCounter: Unknown flags: 0x%X\n", chunk_data[i].flags);
                assert(false && "Unknown flags in MemAlignCounter");
        }
    }
    total_counters.full_5 += full_5;
    total_counters.full_3 += full_3;
    total_counters.full_2 += full_2;
    total_counters.read_byte += read_byte;
    total_counters.write_byte += write_byte;
    uint32_t total_counters_processed = full_2 + full_3 + full_5 + read_byte + write_byte;
    if (total_counters_processed > 0) {
        counters.push_back({chunk_id, full_5, full_3, full_2, read_byte, write_byte});
    };
}

void MemAlignCounter::debug (void) {
    uint32_t index = 0;
    for (auto &count: counters) {
        printf("MEM_ALIGN_COUNTER #%d F5:%d F3:%d F2:%d RB:%d WB:%d\n", index++, count.full_5, count.full_3, count.full_2, count.read_byte, count.write_byte);
    }
}

