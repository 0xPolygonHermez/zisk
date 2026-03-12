.intel_syntax noprefix
.code64

################################################################################
# memcmp_mops - Optimized version with memory ops tracing and actual copy. This
#               is an variant of memcmp operation, xmemcmp operation, that it
#               doesn't read the count from zisk memory.
#
# This function performs two main tasks:
# 1. Records all addresses of memory operations (read and write addresses)
# 2. Performs the actual memory copy from src to dst (with overlap handling)
#
# REGISTER USAGE:
# Uses general-purpose registers: rax, rbx, rcx, rdx, rdi, rsi, r9, r11, r12, r13
# Does NOT use XMM registers (caller doesn't need to save them)
# Preserves callee-saved registers (rbx, r12, r13 saved/restored in wrapper)
#
# PARAMETERS (NON System V AMD64 ABI):
#   rdi -> rbx = dst (u64)              - Destination address
#   rsi -> rax = src (u64)              - Source address
#   rdx -> count (usize)                - Number of bytes to copy
#   [r12 + r13*8] = trace_ptr (u64*)    - Pointer to memory trace buffer (input/output)
#
# MEMORY COPY BEHAVIOR:
# - Handles overlapping src/dst correctly (like memmove)
# - For non-overlapping: optimized copy using pre_count/loop_count/post_count
# - For overlapping: backward byte-by-byte copy to avoid corruption
################################################################################

.global direct_dma_memcmp_mops
.global direct_dma_xmemcmp_mops
.extern fast_dma_encode
.extern fast_memcpy
.extern fast_memcpy64

.include "dma_constants.inc"
.include "fast_dma_encode_macro.inc"

.section .text

# call directly from assembly without standard ABI call
# more eficient

direct_dma_memcmp_mops:
   
    # updated registers: 
    #       r9 = no save (value_reg)
    #       rcx = no save (available from asm)
    #       rdi = no save (available from asm)
    #       rsi = no save (available from asm)
    #       r13 = with new mops index (output)
    #       rax = encoded 

    mov     r9, (MOPS_ALIGNED_READ + EXTRA_PARAMETER_ADDR)
    mov     [r12 + r13 * 8], r9
    inc     r13

direct_dma_xmemcmp_mops:

    # Call fast_dma_encode to calculate encoding
    # Parameters already in correct registers: rdi=dst, rsi=src, rdx=count
    # Result will be returned in rax (encoded value)

    test    rdx, rdx
    jz      .L_dma_memcmp_mops_count_zero

    call    fast_memcmp

    # in case of original count > 0, the effective count must be > 0 because at least
    # need check one byte to see if are equals or not

    # at end the memcpy must return rax with correct value, but these information
    # could be extract from encoded, this is the stragegy to avoid manage two values
    # encoded and result

    mov     r9, rax
    and     r9, 0x1FF
    jz      .L_fast_dma_memcmp_encode_eq 

    # Non-equal case: use table with NEQ offset
    FAST_DMA_ENCODE_MEMCMP FAST_ENCODE_TABLE_WO_NEQ_SIZE

    shl     r9, DMA_CMP_RES_RS                # 1 cycle - shift cmp_result to position (bits 21-28)
    or      rax, r9                           # 1 cycle - combine with encoding
    jmp     .L_pre_dst_to_mops                # 1 cycle

.L_fast_dma_memcmp_encode_eq:
    # Equal case: use base table (offset 0)
    FAST_DMA_ENCODE_MEMCMP 0

.L_pre_dst_to_mops:
    # If pre_count > 0, write aligned dst value to trace
    test    rax, DMA_PRE_COUNT_MASK        # 1 cycle - check if pre_count > 0
    jz      .L_post_dst_to_mops          # 2 cycles (predicted taken)

.L_pre_is_active:
    # Branch with pre_count > 0: save original dst value before it's overwritten
    mov     r9, MOPS_ALIGNED_READ      # r9 = flags aligned read
    add     r9, rdi                    # 1 cycle - get original dst
    and     r9, ALIGN_MASK             # 1 cycle - align to 8-byte boundary
    mov     [r12 + r13 * 8], r9        # ~4 cycles - write dst pre-address to trace

    test    rax, DMA_DOUBLE_SRC_PRE_MASK
    jnz     .L_pre_double_src_to_mops

