#ifndef __MEM_LOCATOR_HPP__
#define __MEM_LOCATOR_HPP__

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
struct MemLocator {
    uint32_t offset;
    uint32_t cpos;
    uint32_t skip;
    uint32_t thread_index;
};

#endif