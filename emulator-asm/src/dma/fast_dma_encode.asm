.intel_syntax noprefix
.code64

################################################################################
# fast_dma_encode - Optimized function to encode dma information
#
# REGISTER USAGE:
# Modified registers: rax, r8
# Does NOT use XMM registers (caller doesn't need to save them)
#
# PARAMETERS (System V AMD64 ABI):
#   rdi = dst (u64)        - Destination address
#   rsi = src (u64)        - Source address
#   rdx = count (usize)    - Number of bytes to copy
#
# RETURN VALUE:
#   rax = encoded value
#
# ENCODED METADATA (bits):
#     0-2: pre_count          - Bytes to copy before alignment (0-7)
#     3-5: post_count         - Bytes to copy after aligned chunks (0-7)
#     6-7: pre_writes         - Number of pre/post partial writes (0, 1, or 2)
#    8-10: dst_offset         - Byte offset within dst qword (0-7)
#   11-13: src_offset         - Byte offset within src qword (0-7)
#      14: double_src_pre     - Flag: pre-read spans two src qwords
#      15: double_src_post    - Flag: post-read spans two src qwords
#   16-17: extra_src_reads    - Additional src qword reads needed (0-3)
#      18: src64_inc_by_pre   - Flag: indicate loop use src64 + 8 
#      19: unaligned_dst_src  - Flag: dst and src has diferent alignement
#   32-63: loop_count         - Number of 8-byte chunks in main copy loop
################################################################################

.global fast_dma_encode

.section .text

fast_dma_encode:
    mov     rax, rdi
    and     rax, 0x07               # dst_offset (0-7)
    shl     rax, 7                  # dst_offset << 7

    mov     r8, rsi
    and     r8, 0x07                # src_offset (0-7)
    shl     r8, 4                   # src_offset << 4

    or      rax, r8                 # combine dst and src offsets

    # Calculate table_count
    mov     r8, rdx
    cmp     r8, 16
    jb      .L_count_lt_16
    
    # count >= 16: table_count = (count & 0x07) | 0x08
    and     r8, 0x07
    or      r8, 0x08

.L_count_lt_16:
    or      rax, r8                 # rax = index = (dst<<7) + (src<<4) + table_count
    
    # Look up encoded value in table (direct access since it's in the same file)
    mov     rax, [fast_dma_encode_table + rax * 8]

    # Add (count >> 3) to result
    mov     r8, rdx
    shl     r8, 29                  # r8 = count << 29
    add     rax, r8                 # result += (count << 29)
    
    ret

# Mark stack as non-executable (required by modern linkers)
.section .note.GNU-stack,"",%progbits

# Include the lookup table in the .rodata section
.include "fast_dma_encode_table.asm"
