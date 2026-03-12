.intel_syntax noprefix
.code64
.text
.global fast_memset
.type fast_memset, @function

# void fast_memset(void* dst, uint8_t v, size_t n)
#   rdi = dst
#   rsi = v  (only low 8 bits used)
#   rdx = n  (bytes)
#
# Clobbers: rax, rcx, r9

fast_memset:
    movzx   eax, sil
    mov     rsi, rdi

    test    rdx, 0xFFFFFFFFFFFFFFF8
    jz      .L_fast_memset_count_lt_8

    # Build 64-bit pattern 0xvvvvvvvvvvvvvvvv in RAX (needed for all paths)
    mov     r9, 0x0101010101010101
    imul    rax, r9                 # rax = v * 0x0101010101010101

    # only first could be lt 32x8=256 bytes
    movzx   ecx, dl
    and     rdx, 0xFFFFFFFFFFFFFF07

    # Jump to entry that leaves exactly q STOSQ until the end
    shr     ecx, 3
    lea     r9, [rip + .L_fast_memcpy_jump_qword_table]
    jmp     [r9 + rcx*8]

.p2align 3
.L_fast_memcpy_jump_qword_table:
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
.L_q32:   stosq
.L_q31:   stosq
.L_q30:   stosq
.L_q29:   stosq
.L_q28:   stosq
.L_q27:   stosq
.L_q26:   stosq
.L_q25:   stosq
.L_q24:   stosq
.L_q23:   stosq
.L_q22:   stosq
.L_q21:   stosq
.L_q20:   stosq
.L_q19:   stosq
.L_q18:   stosq
.L_q17:   stosq
.L_q16:   stosq
.L_q15:   stosq
.L_q14:   stosq
.L_q13:   stosq
.L_q12:   stosq
.L_q11:   stosq
.L_q10:   stosq
.L_q9:    stosq
.L_q8:    stosq
.L_q7:    stosq
.L_q6:    stosq
.L_q5:    stosq
.L_q4:    stosq
.L_q3:    stosq
.L_q2:    stosq
.L_q1:    stosq
.L_q0:

    # check if remain more 32 x 8 = 256 bytes blocks

    test rdx, 0xFFFFFFFFFFFFFF00 # 0xFFFF_FFFF_FFFF_FF00
    jz .L_fast_memset_count_lt_8
    sub     rdx, 256
    jmp     .L_q32


.L_fast_memset_count_lt_8:

    # Jump to byte tail entry
    lea     r9, [rip + .L_fast_memset_jump_byte_table]
    jmp     [r9 + rdx*8]

.p2align 3
.L_fast_memset_jump_byte_table:
    .quad .L_b0
    .quad .L_b1
    .quad .L_b2
    .quad .L_b3
    .quad .L_b4
    .quad .L_b5
    .quad .L_b6
    .quad .L_b7

.L_b7:    stosb
.L_b6:    stosb
.L_b5:    stosb
.L_b4:    stosb
.L_b3:    stosb
.L_b2:    stosb
.L_b1:    stosb
.L_b0:
    mov   rax, rsi
    ret

.size fast_memset, .-fast_memset
