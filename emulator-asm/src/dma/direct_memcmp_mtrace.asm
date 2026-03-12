.intel_syntax noprefix
.code64

#
# memcmp_mtrace - Memory comparison with mtrace recording
#
# This function performs two main tasks:
# 1. Encodes memcmp metadata (offsets, counts, comparison result) using FAST_DMA_ENCODE
# 2. Records memory trace entries for later verification/replay
#
# NOTE: The actual comparison is performed by fast_memcmp (called via tail jump).
# This function only handles the mtrace recording part.
#
# REGISTER USAGE:
# Uses general-purpose registers: rax, rbx, rcx, rdx, rdi, rsi, r9, r12, r13
# Does NOT use XMM registers (caller doesn't need to save them)
# Preserves callee-saved registers (rbx, r12, r13 saved/restored in wrapper)
#
# PARAMETERS (NON System V AMD64 ABI):
#   rdi = ptr1 (u64)                  - First memory region pointer
#   rsi = ptr2 (u64)                  - Second memory region pointer  
#   rdx = count (usize)               - Number of bytes to compare
#   [r12 + r13*8] = mtrace_ptr (u64*) - Pointer to mtrace buffer (input/output)
#
# RETURN VALUE:
#   rax = comparison result (0 if equal, byte difference if not)
#
# MTRACE FORMAT (written to mtrace buffer sequentially):
#   [0]      = Encoded metadata (offsets, counts, cmp_result in bits 21-28)
#   [1]      = Pre-read value at aligned(ptr1) IF pre_count > 0
#   [1 or 2] = Post-read value at aligned(ptr1+count) IF post_count > 0
#   [...]    = All aligned qwords from aligned(ptr2) to aligned(ptr2+count)
#
# The mtrace buffer captures:
# - Source data from ptr2 (for verification)
# - Comparison result encoded in metadata
# - Pre/post values for boundary alignment handling
#
# COMPARISON BEHAVIOR:
# - Does NOT modify memory (read-only operation)
# - Records all data needed to verify/replay the comparison
# - Tail-calls fast_memcmp to perform actual byte comparison
#
# PERFORMANCE:
# - FAST_DMA_ENCODE macro:     ~15-20 cycles (logic + table lookup)
# - Trace writes:               ~4 cycles per qword write
# - Src data copy to trace:     ~1-2 cycles per qword (rep movsq)
# - Function overhead:          ~10-15 cycles (branches, setup)
#
# SIDE EFFECTS:
# - Does NOT modify ptr1 or ptr2 memory (read-only comparison)
# - Modifies mtrace buffer (variable size depending on pre/post counts)
# - Advances r13 (mtrace index)

.global direct_dma_memcmp_mtrace
.global dma_memcmp_mtrace
.global _dma_memcmp_mtrace_test
// .global direct_dma_memcmp_mtrace_with_count_check
.extern fast_memcmp

# .extern trace_resize_request

.include "dma_constants.inc"
.include "fast_dma_encode_macro.inc"
.section .text

direct_dma_memcmp_mtrace:

    # First step calculate the effective count (length), because must be the length
    # really used, the other length it's only used when it's send to bus

    # First step was store the original size to mtrace + 1, because the position 0
    # will used by encoded. The function fast_memcmp modifies rdx with the effective
    # count

    test    rdx, rdx
    jz      .L_dma_memcmp_mtrace_count_zero

    # store on mtrace the bus count, original, no effective
    mov     [r12 + r13 * 8 + 8], rdx

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

    shl     r9, DMA_CMP_RES_RS               # 1 cycle - shift cmp_result to position (bits 21-28)
    or      rax, r9                           # 1 cycle - combine with encoding
    jmp     .L_dma_memcmp_encode_done         # 1 cycle

.L_fast_dma_memcmp_encode_eq:
    # Equal case: use base table (offset 0)
    FAST_DMA_ENCODE_MEMCMP 0

.L_dma_memcmp_encode_done:
    # store before a potential realloc

    mov     [r12 + r13 * 8], rax
    add     r13, 2

    # Check if count exceeds direct mtrace threshold
    # Parameters: rdi=ptr1, rsi=ptr2, rdx=count    

    cmp     rdx, MAX_DMA_BYTES_DIRECT_MTRACE  # 1 cycle - check threshold
    ja      .L_memcmp_check_dynamic_trace     # 1 cycle (not taken usually)
    jmp     .L_memcmp_mtrace_encoded_stored   # 1 cycle - fall through to main function

