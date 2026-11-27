.intel_syntax noprefix
.code64

#
# dot_bf16 - experimental exact BF16 dot product, rounded once to f32 (see
# core/src/operations/dot_bf16.rs). The work is done in C (dot_bf16.c); these wrappers adapt the
# generated code's register convention to the System V ABI, as for dot_f8.
#
# dot_bf16_fast:   computes the result and writes the block results (fast and ROM histogram
#                  generation).
# dot_bf16_mtrace: also appends to the minimal trace, at [r12 + r13*8]:
#     the first 6 qwords of the parameter block, the result, then the data as words (see
#     dot_bf16.c: x and y, or for FP4 weights the scales address, x, the scales and y).
#   It advances r13 past them, growing the trace first when they do not fit in the per-step
#   budget (check_dynamic_mtrace).
#
# REGISTERS (NON System V AMD64 ABI, as the DMA routines):
#   Input:   rdi = parameter block address; r12, r13 = minimal trace; r14 = steps left in chunk
#   Output:  rax = result (f32 bits, zero-extended)
#   Clobbers rcx, rdx, rsi, rdi, r9. The other registers the C code may change (r8, r10, r11 and
#   all xmm registers, which hold RISC-V registers in the generated code) are saved and restored;
#   rbx, rbp and r12-r15 are preserved by the C code itself.

.global dot_bf16_fast
.global dot_bf16_mtrace
.extern check_dynamic_mtrace
.extern dot_bf16_compute
.extern dot_bf16_mtrace_c

.include "dma_constants.inc"

.section .text

# Saves r8, r10, r11 and xmm0-15, and aligns the stack to 16 bytes for the C call.
.macro SAVE_FOR_C
    push    rbp
    mov     rbp, rsp
    push    r8
    push    r10
    push    r11
    and     rsp, -16
    sub     rsp, 256
    movdqa  [rsp + 0*16], xmm0
    movdqa  [rsp + 1*16], xmm1
    movdqa  [rsp + 2*16], xmm2
    movdqa  [rsp + 3*16], xmm3
    movdqa  [rsp + 4*16], xmm4
    movdqa  [rsp + 5*16], xmm5
    movdqa  [rsp + 6*16], xmm6
    movdqa  [rsp + 7*16], xmm7
    movdqa  [rsp + 8*16], xmm8
    movdqa  [rsp + 9*16], xmm9
    movdqa  [rsp + 10*16], xmm10
    movdqa  [rsp + 11*16], xmm11
    movdqa  [rsp + 12*16], xmm12
    movdqa  [rsp + 13*16], xmm13
    movdqa  [rsp + 14*16], xmm14
    movdqa  [rsp + 15*16], xmm15
.endm

# Restores what SAVE_FOR_C saved; keeps rax and rdx (the C results).
.macro RESTORE_FROM_C
    movdqa  xmm0, [rsp + 0*16]
    movdqa  xmm1, [rsp + 1*16]
    movdqa  xmm2, [rsp + 2*16]
    movdqa  xmm3, [rsp + 3*16]
    movdqa  xmm4, [rsp + 4*16]
    movdqa  xmm5, [rsp + 5*16]
    movdqa  xmm6, [rsp + 6*16]
    movdqa  xmm7, [rsp + 7*16]
    movdqa  xmm8, [rsp + 8*16]
    movdqa  xmm9, [rsp + 9*16]
    movdqa  xmm10, [rsp + 10*16]
    movdqa  xmm11, [rsp + 11*16]
    movdqa  xmm12, [rsp + 12*16]
    movdqa  xmm13, [rsp + 13*16]
    movdqa  xmm14, [rsp + 14*16]
    movdqa  xmm15, [rsp + 15*16]
    lea     rsp, [rbp - 24]
    pop     r11
    pop     r10
    pop     r8
    pop     rbp
.endm

dot_bf16_fast:
    SAVE_FOR_C
    call    dot_bf16_compute        # rdi = parameter block
    RESTORE_FROM_C
    ret

dot_bf16_mtrace:
    # Bytes this call appends to the trace, at most: 7 header qwords and n/4 words of each BF16
    # vector (flags 0), or the scales address, n/16 + n/4 words of FP4 and BF16 values and the
    # scale words (flags 1 and 2). (n/2 + 9) words bounds both.
    mov     rdx, [rdi + 16]         # n
    shr     rdx, 1
    add     rdx, 9
    shl     rdx, 3
    cmp     rdx, MAX_BYTES_DIRECT_MTRACE
    jbe     1f
    call    check_dynamic_mtrace    # rdx = bytes; clobbers rcx and r9, keeps rdi
1:
    SAVE_FOR_C
    lea     rsi, [r12 + r13*8]      # where the trace of this call goes
    call    dot_bf16_mtrace_c       # rdi = parameter block; returns rax = result, rdx = words
    RESTORE_FROM_C
    add     r13, rdx
    ret

.section .note.GNU-stack,"",%progbits
