.intel_syntax noprefix
.code64

#
# memcpy_mtrace - Optimized version with memory tracing and actual copy
#
# This function performs three main tasks:
# 1. Encodes memcpy metadata (offsets, counts, flags) using fast_dma_encode
# 2. Records memory trace (pre-values and src data for verification/rollback)
# 3. Performs the actual memory copy from src to dst (with overlap handling)
#
# REGISTER USAGE:
# Uses general-purpose registers: rax, rbx, rcx, rdx, rdi, rsi, r9, r12, r13
# Does NOT use XMM registers (caller doesn't need to save them)
# Preserves callee-saved registers (rbx, r12, r13 saved/restored in wrapper)
#
# PARAMETERS (NON System V AMD64 ABI):
#   rdi = dst (u64)                   - Destination address
#   rsi = src (u64)                   - Source address
#   rdx = count (usize)               - Number of bytes to copy
#   [r12 + r13*8] = trace_ptr (u64*)  - Pointer to memory trace buffer (input/output)
#
# RETURN VALUE:
#   RAX = Number of 64-bit words written to trace buffer
#
# MEMORY TRACE FORMAT (written to trace buffer sequentially):
#   [0]      = Encoded metadata (64-bit value with offsets, counts, flags)
#   [1]      = Pre-write value at aligned(dst) IF pre_count > 0
#   [1 or 2] = Post-write value at aligned(dst+count) IF post_count > 0
#   [...]    = All aligned qwords from aligned(src) to aligned(src+count)
#
# The trace buffer captures:
# - Original destination values (for undo/verification)
# - Source data (for verification)
# - Metadata needed to reconstruct the operation
#
# MEMORY COPY BEHAVIOR:
# - Handles overlapping src/dst correctly (like memmove)
# - For non-overlapping: optimized copy using pre_count/loop_count/post_count
# - For overlapping: backward byte-by-byte copy to avoid corruption
#
# PERFORMANCE:
# - Encoding: ~15-20 cycles (function call to fast_dma_encode, table lookup)
# - Trace writes: ~4 cycles per qword write
# - Src data copy to trace: ~1.5-2 cycles per qword (rep movsq)
# - Final memcpy (non-overlap): ~1.5-2 cycles per qword (rep movsq aligned)
# - Final memcpy (overlap): ~100-150 cycles overhead + ~4-5 cycles per byte (std/rep movsb/cld)
#
# SIDE EFFECTS:
# - Modifies memory at dst (count bytes)
# - Modifies trace buffer (variable size depending on pre/post counts)
# - Preserves direction flag (cld called after any std)

.global direct_dma_memcpy_mtrace
.global dma_memcpy_mtrace
.global direct_dma_memcpy_mtrace_with_count_check

.extern fast_dma_encode
.extern trace_address_threshold


.include "dma_constants.inc"

.section .text

.set R_MT_INDEX,     r13
.set R_MT_ADDR,      r12
.set R_STEP,         r15
.set R_AUX,          r9
.set R_AUX2,         rcx  # NOTE: used by rep
.set R_SRC,          rsi  # NOTE: used by rep
.set R_DST,          rdi  # NOTE: used by rep
.set R_COUNT,        rdx
.set R_ENCODE,       rax

dma_memcpy_mtrace:
    push    R_MT_ADDR                 # ~3 cycles - save callee-saved register
    push    R_MT_INDEX                # ~3 cycles - save callee-saved register
    push    R_AUX                     # ~3 cycles - save callee-saved register
    push    rbx                       # ~3 cycles - save callee-saved register
    
    mov     R_MT_ADDR, R_COUNT        # 1 cycle - setup trace address from count
    xor     R_MT_INDEX, R_MT_INDEX    # 1 cycle - initialize trace index to 0
    call    direct_dma_memcpy_mtrace  # ~5 cycles + function cost

    mov     R_ENCODE, R_MT_INDEX      # 1 cycle - return trace index in R_ENCODE
    pop     rbx                       # ~3 cycles - restore register
    pop     R_AUX                     # ~3 cycles - restore register
    pop     R_MT_INDEX                # ~3 cycles - restore register
    pop     R_MT_ADDR                 # ~3 cycles - restore register

    ret                               # ~5 cycles

