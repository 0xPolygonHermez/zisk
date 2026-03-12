.intel_syntax noprefix
.code64
.text
.global fast_memcpy64
.type fast_memcpy64, @function

# void fast_memcpy64(uint64_t* dst, uint64_t *v, size_t n_qwords)
#   rdi = dst
#   rsi = src
#   rdx = n  (QWORDS)
#
# Clobbers: rax, rcx, r9

fast_memcpy64:

    # only first could be lt 32x8=256 bytes
    # 256 bytes => 32 qwords
    test    rdx, 0x1F
    jz      .L_fast_memcpy64_mul256

    mov     rcx, rdx
    and     rdx, 0xFFFFFFFFFFFFFFE0
    sub     rcx, rdx

    # Jump to entry that leaves exactly q MOVSQ until the end
    lea     r9, [rip + .L_fast_memcpy64_jump_qword_table]
    jmp     [r9 + rcx*8]

.p2align 3
.L_fast_memcpy64_jump_qword_table:
    .quad .L_q0
    .quad .L_q1
    .quad .L_q2
    .quad .L_q3
    .quad .L_q4
    .quad .L_q5
    .quad .L_q6
    .quad .L_q7
    .quad .L_q8
    .quad .L_q9
    .quad .L_q10
    .quad .L_q11
    .quad .L_q12
    .quad .L_q13
    .quad .L_q14
    .quad .L_q15
    .quad .L_q16
    .quad .L_q17
    .quad .L_q18
    .quad .L_q19
    .quad .L_q20
    .quad .L_q21
    .quad .L_q22
    .quad .L_q23
    .quad .L_q24
    .quad .L_q25
    .quad .L_q26
    .quad .L_q27
    .quad .L_q28
    .quad .L_q29
    .quad .L_q30
    .quad .L_q31
    .quad .L_q32

# Fallthrough chain: entering at q31 executes 31 STOSQ down to q1
.L_q32:   movsq
.L_q31:   movsq
.L_q30:   movsq
.L_q29:   movsq
.L_q28:   movsq
.L_q27:   movsq
.L_q26:   movsq
.L_q25:   movsq
.L_q24:   movsq
.L_q23:   movsq
.L_q22:   movsq
.L_q21:   movsq
.L_q20:   movsq
.L_q19:   movsq
.L_q18:   movsq
.L_q17:   movsq
.L_q16:   movsq
.L_q15:   movsq
.L_q14:   movsq
.L_q13:   movsq
.L_q12:   movsq
.L_q11:   movsq
.L_q10:   movsq
.L_q9:    movsq
.L_q8:    movsq
.L_q7:    movsq
.L_q6:    movsq
.L_q5:    movsq
.L_q4:    movsq
.L_q3:    movsq
.L_q2:    movsq
.L_q1:    movsq
.L_q0:

    # check if remain more 32 x 8 = 256 bytes blocks

.L_fast_memcpy64_mul256:
    test rdx, 0xFFFFFFFFFFFFFFE0 # 0xFFFF_FFFF_FFFF_FF00
    jz .L_fast_memcpy64_done
    sub     rdx, 32
    jmp     .L_q32

.L_fast_memcpy64_done:
    ret

.size fast_memcpy64, .-fast_memcpy64
