.intel_syntax noprefix
.code64

#
# check_dynamic_mtrace - Check if mtrace buffer needs dynamic expansion
#
# PURPOSE:
# This function determines whether the memory trace (mtrace) buffer needs to be
# dynamically expanded. It should be called whenever the data to be written to
# the trace exceeds the MAX_BYTES_DIRECT_MTRACE threshold.
#
# HOW IT WORKS:
# The function uses a two-level check to minimize overhead:
#
# 1. FAST PATH (most common): Check if current mtrace position plus required
#    bytes is below the threshold. If yes, return immediately - no expansion needed.
#
# 2. SLOW PATH: If we're past the threshold, check if this is expected based on
#    how many steps have been consumed in the current chunk. We use a worst-case
#    assumption where each step consumes MAX_BYTES_MTRACE_STEP bytes. This value
#    is slightly larger than MAX_BYTES_DIRECT_MTRACE because it includes other
#    mtrace costs like register read operations.
#
# By using worst-case assumptions, we avoid checking on every mtrace write.
# If actual usage is within expected bounds for consumed steps, no expansion
# is needed. Only when actual usage exceeds expected usage do we trigger realloc.
#
# REGISTER USAGE:
# Uses: R_MT_ADDR, R_MT_INDEX, R_STEP, R_COUNT, R_AUX, R_AUX2, R_DST, R_SRC
# Preserves: All XMM registers (saved/restored only when calling _realloc_trace)
# Preserves: r8, r10, r11, R_SRC, R_DST, R_COUNT (when calling _realloc_trace)
#
# PARAMETERS (using DMA register convention):
#   R_MT_ADDR  = Base address of mtrace buffer
#   R_MT_INDEX = Current index into mtrace buffer (in qwords)
#   R_STEP     = Steps remaining to end of chunk
#   R_COUNT    = Bytes required by current request (with margin)
#
# RETURN VALUE:
#   None (returns via ret, mtrace may have been expanded)
#
# PERFORMANCE:
# - Fast path (no expansion needed): ~8-9 cycles
# - Slow path (within expected bounds): ~15-17 cycles
# - Realloc path: ~200-600 cycles (includes XMM save/restore + _realloc_trace call)
#
# SIDE EFFECTS:
# - May trigger _realloc_trace which expands the mtrace buffer
# - Updates trace_address_threshold after realloc

.global check_dynamic_mtrace

.extern fast_dma_encode
.extern trace_address_threshold
# .extern trace_resize_request

.ifdef DEBUG
.section .data
.align 8
    dma_check_case:      .quad 0
    dma_check_step:      .quad 0
    dma_check_aux:       .quad 0
    dma_check_threshold: .quad 0
.endif

.include "dma_constants.inc"

.section .text

# REGISTER INPUTS:
#   R_MT_ADDR  = base address of mtrace buffer
#   R_MT_INDEX = current index into mtrace (qwords)
#   R_STEP     = steps remaining until end of chunk
#   R_COUNT    = bytes needed by current request

check_dynamic_mtrace:

    # FAST PATH: Check if we're below the threshold
    #
    # trace_address_threshold = TRACE_ADDR + trace_size - MAX_CHUNK_TRACE_SIZE
    # This gives us the "safe" limit before considering reallocation.
    # Calculate: current_addr + required_bytes + margin, compare with threshold.

.ifdef DEBUG
    mov     qword ptr [dma_check_case], 1
.endif

    lea     R_AUX, [R_MT_ADDR + 8 * R_MT_INDEX]           # 1 cycle - current mtrace address
    lea     R_AUX, [R_AUX + R_COUNT + MAX_DMA_MT_MARGIN]  # 1 cycle - add required bytes + safety margin
    sub     R_AUX, [trace_address_threshold]              # ~4 cycles - bytes over threshold (negative = OK)
    jnc    .L_calculate_current_margin                    # 2 cycles (predicted) - if negative, space available
    ret                                                   # FAST PATH EXIT: ~8-9 cycles total

    # SLOW PATH: Check if usage is within expected bounds for consumed steps
    #
    # Instead of checking every time, we use worst-case assumptions:
    # - Each step may consume up to MAX_BYTES_MTRACE_STEP bytes
    # - MAX_BYTES_MTRACE_STEP > MAX_BYTES_DIRECT_MTRACE (includes register reads, etc.)
    # - If actual usage <= (steps_consumed * MAX_BYTES_MTRACE_STEP), no realloc needed
    #
    # R_STEP contains steps REMAINING to end of chunk
    # steps_consumed = CHUNK_SIZE - R_STEP

.L_calculate_current_margin:

.ifdef DEBUG
    mov     qword ptr [dma_check_case], 2
    mov     [dma_check_step], R_STEP
    mov     [dma_check_aux], R_AUX
