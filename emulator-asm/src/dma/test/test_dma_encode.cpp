#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sstream>
#include <iostream>

#include "test_dma_encode.hpp"

struct EncodeInfo {
    const char *title;
    uint64_t mask;
    uint64_t rs_bits;
};

EncodeInfo encode_info[] = { 
    {"pre_count", DMA_PRE_COUNT_TEST_MASK, 0},
    {"post_count", DMA_POST_COUNT_TEST_MASK, DMA_POST_COUNT_RS},
    {"pre_writes", DMA_PRE_WRITES_TEST_MASK, DMA_PRE_WRITES_RS},
    {"dst_offset", DMA_DST_OFFSET_TEST_MASK, DMA_DST_OFFSET_RS},
    {"src_offset", DMA_SRC_OFFSET_TEST_MASK, DMA_SRC_OFFSET_RS},
    {"double_src_pre", DMA_DOUBLE_SRC_PRE_TEST_MASK, DMA_DOUBLE_SRC_PRE_RS},
    {"double_src_post", DMA_DOUBLE_SRC_POST_TEST_MASK, DMA_DOUBLE_SRC_POST_RS},
    {"extra_src_reads", DMA_EXTRA_SRC_READS_TEST_MASK, DMA_EXTRA_SRC_READS_RS},
    {"src64_inc_by_pre", DMA_SRC64_INC_BY_PRE_TEST_MASK, DMA_SRC64_INC_BY_PRE_RS},
    {"unaligned_dst_src", DMA_UNALIGNED_DST_SRC_TEST_MASK, DMA_UNALIGNED_DST_SRC_RS},
    {"fill_byte_cmp_negative", DMA_FILL_BYTE_CMD_RES_TEST_MASK, DMA_FILL_BYTE_RS},
    {"requires_dma", DMA_REQUIRES_DMA_TEST_MASK, DMA_REQUIRES_DMA_RS},
    {"lpre_count", DMA_LPRE_COUNT_TEST_MASK, DMA_LPRE_COUNT_RS},
    {"loop_count", DMA_LOOP_COUNT_TEST_MASK, DMA_LOOP_COUNT_RS},
    {"", 0, 0}
};


//                    #bits     bits
// pre_count:  0-7        3     0-2
// post_count: 0-8(*)     4     3-6 (*) memcmp
// pre_writes: 0,1,2      2     7-8
// dst_offset: 0-7        3     9-11
// src_offset: 0-7        3     12-14
// double_src_pre: 0,1    1     15
// double_src_post: 0,1   1     16
// extra_src_reads: 0-3   2     17-18
// src64_inc_by_pre:      1     19
// unaligned_dst_src:     1     20
// fill_byte/cmp:         8     21-28
// cmp_negative:          1     29
// requires_dma:          1     30
// (reserved)             1     31
// lpre_count             3     32-34
// loop_count            29     35-63


uint64_t calculate_encode_memcmp(uint64_t dst, uint64_t src, size_t count, int result) {
    return calculate_encode(dst, src, count, result != 0, true) | DMA_REQUIRES_DMA_TEST_MASK | ((result & DMA_FILL_BITS9_MASK) << DMA_FILL_BYTE_RS);
}

uint64_t calculate_encode_memset(uint64_t dst, size_t count, uint64_t byte) {
    return calculate_encode(dst, 0, count, false, false) | ((byte & DMA_FILL_BYTE_MASK) << DMA_FILL_BYTE_RS);
}

uint64_t calculate_encode_inputcpy(uint64_t dst, size_t count) {
    return calculate_encode(dst, 0, count, false, false);
}

