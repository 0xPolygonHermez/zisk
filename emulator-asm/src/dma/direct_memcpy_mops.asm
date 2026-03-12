.intel_syntax noprefix
.code64

################################################################################
# direct_memcpy_mops / direct_xmemcpy_mops - Memory copy with mops tracing
#
# These functions perform memory copy operations while recording all memory
# operation addresses (mops) for verification. Two variants exist:
#
# - memcpy_mops:  Records an EXTRA_PARAMETER_ADDR read (count comes from memory)
# - xmemcpy_mops: Extended variant where count is passed directly (no extra read)
#
# MAIN TASKS:
# 1. Encode memcpy metadata (offsets, counts, alignment flags)
# 2. Record all memory operation addresses (reads and writes) to mops buffer
# 3. Perform the actual memory copy from src to dst (with overlap handling)
#
# REGISTER USAGE:
#   Uses: rax, rcx, rdx, rdi, rsi, r9, r12, r13
#   Does NOT use XMM registers (caller doesn't need to save them)
#   Modifies: r13 (mops index output)
#
# PARAMETERS (non-standard ABI):
#   rdi = dst (u64)                   - Destination address
#   rsi = src (u64)                   - Source address  
#   rdx = count (usize)               - Number of bytes to copy
#   r12 = mops buffer base address    - Base pointer to memory ops buffer
#   r13 = mops buffer index           - Current index (updated on return)
#
# RETURN:
#   r13 = Updated mops index (number of entries written)
#
# MEMORY COPY BEHAVIOR:
# - Handles overlapping src/dst correctly (like memmove)
# - Non-overlapping: optimized 3-phase copy (pre/loop/post alignment)
# - Overlapping: backward byte-by-byte copy to avoid corruption
################################################################################

.global direct_dma_memcpy_mops
.global direct_dma_xmemcpy_mops
.extern check_dynamic_mtrace

.include "dma_constants.inc"
.include "fast_dma_encode_macro.inc"

.section .text

################################################################################
# direct_dma_xmemcpy_mops - Fast memory copy with mops (extended variant)
#
# Direct entry point for generated code (non-standard ABI). The extended
# variant receives count in rdx, so no extra memory read is recorded.
#
# PARAMETERS (non-standard ABI):
#   rdi = destination address
#   rsi = source address
#   rdx = byte count
#   r12 = mops buffer base address
#   r13 = mops buffer index (input/output)
#
# RETURN:
#   r13 = updated mops index
################################################################################

direct_dma_xmemcpy_mops:

    # Modified registers (no save needed - caller expects these to change):
    #   r9  = scratch register
    #   rcx = scratch register
    #   rdi = advanced during copy
    #   rsi = advanced during copy
    #   r13 = updated mops index (output)
    #   rax = encoded metadata

    # Encode memcpy parameters: rdi=dst, rsi=src, rdx=count
    FAST_DMA_ENCODE  # ~15-20 cycles - table lookup encoding

    # Skip the EXTENDED_PARAM read entry (not needed for xmemcpy)
    jmp     direct_dma_xmemcpy_common_entry_point

################################################################################
# direct_dma_memcpy_mops - Fast memory copy with mops (standard variant)
#
# Direct entry point for generated code (non-standard ABI). Records an extra
# memory read from EXTENDED_PARAM address because the memcpy opcode reads
# count from that location.
#
# PARAMETERS (non-standard ABI):
#   rdi = destination address
#   rsi = source address
#   rdx = byte count
#   r12 = mops buffer base address
#   r13 = mops buffer index (input/output)
#
# RETURN:
#   r13 = updated mops index
################################################################################

direct_dma_memcpy_mops:
   
    # Modified registers (no save needed - caller expects these to change):
    #   r9  = scratch register
    #   rcx = scratch register  
    #   rdi = advanced during copy
    #   rsi = advanced during copy
    #   r13 = updated mops index (output)
    #   rax = encoded metadata

    # Encode memcpy parameters: rdi=dst, rsi=src, rdx=count
    FAST_DMA_ENCODE            # ~15-20 cycles - table lookup encoding

    # Record EXTENDED_PARAM read (memcpy opcode reads count from this address)
    mov     r9, (MOPS_ALIGNED_READ + EXTRA_PARAMETER_ADDR)  # 1 cycle
    mov     [r12 + r13 * 8], r9                             # ~4 cycles
    inc     r13                                             # 1 cycle

