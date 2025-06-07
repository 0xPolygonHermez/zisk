#ifndef __MEM_LOCATORS_HPP__
#define __MEM_LOCATORS_HPP__

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <unistd.h>
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
#include "mem_locator.hpp"

class MemLocators {
public:
    std::atomic<size_t> write_pos{0};
    std::atomic<size_t> read_pos{0};
    std::atomic<bool> completed{false};
    MemLocator locators[MAX_LOCATORS];
    MemLocators() {
    }
    void push_locator(uint32_t thread_index, uint32_t offset, uint32_t cpos, uint32_t skip) {
        size_t pos = write_pos.load(std::memory_order_relaxed);
        locators[pos].thread_index = thread_index;
        locators[pos].offset = offset;
        locators[pos].cpos = cpos;
        locators[pos].skip = skip;
        write_pos.store(pos + 1, std::memory_order_relaxed);
    }
    MemLocator *get_locator(uint32_t &segment_id) {
        size_t current_read = read_pos.load(std::memory_order_relaxed);
        size_t current_write;
        MemLocator *item;

        do {
            current_write = write_pos.load(std::memory_order_acquire);
            if (current_read == current_write) return nullptr;
            item = &locators[current_read];
        } while (!read_pos.compare_exchange_weak(
            current_read,
            current_read + 1,
            std::memory_order_release,
            std::memory_order_relaxed
        ));
        segment_id = current_read;
        return item;
    }
    void set_completed() {
        completed.store(true, std::memory_order_release);
    }
    bool is_completed() {
        return completed.load(std::memory_order_acquire);
    }
    size_t size() {
        return write_pos.load(std::memory_order_relaxed);
    }
};

#endif