uint64_t calculate_encode(uint64_t dst, uint64_t src, size_t count, bool neq, bool has_src) {

    uint64_t dst_offset = dst & 0x07;
    uint64_t src_offset = src & 0x07;

    uint64_t pre_count = 0;
    uint64_t loop_count = 0;
    uint64_t post_count = 0;

    if (dst_offset > 0) {
        pre_count = 8 - dst_offset;
        if (pre_count >= count) {
            pre_count = count;
        } else {
            uint64_t pending = count - pre_count;
            loop_count = pending >> 3;
            post_count = pending & 0x7;
        }
    } else {
        loop_count = count >> 3;
        post_count = count & 0x07;
    }

    uint64_t pre_writes = (pre_count > 0) + (post_count > 0);
    // uint64_T to_src_offset = (src + count - 1) & 0x07;
    uint64_t src_offset_pos = (src_offset + pre_count) & 0x07;
    uint64_t double_src_post = (src_offset_pos + post_count) > 8;
    uint64_t double_src_pre = (src_offset + pre_count) > 8;
    uint64_t extra_src_reads = count == 0 ? 0 : ((((src + count - 1) >> 3) - (src >> 3) + 1) - loop_count);

    uint64_t src64_inc_by_pre = (pre_count > 0 && (src_offset + pre_count) >= 8);
    uint64_t unaligned_dst_src = count > 0 && src_offset != dst_offset;

    if (neq && post_count == 0 && loop_count > 0) {
        // (dst + count) 0x07 == 7  ==> (dst_offset + count) 0x07 == 7  ==> post_count == 0
        //      loop = loop - 1
        //      pre_writes = pre_writes + 1
        //      post = 8
        //      double_src_post = unaligned_dst_src ? 1:0;
        //      extra_src_reads = extra_src_read + 1
        loop_count -= 1;
        pre_writes += 1;
        post_count = 8;
        double_src_post = src_offset != dst_offset;
        extra_src_reads += 1;
    }
    uint64_t requires_dma = count == 0 || pre_count != 0 || post_count != 0;
    if (has_src) {
        return pre_count
        | (post_count << DMA_POST_COUNT_RS)
        | (pre_writes << DMA_PRE_WRITES_RS)
        | (dst_offset << DMA_DST_OFFSET_RS)
        | (src_offset << DMA_SRC_OFFSET_RS)
        | (double_src_pre << DMA_DOUBLE_SRC_PRE_RS)
        | (double_src_post << DMA_DOUBLE_SRC_POST_RS)
        | (extra_src_reads << DMA_EXTRA_SRC_READS_RS)
        | (src64_inc_by_pre << DMA_SRC64_INC_BY_PRE_RS)
        | (unaligned_dst_src << DMA_UNALIGNED_DST_SRC_RS)
        | (pre_count << DMA_LPRE_COUNT_RS) // optimization to read loop_count * 8 + pre_count
        | (loop_count << DMA_LOOP_COUNT_RS)
        | (requires_dma << DMA_REQUIRES_DMA_RS);
    }
    return 
        pre_count
        | (post_count << DMA_POST_COUNT_RS)
        | (pre_writes << DMA_PRE_WRITES_RS)
        | (dst_offset << DMA_DST_OFFSET_RS)
        | (pre_count << DMA_LPRE_COUNT_RS) // optimization to read loop_count * 8 + pre_count
        | (loop_count << DMA_LOOP_COUNT_RS)
        | (requires_dma << DMA_REQUIRES_DMA_RS);
}

void basic_print_encode_mismatch(uint64_t expected, uint64_t found) {
    static const char *_hexdigits = "0123456789ABCDF";
    char _expected[256];
    char _found[256];
    size_t _iexpected = 0;
    size_t _ifound = 0;
    for (size_t i_digit=0; i_digit<16; ++i_digit) {
        uint8_t byte_expected = (expected >> (60 - 4 * i_digit)) & 0x0F;
        uint8_t byte_found = (found >> (60 - 4 * i_digit)) & 0x0F;
        if (i_digit && (i_digit % 4) == 0) {
            _expected[_iexpected] = '_';
            _found[_ifound] = '_';
            ++_ifound;
            ++_iexpected;
        }
        if (byte_expected != byte_found) {
            strcpy(_expected + _iexpected, "\x1B[1;31m");
            strcpy(_found + _ifound, "\x1B[1;31m");
            _ifound += 7;
            _iexpected += 7;
        }
        _expected[_iexpected] = _hexdigits[byte_expected];
        _found[_ifound] = _hexdigits[byte_found];
        ++_ifound;
        ++_iexpected;
        if (byte_expected != byte_found) {
            strcpy(_expected + _iexpected, "\x1B[0m");
            strcpy(_found + _ifound, "\x1B[0m");
            _ifound += 4;
            _iexpected += 4;
        }
    }
    _expected[_iexpected] = '\0';
    _found[_ifound] = '\0';
    printf("expected:%s\n   found:%s\n", _expected, _found);
}


void print_encode_mismatch(uint64_t expected, uint64_t found) {
    static const char *_hexdigits = "0123456789ABCDF";
    std::stringstream s_expected;
    std::stringstream s_found;
    for (size_t i_digit=0; i_digit<16; ++i_digit) {
        uint8_t byte_expected = (expected >> (60 - 4 * i_digit)) & 0x0F;
        uint8_t byte_found = (found >> (60 - 4 * i_digit)) & 0x0F;
        if (i_digit && (i_digit % 4) == 0) {
            s_expected << '_';
            s_found << '_';
        }
        if (byte_expected != byte_found) {
            s_expected << "\x1B[1;31m";
            s_found << "\x1B[1;31m";
        }
        s_expected << _hexdigits[byte_expected];
        s_found << _hexdigits[byte_found];
        if (byte_expected != byte_found) {
            s_expected << "\x1B[0m";
            s_found << "\x1B[0m";
        }
    }
    size_t i_group = 0;
    while (encode_info[i_group].title[0]) {
        uint64_t g_expected = (expected & encode_info[i_group].mask);
        uint64_t g_found = (found & encode_info[i_group].mask);
        if (g_expected != g_found) {
            s_expected << " " << encode_info[i_group].title << ":" << (g_expected >> encode_info[i_group].rs_bits);
            s_found << " " << encode_info[i_group].title << ":" << (g_found >> encode_info[i_group].rs_bits);
        }
        ++i_group;
    }
    printf("expected:%s\n   found:%s\n", s_expected.str().c_str(), s_found.str().c_str());
}
