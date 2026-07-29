.intel_syntax noprefix
.code64
.text
.global arith256_mod
.type arith256_mod, @function

# int arith256_mod(uint64_t **address)
#
#   d = (a*b + c) mod module   (256-bit, 4 little-endian u64 limbs each).
#   address[0..4] -> a, b, c, module, d   (same layout as _opcode_arith256_mod / emu.c).
#   Returns 0 in eax.  module must be non-zero.
#
#   Word-wise reduction using Knuth's Algorithm D in base 2^64: the 512-bit product a*b+c is 8 u64
#   limbs, the modulus is n (1..4) u64 limbs, and the remainder is produced by normalized long
#   division. The main loop runs 8-n+1 iterations; each quotient digit is estimated with one 128/64
#   `div` (capped when it would overflow) and applied with a mul/sub inner loop.
#
#   Uses callee-saved registers (rbx, rbp, r12-r15), saved/restored around the body:
#     rbx = struct ptr   r12 = n   r13 = s (norm shift)   r14 = j   r15 = qhat   rbp = rhat
#   Stack scratch (176 bytes):  RES(64) = product/dividend (8 u64), UN(72) = normalized dividend
#   (9 u64), VN(32) = normalized divisor (4 u64).

.set RES, 0
.set UN,  64
.set VN,  136

# res[i+j] += a[i]*b[j] + carry ; carry = high
.macro MUL_STEP i, j
    mov     rax, [rsi + 8*\i]
    mul     qword ptr [rcx + 8*\j]
    add     rax, r8
    adc     rdx, 0
    add     rax, [rsp + RES + 8*(\i+\j)]
    adc     rdx, 0
    mov     [rsp + RES + 8*(\i+\j)], rax
    mov     r8, rdx
.endm

.macro MUL_ROW i
    xor     r8, r8
    MUL_STEP \i, 0
    MUL_STEP \i, 1
    MUL_STEP \i, 2
    MUL_STEP \i, 3
    mov     [rsp + RES + 8*(\i+4)], r8
.endm

arith256_mod:
    push    rbx
    push    rbp
    push    r12
    push    r13
    push    r14
    push    r15
    sub     rsp, 176

    mov     rbx, rdi

    # ---- res = a*b (zero res first) ----
    xor     rax, rax
    mov     [rsp+RES+0], rax
    mov     [rsp+RES+8], rax
    mov     [rsp+RES+16], rax
    mov     [rsp+RES+24], rax
    mov     [rsp+RES+32], rax
    mov     [rsp+RES+40], rax
    mov     [rsp+RES+48], rax
    mov     [rsp+RES+56], rax
    mov     rsi, [rbx+0]                # a
    mov     rcx, [rbx+8]                # b
    MUL_ROW 0
    MUL_ROW 1
    MUL_ROW 2
    MUL_ROW 3

    # ---- res += c ----
    mov     rsi, [rbx+16]               # c
    mov     rax, [rsi+0]
    add     [rsp+RES+0], rax
    mov     rax, [rsi+8]
    adc     [rsp+RES+8], rax
    mov     rax, [rsi+16]
    adc     [rsp+RES+16], rax
    mov     rax, [rsi+24]
    adc     [rsp+RES+24], rax
    adc     qword ptr [rsp+RES+32], 0
    adc     qword ptr [rsp+RES+40], 0
    adc     qword ptr [rsp+RES+48], 0
    adc     qword ptr [rsp+RES+56], 0

    # ---- n = significant u64 limbs of module (1..4) ----
    mov     rsi, [rbx+24]               # module ptr
    mov     r12, 4
.Lnlen:
    mov     rax, [rsi + r12*8 - 8]
    test    rax, rax
    jnz     .Lnlen_done
    dec     r12
    cmp     r12, 1
    jg      .Lnlen
.Lnlen_done:

    cmp     r12, 1
    jne     .Lmulti

    # ---- n == 1: single u64 divisor ----
    mov     r8, [rsi]                   # v0
    xor     edx, edx                    # rem = 0
    mov     ecx, 7
.Ln1:
    mov     rax, [rsp + RES + rcx*8]
    div     r8                          # rdx:rax / v0 -> rdx = remainder
    dec     ecx
    jns     .Ln1
    mov     rdi, [rbx+32]               # d
    xor     eax, eax
    mov     [rdi+8], rax
    mov     [rdi+16], rax
    mov     [rdi+24], rax
    mov     [rdi+0], rdx
    jmp     .Ldone

.Lmulti:
    # ---- s = nlz(module_u64[n-1]) ----
    mov     rax, [rsi + r12*8 - 8]      # v[n-1]
    bsr     rax, rax
    xor     rax, 63                     # nlz = 63 - bsr
    mov     r13, rax                    # s
    test    r13, r13
    jz      .Lnorm_copy

    # ---- normalize (s > 0) with shld ----
    mov     rcx, r13                    # s in cl
    mov     rdi, r12
    dec     rdi                         # i = n-1
.Lvn:
    mov     rax, [rsi + rdi*8]          # v[i]
    mov     rdx, [rsi + rdi*8 - 8]      # v[i-1]
    shld    rax, rdx, cl                # (v[i]<<s)|(v[i-1]>>(64-s))
    mov     [rsp + VN + rdi*8], rax
    dec     rdi
    jnz     .Lvn
    mov     rax, [rsi]
    shl     rax, cl
    mov     [rsp + VN], rax
    # un[8] = u[7] >> (64-s)
    mov     rax, [rsp + RES + 7*8]
    xor     rdx, rdx
    shld    rdx, rax, cl                # rdx = u[7] >> (64-s)
    mov     [rsp + UN + 8*8], rdx
    mov     rdi, 7