.L_memcmp_check_dynamic_trace:
    call    check_dynamic_mtrace              # expand mtrace buffer if needed

.L_memcmp_mtrace_encoded_stored:        

.L_pre_ptr1_to_mtrace:
    # If pre_count > 0, record aligned ptr1 value
    test    rax, DMA_PRE_COUNT_MASK       # 1 cycle - check if pre_count > 0
    jz      .L_post_ptr1_to_mtrace        # 1 cycle (predicted taken)

    # Pre-read: save qword at aligned(ptr1) for boundary handling
    mov     r9, rdi                       # 1 cycle - r9 = ptr1
    and     r9, ALIGN_MASK                # 1 cycle - align to 8-byte boundary
    mov     r9, [r9]                      # ~4 cycles - read qword from aligned ptr1
    mov     [r12 + r13 * 8], r9           # ~4 cycles - write to mtrace
    inc     r13                           # 1 cycle - advance mtrace index

.L_post_ptr1_to_mtrace:

    # If post_count > 0, record aligned (ptr1+count) value
    test    rax, DMA_POST_COUNT_MASK      # 1 cycle - check if post_count > 0
    jz      .L_ptr2_to_mtrace             # 1 cycle (predicted taken)

    lea     r9, [rdi + rdx - 1]           # 1 cycle - r9 = ptr1 + count - 1 (last byte)
    and     r9, ALIGN_MASK                # 1 cycle - align to 8-byte boundary
    mov     r9, [r9]                      # ~4 cycles - read qword at aligned(ptr1+count)
    mov     [r12 + r13 * 8], r9           # ~4 cycles - write to mtrace
    inc     r13                           # 1 cycle - advance mtrace index
    
.L_ptr2_to_mtrace:
    # Copy ptr2 (source) data to mtrace buffer for verification
    # Total qwords = loop_count + extra_src_reads

    mov     rcx, rax                      # 1 cycle - rcx = encoding
    shr     rcx, DMA_LOOP_COUNT_RS            # 1 cycle - rcx = loop_count (bits 35+)
    
    mov     r9, rax                       # 1 cycle - r9 = encoding
    shr     r9, DMA_EXTRA_SRC_READS_RS        # 1 cycle - shift to extra_src_reads
    and     r9, 0x03                      # 1 cycle - r9 = extra_src_reads (0-3)
    add     rcx, r9                       # 1 cycle - rcx = total qwords to copy

    # Setup for rep movsq: copy aligned ptr2 data to mtrace
    mov     r9, rsi                       # 1 cycle - preserve original ptr2
    and     rsi, ALIGN_MASK               # 1 cycle - rsi = ptr2 aligned to 8 bytes

    push    rdi                           # 1 cycle - save ptr1
    lea     rdi, [r12 + r13 * 8]          # 1 cycle - rdi = mtrace destination
    add     r13, rcx                      # 1 cycle - advance mtrace index
    
    rep movsq                             # ~1-2 cycles per qword (ERMSB optimized)

    pop     rdi                           # 1 cycle - restore ptr1
    mov     rsi, r9                       # 1 cycle - restore original ptr2    
    
.L_mtrace_done:    

    shr     rax, DMA_CMP_RES_RS
    test    rax, 0x100
    jnz     .L_memcmp_mtrace_negative_res
    and     rax, 0xFF
    jmp     .L_memcmp_mtrace_res_ready

.L_memcmp_mtrace_negative_res:
    or      rax, 0xFFFFFFFFFFFFFF00

.L_memcmp_mtrace_res_ready:
    ret                                # ~5 cycles

.L_dma_memcmp_mtrace_count_zero:
    # this path used if bus count is 0
    # bus_count = 0 ==> effective_count = 0
    # bus_count > 0 ==> effective_count > 0 (at least need to check first byte)

    FAST_DMA_ENCODE_COUNT_ZERO 

    mov     [r12 + r13 * 8], rax          # ~4 cycles - write encoding to mtrace

    # rdx contains 0, it's more fast use rdx rather immediate 0.

    mov     [r12 + r13 * 8 + 8], rdx      # ~4 cycles - write encoding to mtrace

    add     r13, 2                        # 1 cycle - advance mtrace index
    xor     rax, rax
    ret

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
