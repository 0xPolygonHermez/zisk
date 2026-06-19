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
    bool compare(const MemSegments &other) const {
        bool equal = true;
        for (const auto &[segment_id, segment] : segments) {
            auto it = other.segments.find(segment_id);
            if (it == other.segments.end()) {
                printf("DIFF segment %d: only in A (%d chunks)\n", segment_id, segment->size());
                equal = false;
            } else {
                if (!segment->compare(*it->second, segment_id)) {
                    equal = false;
                }
            }
        }
        for (const auto &[segment_id, segment] : other.segments) {
            if (segments.find(segment_id) == segments.end()) {
                printf("DIFF segment %d: only in B (%d chunks)\n", segment_id, segment->size());
                equal = false;
            }
        }
        return equal;
    }
};
#endif