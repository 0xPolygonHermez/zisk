#ifndef UTILS_HPP
#define UTILS_HPP

#include <sys/time.h>
#include "goldilocks_base_field.hpp"
#include "definitions.hpp"
#include <nlohmann/json.hpp>
#include "zklog.hpp"

using json = nlohmann::json;
using ordered_json = nlohmann::ordered_json;

struct MemoryInfo {
    uint64_t total;
    uint64_t free;
    uint64_t available;
    uint64_t buffers;
    uint64_t cached;
    uint64_t swapCached;
    uint64_t swapTotal;
    uint64_t swapFree;
};

void getMemoryInfo(MemoryInfo &info);
void printMemoryInfo(bool compact = false, const char * pMessage = NULL);
void printProcessInfo(bool compact = false);
// Prints current call stack with function names (mangled)
void printCallStack (void);

// Returns timestamp in UTC, e.g. "20230110_173200_128863"
std::string getTimestamp(void);

// Returns timestamp in UTC, e.g. "1653327845.128863"
std::string getTimestampWithPeriod(void);

// // Converts a json into/from a file
void json2file(const json &j, const std::string &fileName);
void file2json(const std::string &fileName, json &j);
void file2json(const std::string &fileName, ordered_json &j);

// Returns if file exists
bool fileExists (const std::string &fileName);

// Returns file size
uint64_t fileSize (const std::string &fileName);

// Maps memory into a file
void * mapFile (const std::string &fileName, uint64_t size, bool bOutput);
void unmapFile (void * pAddress, uint64_t size);

// Copies file content into memory; use free after use
void * copyFile (const std::string &fileName, uint64_t size);

// Load file in parallel
void * loadFileParallel(const std::string &fileName, uint64_t size);

#endif