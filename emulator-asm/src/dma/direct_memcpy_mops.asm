.intel_syntax noprefix
.code64

################################################################################
# memcpy_mops - Optimized version with memory ops tracing and actual copy
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

.global direct_dma_memcpy_mops
.global dma_memcpy_mops
.extern fast_dma_encode

.include "dma_constants.inc"
.equ MOPS_ALIGNED_READ_2W, ((2 << MOPS_BLOCK_WORDS_SBITS) + MOPS_ALIGNED_BLOCK_READ)
.equ LOOP_COUNT_TO_MOPS_BLOCK, (MOPS_BLOCK_WORDS_SBITS - LOOP_COUNT_SBITS)
.equ PRE_WRITES_TO_MOPS_BLOCK, (MOPS_BLOCK_WORDS_SBITS - PRE_WRITES_SBITS)

.section .text

#    mov     rdx, [0xA000_0F00]                     # save count before call
#    mov     rdi, rbx                # rdi = dst (rbx)
#    mov     rsi, rax                # rsi = src (rax)

#   [r12 + r13*8] = trace_ptr (u64*)    - Pointer to memory trace buffer (input/output)

# call function with standard ABI call 
dma_memcpy_mops:

    # save registers used
    push    r12         # register used as mops base address
    push    r13         # register used as mops index
    push    rbx         #
    
    mov     r12, rcx
    xor     r13, r13
    call    direct_dma_memcpy_mops

    mov     rax, r13
    pop     rbx
    pop     r13
    pop     r12

    ret

# call directly from assembly without standard ABI call
# more eficient

direct_dma_memcpy_mops:
   
    # updated registers: 
    #       r9 = no save (value_reg)
    #       rcx = no save (available from asm)
    #       rdi = no save (available from asm)
    #       rsi = no save (available from asm)
    #       r13 = with new mops index (output)
    #       rax = encoded 

    # Call fast_dma_encode to calculate encoding
    # Parameters already in correct registers: rdi=dst, rsi=src, rdx=count
    # Result will be returned in rax (encoded value)

    call    fast_dma_encode         # ~15-20 cycles - table lookup encoding

    # add read parameter count to mops

    mov     r9, (MOPS_ALIGNED_READ + EXTRA_PARAMETER_ADDR)
    mov     [r12 + r13 * 8], r9
    inc     r13

    # check if count is zero
    test    rdx, rdx                   # compare count
    jz      .L_done                      # jump if zero

.L_pre_dst_to_mops:
    # If pre_count > 0, write aligned dst value to trace
    test    rax, PRE_COUNT_MASK        # 1 cycle - check if pre_count > 0
    jz      .L_post_dst_to_mops          # 2 cycles (predicted taken)

.L_pre_is_active:
    # Branch with pre_count > 0: save original dst value before it's overwritten
    mov     r9, MOPS_ALIGNED_READ      # r9 = flags aligned read
    add     r9, rdi                    # 1 cycle - get original dst
    and     r9, ALIGN_MASK             # 1 cycle - align to 8-byte boundary
    mov     [r12 + r13 * 8], r9        # ~4 cycles - write dst pre-address to trace

    test    rax, DOUBLE_SRC_PRE_MASK
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
    test    rax, POST_COUNT_MASK       # 1 cycle - check if post_count > 0
    jz      .L_src_to_mops               # 2 cycles (predicted taken) - skip to src copy

.L_post_is_active:
    mov     rcx, MOPS_ALIGNED_READ     # rcx = flags aligned read
    lea     r9, [rdi + rdx - 1]        # 1 cycle - r9 = dst + count - 1 (last dst byte)
    and     r9, ALIGN_MASK             # 1 cycle - align to 8-byte boundary
    add     r9, rcx                    # 1 cycle - r9 mops with dst aligned address
    mov     [r12 + r13 * 8], r9        # ~4 cycles - write dst post-value to trace

    mov     r9, rax
    shr     r9, PRE_AND_LOOP_BYTES_SBITS
    add     r9, rsi
    and     r9, ALIGN_MASK

    test    rax, DOUBLE_SRC_POST_MASK
    jnz     .L_post_double_src_to_mops

.L_post_single_src_to_mops:

    mov     rcx, MOPS_ALIGNED_READ     # r9 = flags aligned read
    add     r9, rcx                    # 1 cycle - get original src
    mov     [r12 + r13 * 8 + 8], r9    # ~4 cycles - write src address
    jmp     .L_post_src_inc_mops_index

.L_post_double_src_to_mops:

    mov     rcx, MOPS_ALIGNED_READ_2W  # r9 = flags aligned read
    add     r9, rcx                    # 1 cycle - get original src
    mov     [r12 + r13 * 8 + 8], r9    # ~4 cycles - write src address

.L_post_src_inc_mops_index:
    add     r13, 2                     # add 2 (pre-write, block single/dual src)

.L_src_to_mops:
    mov     rcx, rax                       # 1 cycle - rcx = encoded
    shr     rcx, LOOP_COUNT_SBITS          # 1 cycle - rcx = loop (32 bits)
    jz      .L_save_dst_with_loop_count_zero
    shl     rcx, MOPS_BLOCK_WORDS_SBITS    # 1 cycle - rcx = loop | 0 (4 bits) | (32 bits)

    test    rax, UNALIGNED_DST_SRC_MASK
    jnz     .L_src_extra_for_unaligned_loop

    mov     r9, MOPS_ALIGNED_BLOCK_READ    # 1 cycle - r9 = read block
    jmp     .L_src_block_before_address

