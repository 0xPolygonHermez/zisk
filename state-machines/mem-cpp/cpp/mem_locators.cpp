#include "mem_locators.hpp"

MemLocators::MemLocators() {
}

void MemLocators::push_locator(uint32_t thread_index, uint32_t offset, uint32_t cpos, uint32_t skip) {
    size_t pos = write_pos.load(std::memory_order_relaxed);
    locators[pos].thread_index = thread_index;
    locators[pos].offset = offset;
    locators[pos].cpos = cpos;
    locators[pos].skip = skip;
    write_pos.store(pos + 1, std::memory_order_relaxed);
}

MemLocator *MemLocators::get_locator(uint32_t &segment_id) {
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