.L_memcpy_check_mtrace_available:

    # trace_address_threshold containt the address "limit" before call _realloc_trace
    # trace_address_threshold = TRACE_ADDR + trace_size - MAX_CHUNK_TRACE_SIZE

    # calculate bytes of mtrace used and verify if throw the limit

    lea     R_AUX, [R_MT_ADDR + 8 * R_MT_INDEX]           # 1 cycle - calculate address mtrace
    lea     R_AUX, [R_AUX + R_COUNT + MAX_DMA_MT_MARGIN]  # 1 cycle - calculate current mtrace bytes usage
    sub     R_AUX, [trace_address_threshold]              # ~4 cycles - bytes over threshold (can be negative)
    jc      .L_memcpy_mtrace_continue                     # 2 cycles (predicted) - negative means space available

    # check if bytes over threshold are usual for current situation on inside chunk
    # R_STEP contain number the steps to end of chunk, we need number the steps consumed

    mov     R_AUX2, CHUNK_SIZE                     # 1 cycle - load chunk size constant
    sub     R_AUX2, R_STEP                         # 1 cycle - calculate steps consumed in chunk
    imul    R_AUX2, MAX_BYTES_MTRACE_STEP          # ~3 cycles - bytes expected for consumed steps
    cmp     R_AUX2, R_AUX                          # 1 cycle - compare expected vs actual
    jae     .L_memcpy_mtrace_continue              # 2 cycles (predicted) - expected >= actual, ok

    # at this point we need to increase trace, registers R_ENCODE, R_AUX no need to save.
    push    R_COUNT                 # ~3 cycles - save general purpose registers
    push    r8                      # ~3 cycles
    push    r10                     # ~3 cycles
    push    r11                     # ~3 cycles
    push    R_SRC                   # ~3 cycles
    push    R_DST                   # ~3 cycles

    # IMPORTANT: inside call means unaligned to 16 bits

    sub     rsp, 16*16 + 8          # 1 cycle - allocate stack space for 16 XMM registers

    movaps  [rsp + 0*16], xmm0      # ~4 cycles - save XMM registers (aligned stores)
    movaps  [rsp + 1*16], xmm1      # ~4 cycles
    movaps  [rsp + 2*16], xmm2      # ~4 cycles
    movaps  [rsp + 3*16], xmm3      # ~4 cycles
    movaps  [rsp + 4*16], xmm4      # ~4 cycles
    movaps  [rsp + 5*16], xmm5      # ~4 cycles
    movaps  [rsp + 6*16], xmm6      # ~4 cycles
    movaps  [rsp + 7*16], xmm7      # ~4 cycles
    movaps  [rsp + 8*16], xmm8      # ~4 cycles
    movaps  [rsp + 9*16], xmm9      # ~4 cycles
    movaps  [rsp + 10*16], xmm10    # ~4 cycles
    movaps  [rsp + 11*16], xmm11    # ~4 cycles
    movaps  [rsp + 12*16], xmm12    # ~4 cycles
    movaps  [rsp + 13*16], xmm13    # ~4 cycles
    movaps  [rsp + 14*16], xmm14    # ~4 cycles
    movaps  [rsp + 15*16], xmm15    # ~4 cycles

    call    _realloc_trace          # ~5 cycles + function cost (~100-500 cycles)

    movaps  xmm0, [rsp + 0*16]      # ~4 cycles - restore XMM registers (aligned loads)
    movaps  xmm1, [rsp + 1*16]      # ~4 cycles
    movaps  xmm2, [rsp + 2*16]      # ~4 cycles
    movaps  xmm3, [rsp + 3*16]      # ~4 cycles
    movaps  xmm4, [rsp + 4*16]      # ~4 cycles
    movaps  xmm5, [rsp + 5*16]      # ~4 cycles
    movaps  xmm6, [rsp + 6*16]      # ~4 cycles
    movaps  xmm7, [rsp + 7*16]      # ~4 cycles
    movaps  xmm8, [rsp + 8*16]      # ~4 cycles
    movaps  xmm9, [rsp + 9*16]      # ~4 cycles
    movaps  xmm10, [rsp + 10*16]    # ~4 cycles
    movaps  xmm11, [rsp + 11*16]    # ~4 cycles
    movaps  xmm12, [rsp + 12*16]    # ~4 cycles
    movaps  xmm13, [rsp + 13*16]    # ~4 cycles
    movaps  xmm14, [rsp + 14*16]    # ~4 cycles
    movaps  xmm15, [rsp + 15*16]    # ~4 cycles
    
    add     rsp, 16*16 +8           # 1 cycle - deallocate stack space

    pop     R_DST                   # ~3 cycles - restore general purpose registers
    pop     R_SRC                   # ~3 cycles
    pop     r11                     # ~3 cycles
    pop     r10                     # ~3 cycles
    pop     r8                      # ~3 cycles
    pop     R_COUNT                 # ~3 cycles

    jmp    .L_memcpy_mtrace_continue

