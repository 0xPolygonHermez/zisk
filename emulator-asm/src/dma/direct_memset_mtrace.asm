.intel_syntax noprefix
.code64

################################################################################
# direct_dma_xmemset_mtrace - Memory set with mtrace (memory trace) recording
#
# This function fills a memory region with a byte value while recording
# the operation encoding and pre-values to the mtrace buffer for verification.
#
# MAIN TASKS:
# 1. Encode memset metadata (dst_offset, count, fill_byte, alignment info)
# 2. Record pre-values of partial qwords (before overwriting)
# 3. Perform the actual memset operation (via fast_memset)
#
# MTRACE SIZE:
# xmemset uses at most 3 qwords: encode + pre + post
# Therefore, no realloc check is needed (always fits within threshold)
#
# REGISTER USAGE:
#   Uses: rax, rcx, rdx, rdi, rsi, r9, r12, r13
#   Does NOT use XMM registers (caller doesn't need to save them)
#   Modifies: r13 (mtrace index output)
#
# PARAMETERS (non-standard ABI):
#   rdi = dst (u64)                     - Destination address to fill
#   rsi = value (u8 in low byte)        - Byte value to set (0-255)
#   rdx = count (usize)                 - Number of bytes to set
#   r12 = mtrace buffer base address    - Base pointer to mtrace buffer
#   r13 = mtrace buffer index           - Current index (updated on return)
#
# RETURN:
#   r13 = Updated mtrace index
#
# BRANCHES:
#   FAST: dst aligned + count multiple of 8 → encode only (no pre-reads)
#   BRANCH 1: dst aligned + count NOT multiple of 8 → encode + 1 post pre-read
#   BRANCH 2: dst unaligned → encode + 0-2 pre-reads depending on alignment
################################################################################

.global direct_dma_xmemset_mtrace
.extern fast_memset

.include "dma_constants.inc"
.include "fast_dma_encode_macro.inc"

.section .text

################################################################################
# direct_dma_xmemset_mtrace - Direct entry point (non-standard ABI)
#
# Called directly from generated assembly code without ABI overhead.
# More efficient when caller manages register preservation.
#
# PARAMETERS:
#   rdi = destination address
#   rsi = byte value (0-255)
#   rdx = byte count
#   r12 = mtrace buffer base
#   r13 = mtrace buffer index (input/output)
################################################################################

direct_dma_xmemset_mtrace:
   
    # Modified registers (caller must handle): 
    #   r9  = scratch for calculations
    #   rcx = scratch for address/value storage
    #   r13 = mtrace index (updated on return)

    # Early exit path if count = 0
    test    rdx, rdx
    jz      .L_xmemset_mtrace_count_zero

    # Check if dst is 8-byte aligned
    test    rdi, 0x7
    jnz     .L_xmemset_mtrace_rdi_unaligned

    # Check if count is multiple of 8
    test    rdx, 0x07
    jnz     .L_memset_mtrace_count_remain

    # ========== FAST PATH ==========
    # dst is aligned AND count is multiple of 8
    # => No partial qwords, no pre-reads needed, only encoding

    # Encode loop_bytes (count is already multiple of 8)
    mov     r9, rdx
    shl     r9, DMA_PRE_AND_LOOP_BYTES_RS     # 1 cycle - shift to loop_bytes position

    # Encode fill byte
    movzx   eax, sil                      # 1 cycle - zero-extend byte value
    shl     rax, DMA_FILL_BYTE_RS         # 1 cycle - shift to fill_byte position
    add     rax, r9                       # 1 cycle - combine

    # Store encoded value to mtrace
    mov     [r12 + r13 * 8], rax          # ~4 cycles - write encoding
    inc     r13                           # 1 cycle - advance mtrace index

    jmp     fast_memset                   # tail call to fast_memset

    # ========== BRANCH 1 ==========
    # dst aligned, count NOT multiple of 8
    # => Need 1 post pre-read (last partial qword)

