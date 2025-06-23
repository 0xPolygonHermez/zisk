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

class MemLocators;

#include "mem_types.hpp"
#include "mem_config.hpp"
#include "mem_locator.hpp"

class MemLocators {
public:
    std::atomic<size_t> write_pos{0};
    std::atomic<size_t> read_pos{0};
    std::atomic<bool> completed{false};
    MemLocator locators[MAX_LOCATORS];
    MemLocators();
    void push_locator(uint32_t thread_index, uint32_t offset, uint32_t cpos, uint32_t skip);
    MemLocator *get_locator(uint32_t &segment_id);
    inline void set_completed();
    inline bool is_completed();
    inline size_t size();
};

void MemLocators::set_completed() {
    completed.store(true, std::memory_order_release);
}
bool MemLocators::is_completed() {
    return completed.load(std::memory_order_acquire);
}
size_t MemLocators::size() {
    return write_pos.load(std::memory_order_relaxed);
}
#endif