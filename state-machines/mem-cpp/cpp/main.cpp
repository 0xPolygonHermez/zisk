#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <fcntl.h>
#include <unistd.h>
#include <sys/stat.h>
#include <vector>
#include <thread>
#include <iostream>
#include <string.h>
#include <sys/time.h>
#include <cstdint>
#include <vector>
#include <map>
#include <unordered_map>
#include <stdexcept>
#include <mutex>
#include <atomic>

#include "mem_types.hpp"
#include "mem_config.hpp"
#include "tools.hpp"
#include "mem_counter.hpp"
#include "mem_align_counter.hpp"
#include "mem_planner.hpp"
#include "mem_locator.hpp"
#include "immutable_mem_planner.hpp"
#include "mem_count_and_plan.hpp"
#include "mem_test.hpp"

// TODO: shared memory slots to balance in a worst scenario
// TODO: incremental memory slots on worst scenario (consolidate full memory slots? to avoid increase).


class LockFreeRingBuffer {
    std::vector<uint32_t> buffer;
    std::atomic<size_t> read_pos{0};
    std::atomic<size_t> write_pos{0};
    size_t capacity;

public:
    LockFreeRingBuffer(size_t size) : buffer(size), capacity(size) {}

    bool try_push(uint32_t value) {
        size_t current_write = write_pos.load(std::memory_order_relaxed);
        size_t next_write = (current_write + 1) % capacity;

        if(next_write == read_pos.load(std::memory_order_acquire)) {
            return false; // full buffer
        }

        buffer[current_write] = value;
        write_pos.store(next_write, std::memory_order_release);
        return true;
    }

    bool try_pop(uint32_t& value) {
        size_t current_read = read_pos.load(std::memory_order_relaxed);

        if(current_read == write_pos.load(std::memory_order_acquire)) {
            return false; // empty buffer
        }

        value = buffer[current_read];
        read_pos.store((current_read + 1) % capacity, std::memory_order_release);
        return true;
    }
};

int main(int argc, const char *argv[]) {
    MemTest mem_test;
    mem_test.load(argc > 1 ? argv[1] : "../bus_data.org/mem_count_data");
    mem_test.execute();
}