.L_memset_mtrace_count_remain:
    # dst_offset = 0

    movzx   r9, sil                      # 1 cycle - zero-extend byte value
    shl     r9, DMA_FILL_BYTE_RS         # 1 cycle - shift to fill_byte position

    FAST_DMA_ENCODE_NO_SRC

    # Encode post_count (remaining bytes after aligned portion)
    add     rax, r9

    # Store encoding to mtrace
    mov     [r12 + r13 * 8], rax          # ~4 cycles

    # Calculate qword count for post pre-read address

    shr     rax, DMA_LOOP_COUNT_RS
    mov     rcx, [rdi + rax * 8]          # 1 cycle - rcx = post qword address
    mov     [r12 + r13 * 8 + 8], rcx      # ~4 cycles - store post pre-read address
    add     r13, 2                        # 1 cycle - advance index by 2

    jmp     fast_memset                   # tail call to fast_memset

    # ========== BRANCH 2 ==========
    # dst NOT aligned - uses full FAST_DMA_ENCODE macro
    # Depending on alignment, may need 0, 1, or 2 pre-reads

.L_xmemset_mtrace_rdi_unaligned:

    # Use macro for complex encoding (handles all alignment cases)
    FAST_DMA_ENCODE_NO_SRC                 # ~15-20 cycles
    
    # Add fill byte to encoding
    movzx   r9, sil                       # 1 cycle - zero-extend byte value  
    shl     r9, DMA_FILL_BYTE_RS          # 1 cycle - shift to position
    or      rax, r9                       # 1 cycle - combine with encoding

    # Store encoding to mtrace
    mov     [r12 + r13 * 8], rax          # ~4 cycles

    # Check if PRE pre-read needed (unaligned start)
    test    rax, DMA_PRE_COUNT_MASK           # 1 cycle
    jz      .L_xmemset_mtrace_rdi_unaligned_no_pre  # 2 cycles (predicted)

    # PRE pre-read: save original value of first partial qword
    mov     r9, rdi
    and     r9, ALIGN_MASK                # 1 cycle - r9 = aligned dst
    mov     rcx, [r9]                     # ~4 cycles - read current value
    mov     [r12 + r13 * 8 + 8], rcx      # ~4 cycles - store pre-value

    # Check if POST pre-read also needed (unaligned end)
    test    rax, DMA_POST_COUNT_MASK          # 1 cycle
    jz      .L_xmemset_mtrace_rdi_unaligned_pre_no_post  # 2 cycles

    # POST pre-read: save original value of last partial qword
    # r9 still contains (dst & ALIGN_MASK) from previous calculation
    # Calculate post qword address: aligned_dst + 8 + loop_count * 8
    
    mov     rcx, rax
    shr     rcx, DMA_PRE_AND_LOOP_BYTES_RS

    mov     rcx, [rdi + rcx]               # ~4 cycles - read post pre-value
    mov     [r12 + r13 * 8 + 16], rcx      # ~4 cycles - store as third mtrace entry
    add     r13, 3                         # 1 cycle - advance by 3 (encode + pre + post)

    jmp     fast_memset                    # tail call to fast_memset

    # ----- BRANCH 2.1: PRE only (no POST) -----
    # Unaligned start but aligned end
.L_xmemset_mtrace_rdi_unaligned_pre_no_post:
    add     r13, 2                         # 1 cycle - advance by 2 (encode + pre)

    jmp     fast_memset                    # tail call to fast_memset

    # ----- BRANCH 2.2: NO PRE (start happens to be aligned) -----
    # When unaligned path was taken but PRE=0 (edge case)