direct_dma_xmemcpy_common_entry_point:

    # Early exit if count is zero
    test    rdx, rdx                   # 1 cycle - check count
    jz      .L_done                    # 2 cycles (predicted) - nothing to copy

    # ========== PHASE 1: Record PRE-alignment memory operations ==========

.L_pre_dst_to_mops:
    # Check if pre_count > 0 (unaligned prefix bytes to copy)
    test    rax, DMA_PRE_COUNT_MASK        # 1 cycle - check pre_count bits
    jz      .L_post_dst_to_mops        # 2 cycles (predicted) - skip if aligned

.L_pre_is_active:
    # Pre-alignment read: record dst read (original value before overwrite)
    mov     r9, MOPS_ALIGNED_READ      # 1 cycle - read operation flag
    add     r9, rdi                    # 1 cycle - add dst address
    and     r9, ALIGN_MASK             # 1 cycle - align to 8-byte boundary
    mov     [r12 + r13 * 8], r9        # ~4 cycles - write mops entry

    # Check if source spans two qwords (unaligned causing double read)
    test    rax, DMA_DOUBLE_SRC_PRE_MASK   # 1 cycle
    jnz     .L_pre_double_src_to_mops  # 2 cycles (predicted)

.L_pre_single_src_to_mops:
    # Source fits in single qword
    mov     r9, MOPS_ALIGNED_READ      # 1 cycle - single read flag
    add     r9, rsi                    # 1 cycle - add src address
    and     r9, ALIGN_MASK             # 1 cycle - align to 8-byte boundary
    mov     [r12 + r13 * 8 + 8], r9    # ~4 cycles - write mops entry
    jmp     .L_pre_src_inc_mops_index  # 2 cycles

.L_pre_double_src_to_mops:
    # Source spans two qwords (needs double read)
    mov     r9, MOPS_ALIGNED_READ_2W   # 1 cycle - double read flag
    add     r9, rsi                    # 1 cycle - add src address
    and     r9, ALIGN_MASK             # 1 cycle - align to 8-byte boundary
    mov     [r12 + r13 * 8 + 8], r9    # ~4 cycles - write mops entry

.L_pre_src_inc_mops_index:
    add     r13, 2                     # 1 cycle - advance index (dst + src entries)

    # ========== PHASE 2: Record POST-alignment memory operations ==========

.L_post_dst_to_mops:
    # Check if post_count > 0 (unaligned suffix bytes to copy)
    test    rax, DMA_POST_COUNT_MASK       # 1 cycle - check post_count bits
    jz      .L_src_to_mops             # 2 cycles (predicted) - skip if no suffix

.L_post_is_active:
    # Post-alignment read: record dst read at end of copy region
    mov     rcx, MOPS_ALIGNED_READ     # 1 cycle - read operation flag
    lea     r9, [rdi + rdx - 1]        # 1 cycle - r9 = last dst byte address
    and     r9, ALIGN_MASK             # 1 cycle - align to 8-byte boundary
    add     r9, rcx                    # 1 cycle - add mops flags
    mov     [r12 + r13 * 8], r9        # ~4 cycles - write mops entry

    # Calculate source address for post-alignment bytes
    mov     r9, rax                    # 1 cycle
    shr     r9, DMA_PRE_AND_LOOP_BYTES_RS  # 1 cycle - extract pre+loop byte offset
    add     r9, rsi                    # 1 cycle - add to source
    and     r9, ALIGN_MASK             # 1 cycle - align to 8-byte boundary

    # Check if source spans two qwords
    test    rax, DMA_DOUBLE_SRC_POST_MASK  # 1 cycle
    jnz     .L_post_double_src_to_mops # 2 cycles (predicted)

.L_post_single_src_to_mops:
    # Source fits in single qword
    mov     rcx, MOPS_ALIGNED_READ     # 1 cycle - single read flag
    add     r9, rcx                    # 1 cycle - add mops flags
    mov     [r12 + r13 * 8 + 8], r9    # ~4 cycles - write mops entry
    jmp     .L_post_src_inc_mops_index # 2 cycles

.L_post_double_src_to_mops:
    # Source spans two qwords (needs double read)
    mov     rcx, MOPS_ALIGNED_READ_2W  # 1 cycle - double read flag
    add     r9, rcx                    # 1 cycle - add mops flags
    mov     [r12 + r13 * 8 + 8], r9    # ~4 cycles - write mops entry

.L_post_src_inc_mops_index:
    add     r13, 2                     # 1 cycle - advance index (dst + src entries)

    # ========== PHASE 3: Record LOOP (aligned bulk) memory operations ==========