.L_pre_single_src_to_mops:

    mov     r9, MOPS_ALIGNED_READ      # r9 = flags aligned read
    add     r9, rsi                    # 1 cycle - get original src
    and     r9, ALIGN_MASK             # 1 cycle - align to 8-byte boundary
    mov     [r12 + r13 * 8 + 8], r9    # ~4 cycles - write src address
    jmp     .L_pre_src_inc_mops_index

.L_pre_double_src_to_mops:

    mov     r9, MOPS_ALIGNED_READ_2W   # r9 = flags double read
    add     r9, rsi                    # 1 cycle - get original src
    and     r9, ALIGN_MASK             # 1 cycle - align to 8-byte boundary
    mov     [r12 + r13 * 8 + 8], r9    # ~4 cycles - write src address

.L_pre_src_inc_mops_index:
    add     r13, 2                     # add 2 (pre-write, block single/dual src)

.L_post_dst_to_mops:

    # If post_count > 0, write aligned (dst+count) value to trace
    test    rax, DMA_POST_COUNT_MASK       # 1 cycle - check if post_count > 0
    jz      .L_src_to_mops               # 2 cycles (predicted taken) - skip to src copy

.L_post_is_active:
    # preparing post pre-write read
    mov     rcx, MOPS_ALIGNED_READ     # rcx = flags aligned read
    lea     r9, [rdi + rdx - 1]        # 1 cycle - r9 = dst + count - 1 (last dst byte)
    and     r9, ALIGN_MASK             # 1 cycle - align to 8-byte boundary
    add     r9, rcx                    # 1 cycle - r9 mops with dst aligned address
    mov     [r12 + r13 * 8], r9        # ~4 cycles - write dst post-value to trace

    # preparing post src read, calculating base src address
    mov     r9, rax
    shr     r9, DMA_PRE_AND_LOOP_BYTES_RS
    add     r9, rsi
    and     r9, ALIGN_MASK

    # check if single read or double read
    test    rax, DMA_DOUBLE_SRC_POST_MASK
    jnz     .L_post_double_src_to_mops

.L_post_single_src_to_mops:
    # not double read, load flags and store in mops trace
    mov     rcx, MOPS_ALIGNED_READ     # r9 = flags aligned read
    add     r9, rcx                    # 1 cycle - get original src
    mov     [r12 + r13 * 8 + 8], r9    # ~4 cycles - write src address
    jmp     .L_post_src_inc_mops_index

.L_post_double_src_to_mops:
    # its double read, load flags for double read, and store in mops trace
    mov     rcx, MOPS_ALIGNED_READ_2W  # r9 = flags aligned read
    add     r9, rcx                    # 1 cycle - get original src
    mov     [r12 + r13 * 8 + 8], r9    # ~4 cycles - write src address

.L_post_src_inc_mops_index:
    # adding two "slots", because we store pre-write read and source-read
    add     r13, 2                     # add 2 (pre-write, block single/dual src)

.L_src_to_mops:
    # extract loop count from encoded
    mov     rcx, rax                    # 1 cycle - rcx = encoded    
    shr     rcx, DMA_LOOP_COUNT_RS      # 1 cycle - rcx = loop (32 bits)
    
    # check edge case loop_count = 0
    jz      .L_prepare_result
    shl     rcx, MOPS_BLOCK_WORDS_RS    # 1 cycle - rcx = loop | 0 (4 bits) | (32 bits)

    # in case of unaligned loop, add and extra read because each qword verification
    # of dst need part of current read and part of next read.

    test    rax, DMA_UNALIGNED_DST_SRC_MASK
    jnz     .L_src_extra_for_unaligned_loop

    # add flags of aligned block read
    mov     r9, MOPS_ALIGNED_BLOCK_READ    # 1 cycle - r9 = read block
    jmp     .L_src_block_before_address

.L_src_extra_for_unaligned_loop:    
    # add special flags with that add one to current count loop count in r9
    mov     r9, MOPS_ALIGNED_BLOCK_READ + MOPS_BLOCK_ONE_WORD

