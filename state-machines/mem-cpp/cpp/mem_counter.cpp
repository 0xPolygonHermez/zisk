#include "mem_counter.hpp"
#include <assert.h>
#include <string.h>

MemCounter::MemCounter(uint32_t id, std::shared_ptr<MemContext> context)
:id(id), context(context), addr_mask(id * 8) {
    count = 0;
    queue_full = 0;
    first_chunk_us = 0;
    tot_wait_us = 0;
    #ifdef USE_ADDR_COUNT_TABLE
    addr_count_table = (AddrCount *)malloc(ADDR_TABLE_SIZE * sizeof(AddrCount));
    explicit_bzero(addr_count_table, ADDR_TABLE_SIZE * sizeof(AddrCount));
    // memset(addr_count_table, 0, ADDR_TABLE_SIZE * sizeof(AddrCount));
    #else
    addr_table = (uint32_t *)malloc(ADDR_TABLE_SIZE * sizeof(uint32_t));
    memset(addr_table, 0, ADDR_TABLE_SIZE * sizeof(uint32_t));
    #endif


    // no memset because informations is overrided.
    addr_slots = (uint32_t *)std::aligned_alloc(64, ADDR_SLOTS_SIZE * sizeof(uint32_t));

    memset(first_offset, 0xFF, sizeof(first_offset));
    explicit_bzero(last_offset, sizeof(last_offset));

    free_slot = 0;
    addr_count = 0;
}

MemCounter::~MemCounter() {
    #ifdef USE_ADDR_COUNT_TABLE
    free(addr_count_table);
    #else
    free(addr_table);
    #endif
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
        while ((chunk = context->get_chunk(id, chunk_id, elapsed_us)) != nullptr) {
#else
        while ((chunk = context->get_chunk(chunk_id, elapsed_us)) != nullptr) {
#endif
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
        const uint8_t bytes = chunk_data->flags & 0xFF;
        const uint32_t addr = chunk_data->addr;
        switch (bytes) {
            case 1: // byte
            case 2: // half word
            case 4: // word
            case 8: // double word
                break;
            default:
                std::ostringstream msg;
                msg << "ERROR: MemCounter execute_chunk: invalid bytes size " << bytes << " at chunk_id " << chunk_id << " addr 0x" << std::hex << addr;
                throw std::runtime_error(msg.str());
        }
        if (bytes == 8 && (addr & 0x07) == 0) {
            // aligned access
            if ((addr & ADDR_MASK) != addr_mask) {
                continue;
            }
            count_aligned(addr, chunk_id, 1);
        } else {
            const uint32_t aligned_addr = addr & 0xFFFFFFF8;

            if ((aligned_addr & ADDR_MASK) == addr_mask) {
                const int ops = 1 + (chunk_data->flags >> 16);
                count_aligned(aligned_addr, chunk_id, ops);
            }
            else if ((bytes + (addr & 0x07)) > 8 && ((aligned_addr + 8) & ADDR_MASK) == addr_mask) {
                const int ops = 1 + (chunk_data->flags >> 16);
                count_aligned(aligned_addr + 8 , chunk_id, ops);
            }
        }
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

void MemCounter::count_aligned(uint32_t addr, uint32_t chunk_id, uint32_t count) {
    uint32_t offset = addr_to_offset(addr, current_chunk);
    #ifdef USE_ADDR_COUNT_TABLE
    uint32_t pos = addr_count_table[offset].pos;
    #else
    uint32_t pos = addr_table[offset];
    #endif
    if (pos == 0) {
        uint32_t pos = get_next_slot_pos();
        addr_slots[pos] = 0;
        addr_slots[pos + 1] = pos;
        addr_slots[pos + 2] = chunk_id;
        addr_slots[pos + 3] = count;
        #ifdef USE_ADDR_COUNT_TABLE
        assert(offset < ADDR_TABLE_SIZE);
        addr_count_table[offset].pos = pos + 2;
        addr_count_table[offset].count = count;
        #else
        addr_table[offset] = pos + 2;
        #endif
        uint32_t page = offset >> ADDR_PAGE_BITS;
        first_offset[page] = std::min(first_offset[page], offset);
        last_offset[page] = std::max(last_offset[page], offset);
        ++addr_count;
    } else {
        #ifdef USE_ADDR_COUNT_TABLE
        addr_count_table[offset].count += count;
        #endif
        if (addr_slots[pos] == chunk_id) {
            addr_slots[pos + 1] += count;
            return;
        }
        if ((pos % ADDR_SLOT_SIZE) == (ADDR_SLOT_SIZE - 2)) {
            uint32_t npos = get_next_slot_pos();
            uint32_t tpos = pos - ADDR_SLOT_SIZE + 2;
            addr_slots[npos] = tpos;
            addr_slots[npos + 1] = addr_slots[tpos + 1];
            addr_slots[npos + 2] = chunk_id;
            addr_slots[npos + 3] = count;
            addr_slots[tpos + 1] = npos;
            #ifdef USE_ADDR_COUNT_TABLE
            addr_count_table[offset].pos = npos + 2;
            #else
            addr_table[offset] = npos + 2;
            #endif
            return;
        }
        addr_slots[pos + 2] = chunk_id;
        addr_slots[pos + 3] = count;
        #ifdef USE_ADDR_COUNT_TABLE
        addr_count_table[offset].pos = pos + 2;
        #else
        addr_table[offset] = pos + 2;
        #endif
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