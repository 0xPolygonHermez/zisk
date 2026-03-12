#ifndef __DMA_ENCODE__HPP_
#define __DMA_ENCODE__HPP_

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#define DMA_PRE_COUNT_TEST_MASK          0x07
#define DMA_PRE_COUNT_MASK               0x07
#define DMA_POST_COUNT_RS                3
#define DMA_POST_COUNT_TEST_MASK         0x78
#define DMA_POST_COUNT_MASK              0x0F
#define DMA_PRE_WRITES_RS                7
#define DMA_PRE_WRITES_TEST_MASK         0x180
#define DMA_PRE_WRITES_MASK              0x003
#define DMA_DST_OFFSET_RS                9
#define DMA_DST_OFFSET_TEST_MASK         0x0E00
#define DMA_DST_OFFSET_MASK              0x007
#define DMA_SRC_OFFSET_RS                12
#define DMA_SRC_OFFSET_TEST_MASK         0x70000
#define DMA_SRC_OFFSET_MASK              0x007
#define DMA_DOUBLE_SRC_PRE_RS            15
#define DMA_DOUBLE_SRC_PRE_TEST_MASK     0x08000
#define DMA_DOUBLE_SRC_POST_RS           16
#define DMA_DOUBLE_SRC_POST_TEST_MASK    0x10000
#define DMA_EXTRA_SRC_READS_RS           17
#define DMA_EXTRA_SRC_READS_TEST_MASK    0x60000
#define DMA_EXTRA_SRC_READS_MASK         0x00003
#define DMA_SRC64_INC_BY_PRE_RS          19
#define DMA_SRC64_INC_BY_PRE_TEST_MASK   0x80000
#define DMA_UNALIGNED_DST_SRC_RS         20
#define DMA_UNALIGNED_DST_SRC_TEST_MASK  0x100000
#define DMA_FILL_BYTE_RS                 21
#define DMA_FILL_BYTE_TEST_MASK          0x1FE00000
#define DMA_FILL_BYTE_CMD_RES_TEST_MASK  0x3FE00000
#define DMA_FILL_BYTE_MASK               0x000000FF
#define DMA_FILL_BITS9_MASK              0x000001FF
#define DMA_FILL_BYTE_SIGN_TEST_MASK     0x20000000
#define DMA_LPRE_COUNT_RS                32
#define DMA_LPRE_COUNT_TEST_MASK         0x700000000
#define DMA_LPRE_COUNT_MASK              0x00000007
#define DMA_REQUIRES_DMA_RS              30
#define DMA_REQUIRES_DMA_TEST_MASK       0x40000000
#define DMA_REQUIRES_DMA_MASK            0x00000001
#define DMA_LOOP_COUNT_TEST_MASK         0xFFFFFFF800000000

#define DMA_PRE_OR_POST_TEST_MASK (DMA_PRE_COUNT_TEST_MASK | DMA_POST_COUNT_TEST_MASK)
#define DMA_LOOP_COUNT_RS 35
#define DMA_FULL_ALIGNED_MASK (DMA_PRE_COUNT_TEST_MASK \
        | DMA_POST_COUNT_TEST_MASK \
        | DMA_PRE_WRITES_TEST_MASK \
        | DMA_DST_OFFSET_TEST_MASK \
        | DMA_SRC_OFFSET_TEST_MASK \
        | DMA_DOUBLE_SRC_PRE_TEST_MASK \
        | DMA_DOUBLE_SRC_POST_TEST_MASK \
        | DMA_EXTRA_SRC_READS_TEST_MASK \
        | DMA_SRC64_INC_BY_PRE_TEST_MASK \
        | DMA_UNALIGNED_DST_SRC_TEST_MASK)

#define DMA_DIRECT_MASK (DMA_FULL_ALIGNED_MASK | DMA_REQUIRES_DMA_TEST_MASK)

uint64_t calculate_encode(uint64_t dst, uint64_t src, size_t count, bool neq = false, bool has_src = true);
uint64_t calculate_encode_memset(uint64_t dst, size_t count, uint64_t byte);
uint64_t calculate_encode_memcmp(uint64_t dst, uint64_t src, size_t count, int result = 0);
uint64_t calculate_encode_inputcpy(uint64_t dst, size_t count);
void print_encode_mismatch(uint64_t expected, uint64_t found);


#endif