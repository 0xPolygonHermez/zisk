.intel_syntax noprefix
.code64

# GENERAL CONSTANTS

.set R_MT_INDEX,     r13
.set R_MT_ADDR,      r12
.set R_STEP,         r14
.set R_AUX,          r9
.set R_AUX2,         rcx  # NOTE: used by rep
.set R_SRC,          rsi  # NOTE: used by rep
.set R_DST,          rdi  # NOTE: used by rep
.set R_COUNT,        rdx
.set R_ENCODE,       rax

# GENERAL CONSTANTS

.equ MAX_MTRACE_REGS_ACCESS_SIZE, (2 + 2 + 3) * 8
.equ CHUNK_SIZE, (1 << 18)
.equ MAX_TRACE_CHUNK_INFO, ((44*8) + 32)
.equ MAX_BYTES_DIRECT_MTRACE, 256
.equ MAX_BYTES_MTRACE_STEP, (MAX_BYTES_DIRECT_MTRACE + MAX_MTRACE_REGS_ACCESS_SIZE)
.equ MAX_CHUNK_TRACE_SIZE, CHUNK_SIZE * MAX_BYTES_MTRACE_STEP + MAX_TRACE_CHUNK_INFO
.equ FAST_ENCODE_TABLE_WO_NEQ_SIZE, 8 * 8 * 16

# 1 encoded + 2 prewrites + 2 src reads
.equ MAX_DMA_EXTRA_BYTES, (2 + 2 + 1) * 8
.equ MAX_DMA_BYTES_DIRECT_MTRACE, (MAX_BYTES_DIRECT_MTRACE - MAX_DMA_EXTRA_BYTES)
.equ MAX_DMA_MT_MARGIN, (MAX_DMA_BYTES_DIRECT_MTRACE + MAX_DMA_EXTRA_BYTES)

.equ EXTRA_PARAMETER_ADDR, 0xA0400F00

# ENCODE CONSTANTS
#
#                     bits  offset  mask
#                     ----  ------  --------
# pre_count:  0-7        3       0  0x0000_0000_0000_0007
# post_count: 0-7        4       3  0x0000_0000_0000_0078 (memcmp uses 8)
# pre_writes: 0,1,2      2       7  0x0000_0000_0000_0180
# dst_offset: 0-7        3       9  0x0000_0000_0000_0E00
# src_offset: 0-7        3      12  0x0000_0000_0000_7000
# double_src_pre: 0,1    1      15  0x0000_0000_0000_8000
# double_src_post: 0,1   1      16  0x0000_0000_0001_0000
# extra_src_reads: 0-3   2      17  0x0000_0000_0006_0000 
# src64_inc_by_pre:      1      19  0x0000_0000_0008_0000
# unaligned_dst_src:     1      20  0x0000_0000_0010_0000
# fill_byte/cmp_res:     8      21  0x0000_0000_1FE0_0000
# requires_dma:          1      29  0x0000_0000_2000_0000
# pre_count: 0-7         3      32  only for optimization PRE_AND_LOOP_BYTES
# loop_count            32       0  0xFFFF_FFFF_0000_0000

# BITS 31,32,32 

.equ DMA_PRE_COUNT_MASK,          0x0000000000000007
.equ DMA_POST_COUNT_MASK,         0x0000000000000078
.equ DMA_PRE_WRITES_MASK,         0x0000000000000180
.equ DMA_DST_OFFSET_MASK,         0x0000000000000E00
.equ DMA_SRC_OFFSET_MASK,         0x0000000000007000
.equ DMA_DOUBLE_SRC_PRE_MASK,     0x0000000000008000
.equ DMA_DOUBLE_SRC_POST_MASK,    0x0000000000010000
.equ DMA_EXTRA_SRC_READS_MASK,    0x0000000000060000
.equ DMA_SRC64_INC_BY_PRE_MASK,   0x0000000000080000
.equ DMA_UNALIGNED_DST_SRC_MASK,  0x0000000000100000
.equ DMA_FILL_BYTE_MASK,          0x000000000FE00000
.equ DMA_CMP_RES_MASK,            0x000000001FE00000
.equ DMA_REQUIRES_DMA_MASK,       0x0000000040000000
.equ DMA_LOOP_COUNT_MASK,         0xFFFFFFFF00000000
.equ DMA_ONLY_LOOP_COUNT_MASK,    0xFFFFFFFF00000000
.equ DMA_FULL_ALIGN_MASK,         0x00000000001FFFFF
.equ DMA_DIRECT_MASK,             0x00000000201FFFFF
.equ DMA_CMP_RES_SIGN_TEST_MASK,  0x0000000020000000


