#include <stdint.h>
#include <stdio.h>
#include <unistd.h>
#include <string.h>
#include <random>
#include <vector>
#include <cstdint>

#include "test_dma_tools.hpp"

uint8_t *fill_pattern(uint8_t *data, size_t count, uint64_t seed) {
    std::mt19937_64 rng(seed);
    uint64_t *p_data = (uint64_t *)data;

    size_t count64 = count >> 3;
    for (size_t i = 0; i < count64; ++i) {
        p_data[i] = rng();
    }
    size_t count_bytes = count & 0x07;
    if (count_bytes > 0) {
        uint64_t value = rng();
        uint8_t *value_bytes = (uint8_t *)&value;
        uint8_t *bytes = data + (count64 << 3);
        for (size_t i = 0; i < count_bytes; ++i) {
            bytes[i] = value_bytes[i];
        }
        
    }
    return data;
}

bool check_pattern_slice(uint64_t *data, size_t from, size_t to, uint64_t seed) {
    std::mt19937_64 rng(seed);

    for (size_t i = 0; i < to; ++i) {
        const uint64_t rvalue = rng();
        if (i < from) continue;
        if (data[i] != rvalue) {
            return false;
        }
    }
    return true;
}

bool check_pattern_exclude_slice(uint64_t *data, size_t count, size_t from, size_t to, uint64_t seed) {
    std::mt19937_64 rng(seed);

    for (size_t i = 0; i < count; ++i) {
        const uint64_t rvalue = rng();
        if (i >= from && i <= to) continue;
        if (data[i] != rvalue) {
            return false;
        }
    }
    return true;
}

int create_memcmp_data(uint8_t *dst, uint8_t *src, size_t ef_count, int diff_dst_src) {
    if (ef_count == 0) {
        return 0;
    }

    size_t count = diff_dst_src == 0 ? ef_count : ef_count - 1;

    for (size_t i = 0; i < count; ++i) {
        dst[i] = src[i];
    }
    if (diff_dst_src > 0) {
        if (src[count] == 255) {
            src[count] = 254;
            dst[count] = 255;
            return 1;
        }
        if (diff_dst_src > (255 - (int)src[count])) {
            dst[count] = 255;
            return 255 - (int)src[count];
        }
        dst[count] = (int) src[count] + diff_dst_src;
        return diff_dst_src;
    }
    if (diff_dst_src < 0) {
        if (src[count] == 0) {
            src[count] = 1;
            dst[count] = 0;
            return -1;
        }
        if (diff_dst_src < (0 - (int)src[count])) {
            dst[count] = 0;
            return 0 - (int)src[count];
        }
        dst[count] = (int) src[count] + diff_dst_src;
        return diff_dst_src;
    }
    return 0;
}
