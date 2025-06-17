#ifndef __CIRCULAR_QUEUE_HPP__
#define __CIRCULAR_QUEUE_HPP__

#include <atomic>
#include <vector>
#include <algorithm>
#include <thread>

#include <immintrin.h>
#include <vector>
#include <atomic>

template<typename T, size_t Capacity>
class CircularQueue {
    T buffer[Capacity];
    std::atomic<size_t> head{0};  // read position (consumer)
    std::atomic<size_t> tail{0};  // write position (producer)

    static_assert((Capacity & (Capacity - 1)) == 0,
                 "Capacity must be a power of 2");

    static constexpr size_t CapacityMask = Capacity - 1;
public:
    CircularQueue() {}

    // producer: insert an element, return false if queue is full
    bool push(const T& value) {
        size_t current_tail = tail.load(std::memory_order_relaxed);
        size_t next_tail = (current_tail + 1) & CapacityMask;

        // check if space is available (producer only updates tail)
        if (next_tail == head.load(std::memory_order_acquire)) {
            return false; // queue full
        }

        buffer[current_tail] = value;
        tail.store(next_tail, std::memory_order_release);
        return true;
    }

    // consumer: extract one element, return false if queue is empty
    bool pop(T& value) {
        size_t current_head = head.load(std::memory_order_relaxed);

        // check if there are elements (consumer only updates head)
        if (current_head == tail.load(std::memory_order_acquire)) {
            return false; // queue empty
        }

        value = buffer[current_head];
        head.store((current_head + 1) & CapacityMask, std::memory_order_release);
        return true;
    }
};

#endif