direct_dma_memcpy_mtrace_with_count_check:

    # Call fast_dma_encode to calculate encoding
    # Parameters already in correct registers: R_DST=dst, R_SRC=src, R_COUNT=count
    # Result will be returned in R_ENCODE (encoded value)

    cmp     R_COUNT, MAX_BYTES_DIRECT_MTRACE # 1 cycle - check if count exceeds direct threshold
    ja      .L_memcpy_check_mtrace_available # 2 cycles (not taken usually) - large count, check trace space

.L_memcpy_mtrace_continue:
direct_dma_memcpy_mtrace:
   
    # Call fast_dma_encode to calculate encoding
    # Parameters already in correct registers: R_DST=dst, R_SRC=src, R_COUNT=count
    # Result will be returned in R_ENCODE (encoded value)

    call    fast_dma_encode         # ~15-20 cycles - table lookup encoding
    
    mov     [R_MT_ADDR + R_MT_INDEX * 8], R_ENCODE    # ~4 cycles - write encoded result to mem trace
    inc     R_MT_INDEX                                # 1 cycle - advance R_MT_INDEX (mem trace index)
        
.L_pre_dst_to_mtrace:
    # If pre_count > 0, write aligned dst value to trace
    test    R_ENCODE, PRE_COUNT_MASK   # 1 cycle - check if pre_count > 0
    jz      .L_post_dst_to_mtrace      # 2 cycles (predicted taken)

    # Branch with pre_count > 0: save original dst value before it's overwritten
    mov     R_AUX, R_DST                         # 1 cycle - get original dst
    and     R_AUX, ALIGN_MASK                    # 1 cycle - align to 8-byte boundary
    mov     R_AUX, [R_AUX]                       # ~4 cycles - read qword from aligned dst
    mov     [R_MT_ADDR + R_MT_INDEX * 8], R_AUX  # ~4 cycles - write dst pre-value to trace
    inc     R_MT_INDEX                           # 1 cycle - advance trace index

.L_post_dst_to_mtrace:

    # If post_count > 0, write aligned (dst+count) value to trace
    test    R_ENCODE, POST_COUNT_MASK    # 1 cycle - check if post_count > 0
    jz      .L_src_to_mtrace             # 2 cycles (predicted taken) - skip to src copy

    lea     R_AUX, [R_DST + R_COUNT - 1]           # 1 cycle - R_AUX = dst + count - 1 (last dst byte)
    and     R_AUX, ALIGN_MASK                      # 1 cycle - align to 8-byte boundary
    mov     R_AUX, [R_AUX]                         # ~4 cycles - read qword at (dst+count) aligned
    mov     [R_MT_ADDR + R_MT_INDEX * 8], R_AUX    # ~4 cycles - write dst post-value to trace
    inc     R_MT_INDEX                             # 1 cycle - advance trace index
    
.L_src_to_mtrace:
    # Copy source data to trace buffer
    # Total qwords = loop_count (bits 0-31) + extra_src_reads (bits 48-50)

    mov     R_AUX2, R_ENCODE           # 1 cycle - R_AUX2 = encoded
    shr     R_AUX2, LOOP_COUNT_SBITS   # 1 cycle - R_AUX2 = loop_count (bits 32-63)
    
    mov     R_AUX, R_ENCODE               # 1 cycle - R_AUX = encoded
    shr     R_AUX, EXTRA_SRC_READS_SBITS  # 1 cycle - shift extra_src_reads to position
    and     R_AUX, 0x03                   # 1 cycle - R_AUX = extra_src_reads (bits 48-50)
    add     R_AUX2, R_AUX                 # 1 cycle - R_AUX2 = total qwords to copy

    # Setup for rep movsq: copy from aligned src to trace buffer
    mov     R_AUX, R_SRC              # 1 cycle - preserve original src pointer
    and     R_SRC, ALIGN_MASK         # 1 cycle - R_SRC = src aligned to 8 bytes

    push    R_DST                                # ~3 cycles - save dst pointer
    lea     R_DST, [R_MT_ADDR + R_MT_INDEX * 8]  # 1 cycle - R_DST = trace buffer destination
    add     R_MT_INDEX, R_AUX2                   # 1 cycle - advance trace index by qwords copied
    
    rep movsq                         # ~1.5-2 cycles per qword (hardware optimized)

    pop     R_DST                     # ~3 cycles - restore dst pointer
    mov     R_SRC, R_AUX              # 1 cycle - restore original src pointer    
    