.L_src_to_mops:
    # Extract loop_count (number of aligned qwords to copy)
    mov     rcx, rax                   # 1 cycle - rcx = encoded
    shr     rcx, DMA_LOOP_COUNT_RS         # 1 cycle - rcx = loop_count
    jz      .L_save_dst_with_loop_count_zero  # 2 cycles - no aligned bulk
    shl     rcx, MOPS_BLOCK_WORDS_RS   # 1 cycle - format for mops block entry

    # Check if source is unaligned (needs extra word per iteration)
    test    rax, DMA_UNALIGNED_DST_SRC_MASK  # 1 cycle
    jnz     .L_src_extra_for_unaligned_loop  # 2 cycles (predicted)

    mov     r9, MOPS_ALIGNED_BLOCK_READ    # 1 cycle - aligned block read flag
    jmp     .L_src_block_before_address    # 2 cycles

.L_src_extra_for_unaligned_loop:    
    # Unaligned source requires one extra word per block
    mov     r9, MOPS_ALIGNED_BLOCK_READ + MOPS_BLOCK_ONE_WORD  # 1 cycle

.L_src_block_before_address:
    add     r9, rcx                        # 1 cycle - add block word count
    test    rax, DMA_SRC64_INC_BY_PRE_MASK     # 1 cycle - check pre-alignment offset
    jnz     .L_src_incr_by_pre             # 2 cycles (predicted)
    add     r9, rsi                        # 1 cycle - use base src address
    jmp     .L_src_to_mops_ready           # 2 cycles

.L_src_incr_by_pre: 
    # Source starts one qword after base (due to pre-alignment)
    lea     r9, [rsi + r9 + 8]             # 1 cycle

.L_src_to_mops_ready:
    and     r9, ALIGN_MASK                 # 1 cycle - align address
    mov     [r12 + r13 * 8], r9            # ~4 cycles - write mops entry
    inc     r13                            # 1 cycle

.L_save_dst_addr_reusing_rcx:
    # Record destination write block
    # Strategy: treat all writes as one block (cannot write same address twice per step)
  
    mov     r9, rax                        # 1 cycle - r9 = encoded
    and     r9, DMA_PRE_WRITES_MASK        # 1 cycle - extract pre_writes count
    shl     r9, PRE_WRITES_TO_MOPS_BLOCK   # 1 cycle - format for mops block
    add     r9, rcx                        # 1 cycle - add loop block count
    add     r9, rdi                        # 1 cycle - add dst base address

    mov     rcx, MOPS_ALIGNED_BLOCK_WRITE  # 1 cycle - write block flag
    add     r9, rcx                        # 1 cycle - add mops flags
    and     r9, ALIGN_MASK                 # 1 cycle - align address

    mov     [r12 + r13 * 8], r9            # ~4 cycles - write mops entry
    inc     r13                            # 1 cycle
    jmp     .L_mops_done                   # 2 cycles

.L_save_dst_with_loop_count_zero:
    # No loop iterations - pre/post writes may be consecutive (single block)

    mov     r9, rax                        # 1 cycle - r9 = encoded
    and     r9, DMA_PRE_WRITES_MASK            # 1 cycle - extract pre_writes count
    shl     r9, PRE_WRITES_TO_MOPS_BLOCK   # 1 cycle - format for mops block
    add     r9, rdi                        # 1 cycle - add dst base address

    mov     rcx, MOPS_ALIGNED_BLOCK_WRITE  # 1 cycle - write block flag
    add     r9, rcx                        # 1 cycle - add mops flags
    and     r9, ALIGN_MASK                 # 1 cycle - align address

    mov     [r12 + r13 * 8], r9            # ~4 cycles - write mops entry
    inc     r13                            # 1 cycle

    # ========== PHASE 4: Perform actual memory copy ==========

.L_mops_done:  

    # Check for memory overlap to decide copy direction
    # Overlap exists if: src < dst < src+count (forward copy would corrupt)
    cmp     rdi, rsi                # 1 cycle - compare dst with src
    jb      .L_copy_forward         # 2 cycles (predicted) - dst < src, safe
    lea     r9, [rsi + rdx]         # 1 cycle - r9 = src + count
    cmp     rdi, r9                 # 1 cycle - compare dst with (src+count)
    jae     .L_copy_forward         # 2 cycles (predicted) - dst >= src+count, safe
    
    # Overlap detected (src < dst < src+count), must copy backward
    # Setup pointers to end of regions for backward copy
    
    mov     rax, rdi
    lea     rsi, [rsi + rdx - 1]    # 1 cycle - rsi = last src byte
    lea     rdi, [rdi + rdx - 1]    # 1 cycle - rdi = last dst byte
    mov     rcx, rdx                # 1 cycle - rcx = byte count

    std                             # ~20-50 cycles - set direction flag (backward)
    rep movsb                       # ~3-5 cycles/byte (backward, slower)
    cld                             # ~20-50 cycles - clear direction flag

    ret                             # ~3 cycles