.Lun:
    mov     rax, [rsp + RES + rdi*8]    # u[i]
    mov     rdx, [rsp + RES + rdi*8 - 8]# u[i-1]
    shld    rax, rdx, cl
    mov     [rsp + UN + rdi*8], rax
    dec     rdi
    jnz     .Lun
    mov     rax, [rsp + RES]
    shl     rax, cl
    mov     [rsp + UN], rax
    jmp     .Lmain

.Lnorm_copy:
    # s == 0: vn = v (n limbs), un = u (8 limbs), un[8] = 0
    xor     rdi, rdi
.Lvc:
    mov     rax, [rsi + rdi*8]
    mov     [rsp + VN + rdi*8], rax
    inc     rdi
    cmp     rdi, r12
    jl      .Lvc
    xor     rdi, rdi
.Luc:
    mov     rax, [rsp + RES + rdi*8]
    mov     [rsp + UN + rdi*8], rax
    inc     rdi
    cmp     rdi, 8
    jl      .Luc
    xor     rax, rax
    mov     [rsp + UN + 8*8], rax

.Lmain:
    mov     r14, 8
    sub     r14, r12                    # j = m - n  (m = 8)
.Ljloop:
    # qhat, rhat = divmod(un[j+n]:un[j+n-1], vn[n-1]), capping on overflow
    lea     r11, [r14 + r12]            # j+n
    mov     rdx, [rsp + UN + r11*8]     # un[j+n]  (high)
    mov     rax, [rsp + UN + r11*8 - 8] # un[j+n-1] (low)
    mov     r8, [rsp + VN + r12*8 - 8]  # vn[n-1]
    cmp     rdx, r8
    jae     .Lcap                       # un[j+n] >= vn[n-1] -> div would overflow
    div     r8                          # rax = qhat (<2^64), rdx = rhat
    mov     r15, rax
    mov     rbp, rdx
    jmp     .Lqhat_corr
.Lcap:
    mov     r15, -1                     # qhat = 2^64 - 1
    mov     rbp, rax                    # rhat = un[j+n-1] + vn[n-1]
    add     rbp, r8
    jc      .Lqdone                     # rhat >= 2^64 -> no correction

.Lqhat_corr:
    # while qhat*vn[n-2] > (rhat<<64) + un[j+n-2]: qhat--, rhat += vn[n-1] (stop if rhat >= 2^64)
    mov     rax, r15
    mul     qword ptr [rsp + VN + r12*8 - 16]  # rdx:rax = qhat * vn[n-2]
    lea     r11, [r14 + r12]
    mov     r8, [rsp + UN + r11*8 - 16] # un[j+n-2]
    cmp     rdx, rbp
    ja      .Ldo_corr
    jb      .Lqdone
    cmp     rax, r8
    jbe     .Lqdone
.Ldo_corr:
    dec     r15
    mov     rax, [rsp + VN + r12*8 - 8] # vn[n-1]
    add     rbp, rax
    jnc     .Lqhat_corr                 # rhat < 2^64 -> re-check
.Lqdone:

    # un[j..j+n-1] -= qhat * vn[0..n-1] ; carry (r9) into un[j+n]
    xor     r9, r9                      # carry
    xor     rcx, rcx                    # i
.Lmsub:
    mov     rax, r15
    mul     qword ptr [rsp + VN + rcx*8]# rdx:rax = qhat * vn[i]
    add     rax, r9                     # + carry
    adc     rdx, 0
    lea     r11, [rcx + r14]            # i+j
    sub     [rsp + UN + r11*8], rax     # un[i+j] -= (plo + carry)
    adc     rdx, 0                      # + borrow
    mov     r9, rdx                     # carry for next limb
    inc     rcx
    cmp     rcx, r12
    jl      .Lmsub
    lea     r11, [r14 + r12]            # j+n
    sub     [rsp + UN + r11*8], r9      # un[j+n] -= carry ; CF = borrow -> qhat too big
    jnc     .Lnext

    # add-back: un[j..j+n-1] += vn[0..n-1], carry into un[j+n] (CF preserved via lea/dec)
    xor     rcx, rcx
    mov     r10, r12
    clc
.Ladd:
    mov     rax, [rsp + VN + rcx*8]
    lea     r11, [r14 + rcx]
    adc     [rsp + UN + r11*8], rax
    lea     rcx, [rcx + 1]
    dec     r10
    jnz     .Ladd
    lea     r11, [r14 + r12]
    adc     qword ptr [rsp + UN + r11*8], 0
.Lnext:
    dec     r14
    jns     .Ljloop

    # ---- denormalize remainder into d ----
    mov     rsi, [rbx+32]               # d
    xor     rax, rax
    mov     [rsi+0], rax
    mov     [rsi+8], rax
    mov     [rsi+16], rax
    mov     [rsi+24], rax
    test    r13, r13
    jz      .Ldenorm0
    mov     rcx, r13                    # s
    xor     rdi, rdi
.Ldenorm:
    mov     rax, [rsp + UN + rdi*8]
    mov     rdx, [rsp + UN + rdi*8 + 8] # un[i+1]
    shrd    rax, rdx, cl                # (un[i]>>s)|(un[i+1]<<(64-s))
    mov     [rsi + rdi*8], rax
    inc     rdi
    cmp     rdi, r12
    jl      .Ldenorm
    jmp     .Ldone
.Ldenorm0:
    xor     rdi, rdi
.Ldenorm0l:
    mov     rax, [rsp + UN + rdi*8]
    mov     [rsi + rdi*8], rax
    inc     rdi
    cmp     rdi, r12
    jl      .Ldenorm0l

.Ldone:
    xor     eax, eax
    add     rsp, 176
    pop     r15
    pop     r14
    pop     r13
    pop     r12
    pop     rbp
    pop     rbx
    ret

.size arith256_mod, .-arith256_mod
