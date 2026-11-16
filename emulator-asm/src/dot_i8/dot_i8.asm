.intel_syntax noprefix
.code64

#
# dot_i8 - experimental int8 dot product precompile (see core/src/operations/dot_i8.rs)
#
# rdi is the address of a parameter block of three qwords: the address of vector x, the
# address of vector y and their length n in bytes. Both addresses are 8-byte aligned and n is
# a multiple of 8, as the Rust emulator checks. The result is the sum of x[i]*y[i] over signed
# bytes, wrapped to i32 and sign-extended to 64 bits.
#
# dot_i8_fast:   computes the result only (fast and ROM histogram generation).
# dot_i8_mtrace: also appends to the minimal trace, at [r12 + r13*8]:
#     [0] x address, [1] y address, [2] n, [3] the result,
#     then the n/8 qwords of x, then the n/8 qwords of y.
#   It advances r13 past them, growing the trace first when they do not fit in the per-step
#   budget (check_dynamic_mtrace).
#
# REGISTERS (NON System V AMD64 ABI, as the DMA routines):
#   Input:   rdi = parameter block address; r12, r13 = minimal trace; r14 = steps left in chunk
#   Output:  rax = result
#   Clobbers rcx, rdx, rsi, rdi, r9. rbx and the xmm registers it uses are saved and restored,
#   because the generated code keeps values in them (RISC-V registers, in the xmm ones).

.global dot_i8_fast
.global dot_i8_mtrace
.extern check_dynamic_mtrace

.include "dma_constants.inc"

.section .text

# Saves xmm0-xmm2, which the loop uses.
.macro SAVE_XMM
    sub     rsp, 48
    movdqu  [rsp], xmm0
    movdqu  [rsp + 16], xmm1
    movdqu  [rsp + 32], xmm2
.endm

.macro RESTORE_XMM
    movdqu  xmm0, [rsp]
    movdqu  xmm1, [rsp + 16]
    movdqu  xmm2, [rsp + 32]
    add     rsp, 48
.endm

# xmm2 += the dot product of the signed bytes of rax and r9, as four i32 lanes.
.macro DOT_WORD
    movq     xmm0, rax
    movq     xmm1, r9
    pmovsxbw xmm0, xmm0             # 8 x i16
    pmovsxbw xmm1, xmm1
    pmaddwd  xmm0, xmm1             # 4 x i32: pairs of products, at most 2 * 128 * 128
    paddd    xmm2, xmm0
.endm

# rax = the sign-extended i32 sum of the four lanes of xmm2.
.macro SUM_LANES
    pshufd  xmm0, xmm2, 0x4e
    paddd   xmm2, xmm0
    pshufd  xmm0, xmm2, 0xb1
    paddd   xmm2, xmm0
    movd    eax, xmm2
    movsxd  rax, eax
.endm

dot_i8_fast:
    SAVE_XMM
    mov     rsi, [rdi + 8]          # y
    mov     rcx, [rdi + 16]         # n
    mov     rdi, [rdi]              # x
    shr     rcx, 3                  # qwords
    pxor    xmm2, xmm2
    test    rcx, rcx
    jz      .L_fast_done
.L_fast_loop:
    mov     rax, [rdi]
    mov     r9, [rsi]
    DOT_WORD
    add     rdi, 8
    add     rsi, 8
    dec     rcx
    jnz     .L_fast_loop
.L_fast_done:
    SUM_LANES
    RESTORE_XMM
    ret

dot_i8_mtrace:
    mov     rcx, [rdi + 16]         # n
    # Bytes this call appends to the trace: 4 header qwords and both vectors
    lea     rdx, [rcx * 2 + 32]
    cmp     rdx, MAX_BYTES_DIRECT_MTRACE
    jbe     .L_mt_fits
    push    rcx                     # check_dynamic_mtrace uses rcx and r9
    call    check_dynamic_mtrace
    pop     rcx
.L_mt_fits:
    SAVE_XMM
    mov     rsi, [rdi + 8]          # y
    mov     rdi, [rdi]              # x
    mov     [r12 + r13*8], rdi
    mov     [r12 + r13*8 + 8], rsi
    mov     [r12 + r13*8 + 16], rcx
    shr     rcx, 3                  # qwords
    push    rbx                     # rbx is the generated code's a register
    lea     rdx, [r12 + r13*8 + 32] # trace pointer for x
    lea     rbx, [rdx + rcx*8]      # trace pointer for y, after the x qwords
    pxor    xmm2, xmm2
    test    rcx, rcx
    jz      .L_mt_done
.L_mt_loop:
    mov     rax, [rdi]
    mov     r9, [rsi]
    mov     [rdx], rax
    mov     [rbx], r9
    DOT_WORD
    add     rdi, 8
    add     rsi, 8
    add     rdx, 8
    add     rbx, 8
    dec     rcx
    jnz     .L_mt_loop
.L_mt_done:
    mov     rdx, rbx                # the end of the appended qwords
    pop     rbx
    SUM_LANES
    RESTORE_XMM
    mov     [r12 + r13*8 + 24], rax
    sub     rdx, r12
    shr     rdx, 3
    mov     r13, rdx
    ret

.section .note.GNU-stack,"",%progbits
