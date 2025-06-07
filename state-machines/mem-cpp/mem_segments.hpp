#ifndef __MEM_SEGMENTS_HPP__
#define __MEM_SEGMENTS_HPP__
#include <vector>
#include <map>
#include <stdexcept>
#include <iostream>
#include <forward_list>
#include <mutex>
#include <thread>
#include "mem_config.hpp"
#include "mem_segment.hpp"

class MemSegments {
public:
    std::map<uint32_t, MemSegment *> segments;
    mutable std::mutex mtx;
    MemSegments() {
    }
    ~MemSegments() {
        for (auto segment : segments) {
            delete segment.second;
        }
    }
    void set(uint32_t segment_id, MemSegment *value) {
        std::lock_guard<std::mutex> lock(mtx);
        segments[segment_id] = value;
    }
    void debug () const {
        std::lock_guard<std::mutex> lock(mtx);
        for (const auto &[segment_id, segment] : segments) {
            segment->debug(segment_id);
        }
    }
};
#endif