.intel_syntax noprefix
.code64
.text
.global legacy_op, new_op, legacy_nop, new_nop
.extern Arith256Mod, arith256_mod

# Both wrappers take rdi = address (pointer to 5 pointers: a,b,c,module,d).
#
# legacy_op: mimics the current emulator call site for the *existing* function. Because that callee
#   is an opaque ABI call, the caller must conservatively preserve everything it might clobber:
#   8 caller-saved GPRs + all 16 xmm registers. Then it unpacks the struct and calls the Rust
#   Arith256Mod(a,b,c,module,d).
#
# new_op: calls our tight assembly arith256_mod. It is not an opaque ABI call — we know exactly what
#   it touches — so the caller only preserves the volatile GPRs it needs, i.e. everything except
#   r8, r9, r11, and *no* xmm registers (the routine uses none). arith256_mod preserves rbx/rbp/r12-r15
#   itself.

legacy_op:
    push    rax
    push    rcx
    push    rdx
    push    rdi
    push    r8
    push    r9
    push    r10
    push    r11
    sub     rsp, 16*16 + 8               # +8 keeps rsp 16-aligned for movaps / the ABI call
    movaps  [rsp + 0*16], xmm0
    movaps  [rsp + 1*16], xmm1
    movaps  [rsp + 2*16], xmm2
    movaps  [rsp + 3*16], xmm3
    movaps  [rsp + 4*16], xmm4
    movaps  [rsp + 5*16], xmm5
    movaps  [rsp + 6*16], xmm6
    movaps  [rsp + 7*16], xmm7
    movaps  [rsp + 8*16], xmm8
    movaps  [rsp + 9*16], xmm9
    movaps  [rsp + 10*16], xmm10
    movaps  [rsp + 11*16], xmm11
    movaps  [rsp + 12*16], xmm12
    movaps  [rsp + 13*16], xmm13
    movaps  [rsp + 14*16], xmm14
    movaps  [rsp + 15*16], xmm15
    mov     rax, rdi                     # address
    mov     rdi, [rax+0]                 # a
    mov     rsi, [rax+8]                 # b
    mov     rdx, [rax+16]                # c
    mov     rcx, [rax+24]                # module
    mov     r8,  [rax+32]                # d
    call    Arith256Mod
    movaps  xmm0,  [rsp + 0*16]
    movaps  xmm1,  [rsp + 1*16]
    movaps  xmm2,  [rsp + 2*16]
    movaps  xmm3,  [rsp + 3*16]
    movaps  xmm4,  [rsp + 4*16]
    movaps  xmm5,  [rsp + 5*16]
    movaps  xmm6,  [rsp + 6*16]
    movaps  xmm7,  [rsp + 7*16]
    movaps  xmm8,  [rsp + 8*16]
    movaps  xmm9,  [rsp + 9*16]
    movaps  xmm10, [rsp + 10*16]
    movaps  xmm11, [rsp + 11*16]
    movaps  xmm12, [rsp + 12*16]
    movaps  xmm13, [rsp + 13*16]
    movaps  xmm14, [rsp + 14*16]
    movaps  xmm15, [rsp + 15*16]
    add     rsp, 16*16 + 8
    pop     r11
    pop     r10
    pop     r9
    pop     r8
    pop     rdi
    pop     rdx
    pop     rcx
    pop     rax
    ret

new_op:
    push    rax
    push    rcx
    push    rdx
    push    rsi
    push    rdi
    push    r10
    sub     rsp, 8                       # 16-align the call
    call    arith256_mod
    add     rsp, 8
    pop     r10
    pop     rdi
    pop     rsi
    pop     rdx
    pop     rcx
    pop     rax
    ret

# Empty callee to isolate the wrapper cost (same call/ret shape, no work).
empty_fn:
    xor     eax, eax
    ret

# legacy_nop / new_nop: the two wrappers around empty_fn, so the benchmark can measure the
# register-save overhead on its own (subtract from legacy_op/new_op to see the body cost).
legacy_nop:
    push    rax
    push    rcx
    push    rdx
    push    rdi
    push    r8
    push    r9
    push    r10
    push    r11
    sub     rsp, 16*16 + 8
    movaps  [rsp + 0*16], xmm0
    movaps  [rsp + 1*16], xmm1
    movaps  [rsp + 2*16], xmm2
    movaps  [rsp + 3*16], xmm3
    movaps  [rsp + 4*16], xmm4
    movaps  [rsp + 5*16], xmm5
    movaps  [rsp + 6*16], xmm6
    movaps  [rsp + 7*16], xmm7
    movaps  [rsp + 8*16], xmm8
    movaps  [rsp + 9*16], xmm9
    movaps  [rsp + 10*16], xmm10
    movaps  [rsp + 11*16], xmm11
    movaps  [rsp + 12*16], xmm12
    movaps  [rsp + 13*16], xmm13
    movaps  [rsp + 14*16], xmm14
    movaps  [rsp + 15*16], xmm15
    call    empty_fn
    movaps  xmm0,  [rsp + 0*16]
    movaps  xmm1,  [rsp + 1*16]
    movaps  xmm2,  [rsp + 2*16]
    movaps  xmm3,  [rsp + 3*16]
    movaps  xmm4,  [rsp + 4*16]
    movaps  xmm5,  [rsp + 5*16]
    movaps  xmm6,  [rsp + 6*16]
    movaps  xmm7,  [rsp + 7*16]
    movaps  xmm8,  [rsp + 8*16]
    movaps  xmm9,  [rsp + 9*16]
    movaps  xmm10, [rsp + 10*16]
    movaps  xmm11, [rsp + 11*16]
    movaps  xmm12, [rsp + 12*16]
    movaps  xmm13, [rsp + 13*16]
    movaps  xmm14, [rsp + 14*16]
    movaps  xmm15, [rsp + 15*16]
    add     rsp, 16*16 + 8
    pop     r11
    pop     r10
    pop     r9
    pop     r8
    pop     rdi
    pop     rdx
    pop     rcx
    pop     rax
    ret

new_nop:
    push    rax
    push    rcx
    push    rdx
    push    rsi
    push    rdi
    push    r10
    sub     rsp, 8
    call    empty_fn
    add     rsp, 8
    pop     r10
    pop     rdi
    pop     rsi
    pop     rdx
    pop     rcx
    pop     rax
    ret

.section .note.GNU-stack,"",@progbits