.endif

    # Calculate expected worst-case mtrace usage for consumed steps
    mov     R_AUX2, CHUNK_SIZE                     # 1 cycle - total steps per chunk
    sub     R_AUX2, R_STEP                         # 1 cycle - steps_consumed = total - remaining
    imul    R_AUX2, MAX_BYTES_MTRACE_STEP          # ~3 cycles - expected_bytes = steps * worst_case_per_step
    cmp     R_AUX2, R_AUX                          # 1 cycle - compare expected vs actual overflow
    jb     .L_call_realloc                         # 2 cycles (predicted) - if expected < actual, need realloc
    ret                                            # SLOW PATH EXIT: ~15-17 cycles total

.L_call_realloc:

    # REALLOC PATH: Actual usage exceeds expected bounds, must expand mtrace
    #
    # Save all volatile registers since _realloc_trace follows System V ABI
    # and may use any of them. We're being called from non-ABI-compliant code,
    # so we must preserve everything our caller expects.
.ifdef DEBUG
    mov     qword ptr [dma_check_case], 3
    mov     R_AUX, [trace_address_threshold]
    mov     [dma_check_threshold], R_AUX
.endif

    # Save general purpose registers used by DMA operations
    push    R_COUNT                 # 1 cycle - save count
    push    r8                      # 1 cycle
    push    r10                     # 1 cycle
    push    r11                     # 1 cycle
    push    R_SRC                   # 1 cycle - save source address
    push    R_DST                   # 1 cycle - save destination address

    # Allocate stack for XMM registers (16 registers x 16 bytes + 8 for alignment)
    # Note: We're inside a call, so stack is unaligned to 16 bytes
    sub     rsp, 16*16 + 8          # 1 cycle - allocate 264 bytes

    # Save all XMM registers (may be used by caller for optimized operations)
    movaps  [rsp + 0*16], xmm0      # 1 cycle - aligned 128-bit stores
    movaps  [rsp + 1*16], xmm1      # 1 cycle
    movaps  [rsp + 2*16], xmm2      # 1 cycle
    movaps  [rsp + 3*16], xmm3      # 1 cycle
    movaps  [rsp + 4*16], xmm4      # 1 cycle
    movaps  [rsp + 5*16], xmm5      # 1 cycle
    movaps  [rsp + 6*16], xmm6      # 1 cycle
    movaps  [rsp + 7*16], xmm7      # 1 cycle
    movaps  [rsp + 8*16], xmm8      # 1 cycle
    movaps  [rsp + 9*16], xmm9      # 1 cycle
    movaps  [rsp + 10*16], xmm10    # 1 cycle
    movaps  [rsp + 11*16], xmm11    # 1 cycle
    movaps  [rsp + 12*16], xmm12    # 1 cycle
    movaps  [rsp + 13*16], xmm13    # 1 cycle
    movaps  [rsp + 14*16], xmm14    # 1 cycle
    movaps  [rsp + 15*16], xmm15    # 1 cycle

    call    _realloc_trace          # ~5 cycles call + ~100-500 cycles function body

    # Restore all XMM registers
    movaps  xmm0, [rsp + 0*16]      # 1 cycle - aligned 128-bit loads
    movaps  xmm1, [rsp + 1*16]      # 1 cycle
    movaps  xmm2, [rsp + 2*16]      # 1 cycle
    movaps  xmm3, [rsp + 3*16]      # 1 cycle
    movaps  xmm4, [rsp + 4*16]      # 1 cycle
    movaps  xmm5, [rsp + 5*16]      # 1 cycle
    movaps  xmm6, [rsp + 6*16]      # 1 cycle
    movaps  xmm7, [rsp + 7*16]      # 1 cycle
    movaps  xmm8, [rsp + 8*16]      # 1 cycle
    movaps  xmm9, [rsp + 9*16]      # 1 cycle
    movaps  xmm10, [rsp + 10*16]    # 1 cycle
    movaps  xmm11, [rsp + 11*16]    # 1 cycle
    movaps  xmm12, [rsp + 12*16]    # 1 cycle
    movaps  xmm13, [rsp + 13*16]    # 1 cycle
    movaps  xmm14, [rsp + 14*16]    # 1 cycle
    movaps  xmm15, [rsp + 15*16]    # 1 cycle
    
    add     rsp, 16*16 +8           # 1 cycle - deallocate stack space

    # Restore general purpose registers
    pop     R_DST                   # 1 cycle - restore destination address
    pop     R_SRC                   # 1 cycle - restore source address
    pop     r11                     # 1 cycle
    pop     r10                     # 1 cycle
    pop     r8                      # 1 cycle
    pop     R_COUNT                 # 1 cycle - restore count
.L_memcpy_mtrace_continue:
    ret

.section .note.GNU-stack,"",%progbits