.L_copy_forward:
    # No overlap - perform optimized forward copy
    // cmp      rdx, 16                # 1 cycle - check if count >= 16
    // jae      .L_copy_forward_pre    # 2 cycles (predicted) - use 3-phase copy

    mov     rax, rdi
    # Small copy (count < 16): direct byte copy
    mov     rcx, rdx                # 1 cycle - rcx = count
    rep movsb                       # ~3-5 cycles/byte

    ret                             # ~3 cycles
/*
.L_copy_forward_pre:
    # 3-phase copy: pre-alignment bytes, aligned qwords, post-alignment bytes
    
    # Phase A: Pre-alignment bytes (0-7 bytes to reach 8-byte alignment)
    test    rax, DMA_PRE_COUNT_MASK     # 1 cycle - check if pre_count > 0
    jz      .L_copy_forward_loop    # 2 cycles (predicted)

    mov     rcx, rax                # 1 cycle
    and     rcx, DMA_PRE_COUNT_MASK     # 1 cycle - rcx = pre_count
    rep     movsb                   # ~3-5 cycles/byte - rsi/rdi now aligned

.L_copy_forward_loop:
    # Phase B: Aligned qwords (bulk data transfer)
    mov     rcx, rax                # 1 cycle
    shr     rcx, DMA_LOOP_COUNT_RS      # 1 cycle - rcx = loop_count
    rep     movsq                   # ~1.5-2 cycles/qword (optimized)

.L_check_forward_post:
    # Phase C: Post-alignment bytes (0-7 remaining bytes)
    test    rax, DMA_POST_COUNT_MASK    # 1 cycle - check if post_count > 0
    jz      .L_done                 # 2 cycles (predicted)

    mov     rcx, rax                # 1 cycle
    shr     rcx, DMA_POST_COUNT_RS  # 1 cycle - extract post_count
    and     rcx, 0x0F               # 1 cycle - mask to 3 bits
    rep     movsb                   # ~3-5 cycles/byte
*/
.L_done:
    mov    rax, rdi
    ret                             # ~3 cycles

################################################################################
# PERFORMANCE ESTIMATES (Modern x86-64, L1 cache hits)
#
# NON-OVERLAPPING FORWARD COPY:
#   - FAST_DMA_ENCODE macro:        ~15-20 cycles (table lookup)
#   - EXTENDED_PARAM entry:         ~6 cycles (memcpy variant only)
#   - Pre-alignment mops:           ~12 cycles (if pre_count > 0)
#   - Post-alignment mops:          ~12 cycles (if post_count > 0)
#   - Block src/dst mops:           ~10-15 cycles (address calc + writes)
#   - Pre-bytes copy:               ~3-5 cycles/byte (max 7 bytes)
#   - Aligned qwords copy:          ~1.5-2 cycles/qword (rep movsq)
#   - Post-bytes copy:              ~3-5 cycles/byte (max 7 bytes)
#
#   Best case (aligned, no pre/post):  ~35 cycles + ~2 cycles/qword
#   Typical case (some alignment):     ~55 cycles + ~2 cycles/qword
#
# OVERLAPPING BACKWARD COPY:
#   - Same mops overhead:           ~35-55 cycles
#   - std instruction:              ~20-50 cycles (pipeline flush)
#   - Backward byte copy:           ~3-5 cycles/byte (rep movsb)
#   - cld instruction:              ~20-50 cycles (pipeline flush)
#
#   Worst case:  ~100-150 cycles + ~4-5 cycles/byte
#
# NOTES:
#   - Assumes L1 cache hits for all memory accesses
#   - rep movsq/movsb performance varies by microarchitecture
#   - Actual cycles may vary ±20% depending on CPU model
#   - Forward aligned path is ~2-3x faster than backward path
################################################################################

# Mark stack as non-executable (required by modern linkers)
.section .note.GNU-stack,"",%progbits