.L_mtrace_done:    
    # Check for memory overlap to decide copy direction
    # NOTE: R_DST and R_SRC now contain their ORIGINAL values (restored above)
    # Overlap exists if: src < dst < src+count (forward overlap)
    cmp     R_DST, R_SRC              # 1 cycle - compare dst with src
    jb      .L_copy_forward           # 2 cycles (predicted) - dst < src, no overlap
    lea     R_AUX, [R_SRC + R_COUNT]  # 1 cycle - R_AUX = src + count
    cmp     R_DST, R_AUX              # 1 cycle - compare dst with (src+count)
    jae     .L_copy_forward           # 2 cycles (predicted) - dst >= src+count, no overlap
    
    # Overlap detected (src < dst < src+count), must copy backward
    # Setup: R_SRC = src+count-1, R_DST = dst+count-1, R_AUX2 = count
    # Uses ORIGINAL R_SRC and R_DST values (restored from R_AUX and stack)
    
    lea     R_SRC, [R_SRC + R_COUNT - 1]   # 1 cycle - R_SRC = src + count - 1 (from original)
    lea     R_DST, [R_DST + R_COUNT - 1]   # 1 cycle - R_DST = dst + count - 1 (from original)
    mov     R_AUX2, R_COUNT                # 1 cycle - R_AUX2 = count

    std               # ~20-50 cycles - set DF (serializing, pipeline flush)
    rep movsb         # ~3-5 cycles per byte (backward copy, slower than forward)
    cld               # ~20-50 cycles - clear DF (serializing, pipeline flush)

    ret               # ~5 cycles

.L_copy_forward:
    # No overlap detected, perform optimized forward copy
    cmp      R_COUNT, 16            # 1 cycle - check if count >= 16 (worth alignment)
    jae      .L_copy_forward_pre    # 2 cycles (predicted) - use 3-phase aligned copy

    # Small copy (count < 16): copy all bytes directly
    mov     R_AUX2, R_COUNT         # 1 cycle - R_AUX2 = count
    rep movsb                       # ~3-5 cycles per byte (unaligned small copy)

    ret                             # ~5 cycles

.L_copy_forward_pre:
    # Copy in 3 phases: pre-alignment bytes, aligned qwords, post-alignment bytes

    # If pre_count > 0, copy unaligned prefix bytes

    test    R_ENCODE, PRE_COUNT_MASK  # 1 cycle - check if pre_count > 0
    jz      .L_copy_forward_loop      # 2 cycles (predicted)

    # Extract and copy pre_count bytes (1-7 bytes to reach alignment)

    mov     R_AUX2, R_ENCODE          # 1 cycle
    and     R_AUX2, PRE_COUNT_MASK    # 1 cycle - R_AUX2 = pre_count (bits 0-3)

    rep     movsb                     # ~3-5 cycles per byte
                                      # R_SRC, R_DST now 8-byte aligned

.L_copy_forward_loop:
    # Copy aligned qwords (main bulk of data)
    mov     R_AUX2, R_ENCODE          # 1 cycle
    shr     R_AUX2, LOOP_COUNT_SBITS  # 1 cycle - R_AUX2 = loop_count (bits 32-63)
    rep     movsq                     # ~1.5-2 cycles per qword (aligned, optimized)
                                      # R_SRC, R_DST advanced by loop_count * 8

.L_check_forward_post:

    # If post_count > 0, copy remaining unaligned suffix bytes
    test    R_ENCODE, POST_COUNT_MASK  # 1 cycle - check if post_count > 0
    jz      .L_done                    # 2 cycles (predicted)

    # Extract and copy post_count bytes (1-7 bytes after aligned data)
    mov     R_AUX2, R_ENCODE           # 1 cycle
    shr     R_AUX2, POST_COUNT_SBITS   # 1 cycle - shift post_count to position
    and     R_AUX2, 0x07               # 1 cycle - R_AUX2 = post_count (bits 43-45)

    rep     movsb                      # ~3-5 cycles per byte
                                       # R_SRC, R_DST now point past end of data

.L_done:
    ret                                # ~5 cycles

# Performance estimate (Modern x86-64, L1 cache hits):
#
# NON-OVERLAPPING FORWARD COPY PATH:
# - fast_dma_encode call:           ~15-20 cycles (function call + table lookup)
# - Write encoding to trace:        ~4 cycles
# - Pre-value trace (conditional):  ~12 cycles (if pre_count > 0)
# - Post-value trace (conditional): ~12 cycles (if post_count > 0)
# - Source data to trace:           ~1.5-2 cycles per qword (rep movsq)
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
# - Same trace overhead:            ~30-50 cycles
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
