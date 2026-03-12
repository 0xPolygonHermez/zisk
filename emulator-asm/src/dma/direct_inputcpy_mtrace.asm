.intel_syntax noprefix
.code64

#
# inputcpy_mtrace - Optimized version with memory tracing and actual copy
#
# This function performs three main tasks:
# 1. Encodes inputcpy metadata (offsets, counts, flags) using fast_dma_encode
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
#   rsi = reserved                    - Input source
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
# - Final inputcpy (non-overlap): ~1.5-2 cycles per qword (rep movsq aligned)
# - Final inputcpy (overlap): ~100-150 cycles overhead + ~4-5 cycles per byte (std/rep movsb/cld)
#
# SIDE EFFECTS:
# - Modifies memory at dst (count bytes)
# - Modifies trace buffer (variable size depending on pre/post counts)
# - Preserves direction flag (cld called after any std)

.global direct_dma_inputcpy_mtrace
.global direct_dma_inputcpy_mtrace_with_count_check

.extern trace_address_threshold
.extern fcall_ctx
.extern fast_memcpy
.extern fast_memcpy64

.include "dma_constants.inc"
.include "fast_dma_encode_macro.inc"

.section .text

.set R_MT_INDEX,     r13
.set R_MT_ADDR,      r12
.set R_STEP,         r14
.set R_AUX,          r9
.set R_AUX2,         rcx  # NOTE: used by rep
.set R_SRC,          rsi  # NOTE: used by rep
.set R_DST,          rdi  # NOTE: used by rep
.set R_COUNT,        rdx
.set R_ENCODE,       rax


# DIRECT CALL
# RDI = DST
# RSI = RESERVED(INPUT)
# RDX = COUNT
# RCX = TRACE

direct_dma_inputcpy_mtrace_with_count_check:

    # Call fast_dma_encode to calculate encoding
    # Parameters already in correct registers: R_DST=dst, R_SRC=src, R_COUNT=count
    # Result will be returned in R_ENCODE (encoded value)

    cmp     R_COUNT, MAX_DMA_BYTES_DIRECT_MTRACE  # 1 cycle - check if count exceeds direct threshold
    ja      .L_inputcpy_check_dynamic_trace       # 2 cycles (not taken usually) - large count, check trace space
    jmp     direct_dma_inputcpy_mtrace

.L_inputcpy_check_dynamic_trace:
    call    check_dynamic_mtrace

direct_dma_inputcpy_mtrace:
   
    # Call fast_dma_encode to calculate encoding
    # Parameters already in correct registers: R_DST=dst, R_SRC=src, R_COUNT=count
    # Result will be returned in R_ENCODE (encoded value)

    FAST_DMA_ENCODE_NO_SRC    
    
    mov     [R_MT_ADDR + R_MT_INDEX * 8], R_ENCODE    # ~4 cycles - write encoded result to mem trace
    inc     R_MT_INDEX                                # 1 cycle - advance R_MT_INDEX (mem trace index)
        
.L_pre_dst_to_mtrace:
    # If pre_count > 0, write aligned dst value to trace
    test    R_ENCODE, DMA_PRE_COUNT_MASK   # 1 cycle - check if pre_count > 0
    jz      .L_post_dst_to_mtrace      # 2 cycles (predicted taken)

    # Branch with pre_count > 0: save original dst value before it's overwritten
    mov     R_AUX, R_DST                         # 1 cycle - get original dst
    and     R_AUX, ALIGN_MASK                    # 1 cycle - align to 8-byte boundary
    mov     R_AUX, [R_AUX]                       # ~4 cycles - read qword from aligned dst
    mov     [R_MT_ADDR + R_MT_INDEX * 8], R_AUX  # ~4 cycles - write dst pre-value to trace
    inc     R_MT_INDEX                           # 1 cycle - advance trace index

.L_post_dst_to_mtrace:

    # If post_count > 0, write aligned (dst+count) value to trace
    test    R_ENCODE, DMA_POST_COUNT_MASK            # 1 cycle - check if post_count > 0
    jz      .L_input_to_mtrace                   # 2 cycles (predicted taken) - skip to input copy

    lea     R_AUX, [R_DST + R_COUNT - 1]         # 1 cycle - R_AUX = dst + count - 1 (last dst byte)
    and     R_AUX, ALIGN_MASK                    # 1 cycle - align to 8-byte boundary
    mov     R_AUX, [R_AUX]                       # ~4 cycles - read qword at (dst+count) aligned
    mov     [R_MT_ADDR + R_MT_INDEX * 8], R_AUX  # ~4 cycles - write dst post-value to trace
    inc     R_MT_INDEX                           # 1 cycle - advance trace index
    
.L_input_to_mtrace:
    # Copy input data to trace buffer, always aligned.
    # Total qwords = (count + 7)

    lea     R_AUX2, [R_COUNT + 7]          # 1 cycle - R_AUX2 = count + 7
    shr     R_AUX2, 3                      # 1 cycle - R_AUX2 = round_up(count/8)
    
    mov     R_AUX, qword ptr [fcall_ctx + FCALL_RESULT_GOT * 8]
    lea     R_SRC, [fcall_ctx + R_AUX * 8 + FCALL_RESULT * 8 - 8]

    push    R_DST                                # ~3 cycles - save dst pointer
    lea     R_DST, [R_MT_ADDR + R_MT_INDEX * 8]  # 1 cycle - R_DST = trace buffer destination
    add     R_MT_INDEX, R_AUX2                   # 1 cycle - advance trace index by qwords copied
    
    push    R_COUNT
    mov     R_COUNT, R_AUX2
    call    fast_memcpy64
    # rep movsq                         # ~1.5-2 cycles per qword (hardware optimized)

    pop     R_COUNT
    pop     R_DST                     # ~3 cycles - restore dst pointer
    
    mov     R_AUX2, R_COUNT
    jmp     fast_inputcpy
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
