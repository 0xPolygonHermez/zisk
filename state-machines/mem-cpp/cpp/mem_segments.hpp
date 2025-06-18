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
        clear();
    }
    void set(uint32_t segment_id, MemSegment *value) {
        std::lock_guard<std::mutex> lock(mtx);
        segments[segment_id] = value;
    }
    void clear() {
        std::lock_guard<std::mutex> lock(mtx);
        for (auto segment : segments) {
            delete segment.second;
        }
        segments.clear();
    }
    void debug () const {
        std::lock_guard<std::mutex> lock(mtx);
        for (const auto &[segment_id, segment] : segments) {
            segment->debug(segment_id);
        }
    }
    size_t size() const {
        std::lock_guard<std::mutex> lock(mtx);
        return segments.size();
    }
    const MemSegment *get(uint32_t segment_id) const {
        std::lock_guard<std::mutex> lock(mtx);
        auto it = segments.find(segment_id);
        if (it != segments.end()) {
            return it->second;
        }
        return nullptr;
    }
};
#endif