.equ DMA_PRE_COUNT_RS,              0 
.equ DMA_POST_COUNT_RS,             3
.equ DMA_PRE_WRITES_RS,             7
.equ DMA_DST_OFFSET_RS,             9
.equ DMA_SRC_OFFSET_RS,             12
.equ DMA_DOUBLE_SRC_PRE_RS,         15
.equ DMA_DOUBLE_SRC_POST_RS,        16
.equ DMA_EXTRA_SRC_READS_RS,        17
.equ DMA_SRC64_INC_BY_PRE_RS,       19
.equ DMA_UNALIGNED_DST_SRC_RS,      20
.equ DMA_FILL_BYTE_RS,              21
.equ DMA_CMP_RES_RS,                21
.equ DMA_LOOP_COUNT_RS,             35
.equ DMA_PRE_AND_LOOP_BYTES_RS,     32
.equ ALIGN_MASK,                0xFFFFFFFFFFFFFFF8
.equ MOPS_BLOCK_WORDS_RS,       36
.equ MOPS_BLOCK_ONE_WORD,       0x0000001000000000       
.equ MOPS_BLOCK_READ,           0x0000000A00000000
.equ MOPS_BLOCK_WRITE,          0x0000000B00000000
.equ MOPS_ALIGNED_READ,         0x0000000C00000000
.equ MOPS_ALIGNED_WRITE,        0x0000000D00000000
.equ MOPS_ALIGNED_BLOCK_READ,   0x0000000E00000000
.equ MOPS_ALIGNED_BLOCK_WRITE,  0x0000000F00000000

.equ FCALL_PARAMS_LENGTH,       386
.equ FCALL_RESULT_LENGTH,       8193
.equ FCALL_FUNCTION_ID,         0
.equ FCALL_PARAMS_CAPACITY,     FCALL_FUNCTION_ID + 1
.equ FCALL_PARAMS_SIZE,         FCALL_PARAMS_CAPACITY + 1
.equ FCALL_PARAMS,              FCALL_PARAMS_SIZE + 1
.equ FCALL_RESULT_CAPACITY,     FCALL_PARAMS + FCALL_PARAMS_LENGTH
.equ FCALL_RESULT_SIZE,         FCALL_RESULT_CAPACITY + 1
.equ FCALL_RESULT,              FCALL_RESULT_SIZE + 1
.equ FCALL_RESULT_GOT,          FCALL_RESULT + FCALL_RESULT_LENGTH

.equ MOPS_ALIGNED_READ_2W, ((2 << MOPS_BLOCK_WORDS_RS) + MOPS_ALIGNED_BLOCK_READ)
.equ LOOP_COUNT_TO_MOPS_BLOCK, (MOPS_BLOCK_WORDS_RS - DMA_LOOP_COUNT_RS)
.equ PRE_WRITES_TO_MOPS_BLOCK, (MOPS_BLOCK_WORDS_RS - DMA_PRE_WRITES_RS)

# Additional encode constants for fast_dma_encode
.equ DMA_LPRE_COUNT_RS, 32
.equ DMA_FILL_BYTE_RS, 21
.equ DMA_FILL_BITS9_MASK, 0x1FF

.equ ENCODE_MEMSET_ZERO,        DMA_REQUIRES_DMA_MASK
.equ ENCODE_MEMSET_ALIGNED_NO_COUNT_M8, DMA_REQUIRES_DMA_MASK | (1 << DMA_PRE_WRITES_RS)