.L_src_block_before_address:
    # at this point in r9 we have flags, and lenght but we need to add the 
    # base src address. For do it, first we need to know if we need to pass
    # the first src because it was used only for pre part.
    add     r9, rcx
    test    rax, DMA_SRC64_INC_BY_PRE_MASK
    jnz     .L_src_incr_by_pre
    add     r9, rsi                        # 1 cycle - rcx = first block src address
    jmp     .L_src_to_mops_ready

.L_src_incr_by_pre: 
    # in this patch the first src address is used exclusively by pre part, for this 
    # reason we add rsi + 8 to r9 
    lea     r9, [rsi + r9 + 8]

.L_src_to_mops_ready:
    # before store all, we need to align them
    and     r9, ALIGN_MASK
    mov     [r12 + r13 * 8], r9            # ~4 cycles - write first block src read address

    # rcx = loop_count
    mov     r9, MOPS_ALIGNED_BLOCK_READ
    add     rcx, r9
    test    rax, DMA_PRE_COUNT_MASK
    jz      .L_dst_loop_has_not_offset
    add     rcx, 8

.L_dst_loop_has_not_offset:
    add     rcx, rdi
    and     rcx, ALIGN_MASK
    mov     [r12 + r13 * 8 + 8], rcx      # ~4 cycles - write first block src read address

    add     r13, 2

.L_prepare_result: 
    # how we are in comparation, we don't write pre/post parts because when pre-read
    # this parts and with this is enough. In case of loop is different because we don't
    # pre-read for this reason we need to read to verify that are equals. The loop part
    # only could verify that all are equal.

    # extract result from encoded
    shr     rax, DMA_CMP_RES_RS
    test    rax, 0x100
    jnz     .L_memcmp_mops_negative_res
    and     rax, 0xFF
    jmp     .L_memcmp_mops_res_ready

.L_memcmp_mops_negative_res:
    or      rax, 0xFFFFFFFFFFFFFF00

.L_memcmp_mops_res_ready:
    ret

.L_dma_memcmp_mops_count_zero:
    xor     rax, rax
    ret

# Performance estimate (Modern x86-64, L1 cache hits):
#
# NON-OVERLAPPING FORWARD COPY PATH:
# - fast_dma_encode call:           ~15-20 cycles (function call + table lookup)
# - Write mops entries:             ~4-6 cycles per entry
# - Pre-read mops (conditional):    ~12 cycles (if pre_count > 0)
# - Post-read mops (conditional):   ~12 cycles (if post_count > 0)
# - Block src read mops:            ~8-12 cycles (address calculation + write)
# - Pre-bytes copy:                 ~3-5 cycles per byte (if pre_count > 0, max 7 bytes)
# - Aligned qwords copy:            ~1.5-2 cycles per qword (rep movsq, main data)
# - Post-bytes copy:                ~3-5 cycles per byte (if post_count > 0, max 7 bytes)
# - Function overhead:              ~10 cycles (push/pop, branches, return)
#
# TOTAL (best case, aligned, no pre/post):
#   ~30 cycles base + ~2 cycles per qword (trace + copy)
#
# TOTAL (typical case, some alignment):
#   ~50 cycles base + ~2 cycles per qword + ~4 cycles per pre/post byte
#
# OVERLAPPING BACKWARD COPY PATH:
# - Same mops overhead:             ~30-50 cycles
# - std instruction:                ~20-50 cycles (serializing, causes pipeline flush)
# - Backward byte copy:             ~3-5 cycles per byte (rep movsb backward)
# - cld instruction:                ~20-50 cycles (serializing, causes pipeline flush)
#
# TOTAL (overlap, worst case):
#   ~100-150 cycles base + ~4-5 cycles per byte
#
# NOTES:
# - Assumes L1 cache hits for all memory accesses
# - rep movsq/movsb performance varies by microarchitecture
# - Actual cycles may vary ±20% depending on CPU model and memory alignment
# - Fast path (aligned, no overlap) is ~2-3x faster than overlap path

# Mark stack as non-executable (required by modern linkers)
.section .note.GNU-stack,"",%progbits
