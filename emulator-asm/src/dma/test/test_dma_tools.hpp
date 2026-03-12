#ifndef __TEST_DMA_TOOLS__HPP__
#define __TEST_DMA_TOOLS__HPP__

#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "test_dma_tools.hpp"

uint8_t *fill_pattern(uint8_t *data, size_t count, uint64_t seed);
bool check_pattern_slice(uint64_t *data, size_t from, size_t to, uint64_t seed);
bool check_pattern_exclude_slice(uint64_t *data, size_t count, size_t from, size_t to, uint64_t seed);
int create_memcmp_data(uint8_t *dst, uint8_t *src, size_t ef_count, int diff_dst_src);

#endif