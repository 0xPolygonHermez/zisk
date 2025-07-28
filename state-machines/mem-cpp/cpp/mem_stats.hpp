#include <mutex>
#include <vector>

#ifndef MEM_STATS_HPP
#define MEM_STATS_HPP

// Uncomment to activate the memory statistics feature
//#define MEM_STATS_ACTIVE

#ifdef MEM_STATS_ACTIVE

#define MEM_STATS_COUNT_PHASE 1
#define MEM_STATS_PLAN_PHASE 2
#define MEM_STATS_EXECUTE_CHUNK_0 3
#define MEM_STATS_EXECUTE_CHUNK_1 4
#define MEM_STATS_EXECUTE_CHUNK_2 5
#define MEM_STATS_EXECUTE_CHUNK_3 6
#define MEM_STATS_EXECUTE_CHUNK_4 7
#define MEM_STATS_EXECUTE_CHUNK_5 8
#define MEM_STATS_EXECUTE_CHUNK_6 9
#define MEM_STATS_EXECUTE_CHUNK_7 10

class MemStats{
public:
    std::vector<uint64_t> stats;
    std::mutex stats_mutex;
public:
    MemStats() = default;
    ~MemStats() = default;

    // Add other member functions and variables as needed

    void add_stat(uint64_t id, uint64_t start_seconds, uint64_t start_nanos, uint64_t duration_nanos) {
        stats_mutex.lock();
        stats.push_back(id);
        stats.push_back(start_seconds);
        stats.push_back(start_nanos);
        stats.push_back(duration_nanos);
        stats_mutex.unlock();
    }

};

#endif // MEM_STATS_ACTIVE

#endif // MEM_STATS_HPP