.ifndef MOPS_RECORD_MACROS
.set MOPS_RECORD_MACROS, 1
# MEMORY-OPS RECORDS
# Every stream word carries a tag in bit 63: set on header words, clear on payload words. A read
# is one header word (address | mode | step field << MOPS_STEP_SHIFT). Block records are a header
# plus the step field as payload. A value block (MOPS_BLOCK_VALUES) is a header (aligned address | mode |
# words << 36 | write step field << 42), a word with the values' bits 63, then up to 63 values
# with bit 63 cleared. R_MOPS_R holds the read step field shifted into place.
.set R_MOPS_R, r10
.equ MOPS_STEP_SHIFT, 38
.equ MOPS_TAG, 0x8000000000000000
.equ MOPS_BLOCK_VALUES, 0x0000000700000000
.equ MOPS_VALUES_STEP_SHIFT, 42
.equ MOPS_VALUES_MAX, 63
# A recorder writing more than this many bytes checks the trace space first.
.equ MAX_DMA_BYTES_DIRECT_MOPS, 256
.equ MAX_DMA_MOPS_MARGIN, 8 * 16
.extern chunk_size

.macro MOPS_STEP_HEADERS
    mov     R_MOPS_R, [chunk_size]
    sub     R_MOPS_R, R_STEP
    shl     R_MOPS_R, 2
    or      R_MOPS_R, 2
    shl     R_MOPS_R, MOPS_STEP_SHIFT
.endm

# \reg = address | non-block read mode: one tagged header word with the step field, appended.
.macro MOPS_REC_READ reg
    add     \reg, R_MOPS_R
    bts     \reg, 63
    mov     [R_MT_ADDR + R_MT_INDEX * 8], \reg
    inc     R_MT_INDEX
.endm

# \reg = block header (address | block mode | word count): tagged header, step field payload.
.macro MOPS_REC_BLOCK reg
    bts     \reg, 63
    mov     [R_MT_ADDR + R_MT_INDEX * 8], \reg
    mov     [R_MT_ADDR + R_MT_INDEX * 8 + 8], R_MOPS_R
    add     R_MT_INDEX, 2
.endm

# The 8-byte block-write descriptor a recorder left at the current index (aligned address in the
# low 32 bits, word count above MOPS_BLOCK_WORDS_RS) becomes value blocks of up to 63 words, the
# values read back from memory: the routine that wrote the words has run. Uses rcx, rdx, rsi,
# rdi, r8, r9, r11; keeps rax. A zero count leaves no record.
.macro MOPS_EXPAND_BLOCK_WRITE
    mov     rdx, [R_MT_ADDR + R_MT_INDEX * 8]
    mov     rcx, rdx
    shr     rcx, MOPS_BLOCK_WORDS_RS      # rcx = words left
    jz      9f
    push    rax
    mov     edx, edx                      # rdx = aligned address
    mov     r11, R_MOPS_R
    shl     r11, 4                        # read step field moved to bit 42
    bts     r11, MOPS_VALUES_STEP_SHIFT   # slot 3, the write slot
    bts     r11, 63                       # tag
    mov     r8, MOPS_BLOCK_VALUES
    add     r11, r8                       # r11 = mode | write step field | tag
5:  # one value block of r8 = min(words left, 63) words
    mov     r8, MOPS_VALUES_MAX
    cmp     rcx, r8
    cmovb   r8, rcx
    sub     rcx, r8
    lea     rdi, [R_MT_ADDR + R_MT_INDEX * 8]
    mov     r9, r8
    shl     r9, MOPS_BLOCK_WORDS_RS
    add     r9, rdx
    add     r9, r11
    mov     [rdi], r9                     # header
    lea     R_MT_INDEX, [R_MT_INDEX + r8 + 2]
    add     rdi, 16                       # first value slot; the mask slot is at rdi - 8
    xor     r9d, r9d                      # r9 = the values' bits 63
    xor     esi, esi                      # rsi = index in the block
8:  mov     rax, [rdx]
    bt      rax, 63
    jnc     7f
    bts     r9, rsi
    btr     rax, 63
7:  mov     [rdi], rax
    add     rdx, 8
    add     rdi, 8
    inc     rsi
    cmp     rsi, r8
    jb      8b
    lea     rsi, [r8 * 8 + 8]
    sub     rdi, rsi
    mov     [rdi], r9                     # the mask word
    test    rcx, rcx
    jnz     5b
    pop     rax
9:
.endm
.endif
