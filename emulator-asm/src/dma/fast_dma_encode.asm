.intel_syntax noprefix
.code64

# Include DMA constants
.include "dma_constants.inc"

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
#     3-6: post_count         - Bytes to copy after aligned chunks (0-8) (* 8 memset case)
#     7-8: pre_writes         - Number of pre/post partial writes (0, 1, or 2)
#    9-11: dst_offset         - Byte offset within dst qword (0-7)
#   12-14: src_offset         - Byte offset within src qword (0-7)
#      15: double_src_pre     - Flag: pre-read spans two src qwords
#      16: double_src_post    - Flag: post-read spans two src qwords
#   17-18: extra_src_reads    - Additional src qword reads needed (0-3)
#      19: src64_inc_by_pre   - Flag: indicate loop use src64 + 8 
#      20: unaligned_dst_src  - Flag: dst and src has diferent alignement
#   21-28: fill_byte/cmp_res  - Byte value for fill or compare result
#      29: cmp_negative flag  - Comparation between two bytes generate 9 bits (one of them for sign)
#      30: requires_dma       - Flag: indicates if operation requires DMA (*)
#      31: reserved 
#   32-34: pre_count (loop)   - Byte value for fill or compare result
#   35-63: loop_count         - Number of 8-byte chunks in main copy loop
#
# (*) when compare exists an edge case when dst is aligned and effetive_count is multiple of 8, only
#     PRE_POST machine could verify the different byte, in this case POST could be 8 bytes of length.
#     effective_count is the number of bytes to check to compare.
#
# (*) The requires_dma flag is set by function caller when the operation it's a memcmp, because for
#     this operation always a DMA is required.
#
################################################################################

.global fast_dma_memcpy_encode
.global fast_dma_memcmp_encode
.global fast_dma_memset_encode
.global fast_dma_memset_with_byte_encode
.global fast_dma_inputcpy_encode
.global fast_dma_memcmp_with_result_encode
.global fast_dma_encode
.section .text

.include "dma_constants.inc"
.include "fast_dma_encode_macro.inc"

# PARAMETERS (System V AMD64 ABI):
#   rdi = dst (u64)        - Destination address
#   rsi = src (u64)        - Source address
#   rdx = count (usize)    - Number of bytes to copy
# RESULT rax = encoded value
fast_dma_memcpy_encode:
    FAST_DMA_ENCODE
    ret

# PARAMETERS (System V AMD64 ABI):
#   rdi = dst (u64)        - Destination address
#   rsi = src (u64)        - Source address
#   rdx = count (usize)    - Number of bytes to copy
# RESULT rax = encoded value
# NOTE: This function don't encode the result, only take in consideration to calculate
# NOTE: FAST_ENCODE_TABLE_WO_NEQ_SIZE ==> DMA_REQUIRES_DMA_MASK
fast_dma_memcmp_neq_encode:
    FAST_DMA_ENCODE_MEMCMP FAST_ENCODE_TABLE_WO_NEQ_SIZE 
    ret

# PARAMETERS (System V AMD64 ABI):
#   rdi = dst (u64)        - Destination address
#   rsi = src (u64)        - Source address
#   rdx = count (usize)    - Number of bytes to copy
# RESULT rax = encoded value
# NOTE: This function don't encode the result, only take in consideration to calculate
fast_dma_memcmp_eq_encode:
    FAST_DMA_ENCODE_MEMCMP 0
    ret

# PARAMETERS (System V AMD64 ABI):
#   rdi = dst (u64)        - Destination address
#   rsi = src (u64)        - Source address
#   rdx = count (usize)    - Number of bytes to copy
#   r9  = result (9 bits)  - NOTE: value will be modified
# RESULT rax = encoded value
fast_dma_memcmp_encode:
    and     r9, DMA_FILL_BITS9_MASK  # Ensure result is in lower 9 bits
    jz      .L_fast_dma_memcmp_encode_eq
    FAST_DMA_ENCODE_MEMCMP FAST_ENCODE_TABLE_WO_NEQ_SIZE
    shl     r9, DMA_FILL_BYTE_RS     # r8 has the result byte in the lower 8 bits
    or      rax, r9
    ret

.L_fast_dma_memcmp_encode_eq:
    FAST_DMA_ENCODE_MEMCMP 0
    ret


# PARAMETERS:
#   rdi = dst (u64)        - Destination address
#   rdx = count (usize)    - Number of bytes to copy


fast_dma_inputcpy_encode:
fast_dma_no_src_encode:
    FAST_DMA_ENCODE_NO_SRC   
    ret


# PARAMETERS (System V AMD64 ABI):
#   rdi = dst (u64)        - Destination address
#   rsi = fill byte        - Source address
#   rdx = count (usize)    - Number of bytes to copy
fast_dma_memset_with_byte_encode:
    FAST_DMA_ENCODE_NO_SRC
    movzx   r9, sil  
    shl     r9, DMA_FILL_BYTE_RS     # r8 has the result byte in the lower 8 bits
    or      rax, r9
    ret

# Mark stack as non-executable (required by modern linkers)
.section .note.GNU-stack,"",%progbits

# Include the lookup table in the .rodata section
.include "fast_dma_encode_table.asm"