.L_xmemset_mtrace_rdi_unaligned_no_pre:
    # Check if POST pre-read needed
    test    rax, DMA_POST_COUNT_MASK           # 1 cycle
    jz      .L_xmemset_mtrace_rdi_unaligned_no_pre_no_post  # 2 cycles

    # Calculate aligned dst for post address
    mov     r9, rdi                        # 1 cycle
    and     r9, ALIGN_MASK                 # 1 cycle - r9 = aligned dst

    # Extract loop_count to calculate post address
    mov     rcx, rax                       # 1 cycle
    shr     rcx, DMA_LOOP_COUNT_RS             # 1 cycle - rcx = loop_count

    # POST pre-read: save original value of last qword
    # Address: aligned_dst + 8 + loop_count * 8
    mov     rcx, [r9 + 8 + rcx * 8]        # ~4 cycles - read post pre-value
    mov     [r12 + r13 * 8 + 8], rcx       # ~4 cycles - store as second mtrace entry
    add     r13, 2                         # 1 cycle - advance by 2 (encode + post)

    jmp     fast_memset                    # tail call to fast_memset

    # ----- BRANCH 2.3: NO PRE, NO POST -----
    # Edge case where both ends happen to be aligned despite taking unaligned path
.L_xmemset_mtrace_rdi_unaligned_no_pre_no_post:
    inc     r13                            # 1 cycle - advance by 1 (encode only)

    jmp     fast_memset                    # tail call to fast_memset

    # ========== COUNT = 0 CASE ==========
    # Zero-length memset: only encoding, no pre-reads needed
    # Creates minimal mtrace entry for zero-byte operation
.L_xmemset_mtrace_count_zero:

    FAST_DMA_ENCODE_COUNT_ZERO 0

    # Encode fill byte
    movzx   r9, sil                       # 1 cycle - zero-extend byte value
    shl     r9, DMA_FILL_BYTE_RS          # 1 cycle - shift to position

    # Add template for MEMSET_ZERO operation type
    add     rax, r9                      # 1 cycle


    # Store encoding to mtrace (no pre-reads for zero-length)
    mov     [r12 + r13 * 8], rax           # ~4 cycles
    inc     r13                            # 1 cycle

    jmp     fast_memset                    # tail call to fast_memset

    # NOTE: This label is unreachable - all paths use tail calls to fast_memset
 

# Performance Estimate (Modern x86-64, Intel Skylake/AMD Zen+, L1 cache hits):
#
# MEMSET OPERATION WITH MTRACE RECORDING:
# - FAST_DMA_ENCODE macro:          ~15-20 cycles (logic + table lookup)
# - Encoding store:                 ~4 cycles (mov to mtrace buffer)
# - Pre pre-read (if needed):       ~8-10 cycles (and + mov load + mov store)
# - Post pre-read (if needed):      ~10-12 cycles (lea + mov load + mov store)
# - Fill byte insertion:            ~3 cycles (movzx + shl + or)
# - Tail call jump:                 ~1-2 cycles
#
# PATH TIMING:
#
# FAST PATH (aligned dst, count % 8 == 0):
#   7 (setup) + 3 (fill byte) + 4 (store) + 8 (post pre-read) + 2 (tail)
#   = ~24 cycles overhead + fast_memset execution
#
# BRANCH 1 (aligned dst, count % 8 != 0):
#   15 (encode) + 3 (fill byte) + 4 (store) + 8 (post pre-read) + 2 (tail)
#   = ~32 cycles overhead + fast_memset execution
#
# BRANCH 2.1 (unaligned dst, PRE only):
#   20 (encode) + 3 (fill byte) + 4 (store) + 10 (pre pre-read) + 2 (tail)
#   = ~39 cycles overhead + fast_memset execution
#
# BRANCH 2.2 (unaligned dst, PRE + POST):
#   20 (encode) + 3 (fill byte) + 4 (store) + 10 (pre) + 12 (post) + 2 (tail)
#   = ~51 cycles overhead + fast_memset execution
#
# NOTES:
# - Mtrace overhead is independent of fill size (constant per operation)
# - Pre-reads capture original values for later verification
# - All paths use tail calls, minimizing return overhead
# - Encoding + pre-reads add ~24-51 cycles vs direct fast_memset call
# - Actual fill performance depends on fast_memset (ERMSB ~0.5 cycles/qword)

# Mark stack as non-executable (required by modern linkers)
.section .note.GNU-stack,"",%progbits