.L_src_extra_for_unaligned_loop:    
    mov     r9, MOPS_ALIGNED_BLOCK_READ + MOPS_BLOCK_ONE_WORD

.L_src_block_before_address:
    add     r9, rcx
    test    rax, SRC64_INC_BY_PRE_MASK
    jnz     .L_src_incr_by_pre
    add     r9, rsi                        # 1 cycle - rcx = first block src address
    jmp     .L_src_to_mops_ready

.L_src_incr_by_pre: 
    lea     r9, [rsi + r9 + 8]

.L_src_to_mops_ready:
    and     r9, ALIGN_MASK
    mov     [r12 + r13 * 8], r9            # ~4 cycles - write first block src read address
    inc     r13

.L_save_dst_addr_reusing_rcx:
  
    mov     r9, rax                        # rcx = encoded
    and     r9, PRE_WRITES_MASK            # rcx = pre_writes mask
    shl     r9, PRE_WRITES_TO_MOPS_BLOCK   # rcx = pre_writes offset (with correct shift)
    add     r9, rcx
    add     r9, rdi

    mov     rcx, MOPS_ALIGNED_BLOCK_WRITE
    add     r9, rcx
    and     r9, ALIGN_MASK

    mov     [r12 + r13 * 8], r9
    inc     r13
    jmp     .L_mops_done

.L_save_dst_with_loop_count_zero:
    mov     r9, rax                        # rcx = encoded
    and     r9, PRE_WRITES_MASK            # rcx = pre_writes mask
    shl     r9, PRE_WRITES_TO_MOPS_BLOCK   # rcx = pre_writes offset (with correct shift)
    add     r9, rdi

    mov     rcx, MOPS_ALIGNED_BLOCK_WRITE
    add     r9, rcx
    and     r9, ALIGN_MASK

    mov     [r12 + r13 * 8], r9
    inc     r13

.L_mops_done:  

    # Check for memory overlap to decide copy direction
    # NOTE: rdi and rsi contain their ORIGINAL values (not modified in mops section)
    # Overlap exists if: src < dst < src+count (forward overlap)
    cmp     rdi, rsi                # 1 cycle - compare dst with src
    jb      .L_copy_forward           # 2 cycles (predicted) - dst < src, no overlap
    lea     r9, [rsi + rdx]         # 1 cycle - r9 = src + count
    cmp     rdi, r9                 # 1 cycle - compare dst with (src+count)
    jae     .L_copy_forward           # 2 cycles (predicted) - dst >= src+count, no overlap
    
    # Overlap detected (src < dst < src+count), must copy backward
    # Setup: rsi = src+count-1, rdi = dst+count-1, rcx = count
    # Uses ORIGINAL rsi and rdi values (not modified during mops recording)
    
    lea     rsi, [rsi + rdx - 1]    # 1 cycle - rsi = src + count - 1 (from original)
    lea     rdi, [rdi + rdx - 1]    # 1 cycle - rdi = dst + count - 1 (from original)
    mov     rcx, rdx                # 1 cycle - rcx = count

    std                             # ~20-50 cycles - set DF (serializing, pipeline flush)
    rep movsb                       # ~3-5 cycles per byte (backward copy, slower than forward)
    cld                             # ~20-50 cycles - clear DF (serializing, pipeline flush)

    ret                             # ~5 cycles

.L_copy_forward:
    # No overlap detected, perform optimized forward copy
    cmp      rdx, 16                  # 1 cycle - check if count >= 16 (worth alignment)
    jae      .L_copy_forward_pre        # 2 cycles (predicted) - use 3-phase aligned copy

    # Small copy (count < 16): copy all bytes directly
    mov     rcx, rdx                # 1 cycle - rcx = count
    rep movsb                       # ~3-5 cycles per byte (unaligned small copy)

    ret                             # ~5 cycles

.L_copy_forward_pre:
    # Copy in 3 phases: pre-alignment bytes, aligned qwords, post-alignment bytes
    # If pre_count > 0, copy unaligned prefix bytes
    test    rax, PRE_COUNT_MASK     # 1 cycle - check if pre_count > 0
    jz      .L_copy_forward_loop      # 2 cycles (predicted)

    # Extract and copy pre_count bytes (1-7 bytes to reach alignment)
    mov     rcx, rax                # 1 cycle
    and     rcx, PRE_COUNT_MASK     # 1 cycle - rcx = pre_count (bits 0-3)

    rep     movsb                   # ~3-5 cycles per byte
                                    # rsi, rdi now 8-byte aligned

.L_copy_forward_loop:
    # Copy aligned qwords (main bulk of data)
    mov     rcx, rax                # 1 cycle
    shr     rcx, LOOP_COUNT_SBITS   # 1 cycle - rcx = loop_count (bits 32-63)
    rep     movsq                   # ~1.5-2 cycles per qword (aligned, optimized)
                                    # rsi, rdi advanced by loop_count * 8

.L_check_forward_post:

    # If post_count > 0, copy remaining unaligned suffix bytes
    test    rax, POST_COUNT_MASK    # 1 cycle - check if post_count > 0
    jz      .L_done                   # 2 cycles (predicted)

    # Extract and copy post_count bytes (1-7 bytes after aligned data)
    mov     rcx, rax                # 1 cycle
    shr     rcx, POST_COUNT_SBITS   # 1 cycle - shift post_count to position
    and     rcx, 0x07               # 1 cycle - rcx = post_count (bits 43-45)

    rep     movsb                   # ~3-5 cycles per byte
                                    # rsi, rdi now point past end of data

.L_done:
    ret                             # ~5 cycles

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
