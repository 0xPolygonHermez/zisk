.intel_syntax noprefix
.code64


######################################################################################
# fast_memcmp - Optimized comparison of two memory regions. Returns (a - b) of the 
#               first different byte, or 0 if equal. Also updates rdx with the 
#               effective count (number of bytes checked to find the difference).
# PARAMETERS
#   rdi: addr_a
#   rsi: addr_b
#   rdx: count (bytes)
#
# RESULT
#   rdx: updated with effective count (bytes)
#   rax: i64 comparison result of first different bytes (a - b)
#
# CLOBBERED REGS: rcx
# ───────────────────────────────────────────────────────────────────────────────────
# fast_memcmp_count_nz - Same as fast_memcmp, but assumes count > 0 (caller must
#                        verify this beforehand).
# PARAMETERS
#   rdi: addr_a
#   rsi: addr_b
#   rdx: count (bytes)
#
# RESULT
#   rdx: updated with effective count (bytes)
#   rax: i64 comparison result of first different bytes (a - b)
#
# CLOBBERED REGS: rcx
# ───────────────────────────────────────────────────────────────────────────────────
# get_memcmp_effective_count - Updates rdx with the effective count, which is the
#                              number of bytes needed to compare a and b.
# PARAMETERS
#   rdi: addr_a
#   rsi: addr_b
#   rdx: count (bytes)
#
# RESULT
#   rdx: updated with effective count (bytes)
#
# CLOBBERED REGS: rcx
################################################################################

.global fast_memcmp
.global fast_memcmp_count_nz
.global get_memcmp_effective_count

.section .text

# PARAMETERS                
#   rdi: addr_a
#   rsi: addr_b
#   rdx: count (bytes)
#
# RESULT
#   rdx: updated with effective count (bytes)
#   rax: i64 comparison result of first different bytes (a - b)
#
# CLOBBERED REGS: rcx

fast_memcmp:

    mov     rcx, rdx                # rcx = rdx (count)
    test    rcx, rcx
    jz      .L_dma_memcmp_zero

    repe    cmpsb                   # Compare byte-by-byte; on mismatch, increments
                                    # rdi, rsi and decrements rcx

    jz      .L_fast_memcmp_eq       # Jump if all bytes were equal
    sub     rdx, rcx                # rdx = rdx - rcx (*)
    movzx   rax, byte ptr [rdi - 1] # rax = a (zero-extended)
    movzx   rcx, byte ptr [rsi - 1] # rcx = b (zero-extended)  
    sub     rax, rcx                # rax = a - b
    sub     rdi, rdx                # restore rdi
    sub     rsi, rdx                # restore rsi
    ret

.L_fast_memcmp_eq:
    xor     rax, rax                # rax = 0
    sub     rdi, rdx                # restore rdi
    sub     rsi, rdx                # restore rsi
    ret

.L_dma_memcmp_zero:
    xor     rax, rax                # rax = 0
    ret

# PARAMETERS
#   rdi: addr_a
#   rsi: addr_b
#   rdx: count (bytes), must be > 0
#
# RESULT
#   rdx: updated with effective count (bytes)
#   rax: i64 comparison result of first different bytes (a - b)
#
# CLOBBERED REGS: rcx

fast_memcmp_count_nz:

    mov     rcx, rdx                    # rcx = rdx (count)
    repe    cmpsb                       # Compare byte-by-byte; on mismatch, increments
                                        # rdi, rsi and decrements rcx

    jz      .L_fast_memcmp_count_nz_eq  # Jump if all bytes were equal

    sub     rdx, rcx                    # rdx = rdx - rcx (*)
    movzx   rax, byte ptr [rdi - 1]     # rax = a (zero-extended)
    movzx   rcx, byte ptr [rsi - 1]     # rcx = b (zero-extended)  
    sub     rax, rcx                    # rax = a - b
    add     rdi, rdx                    # restore rdi                   
    add     rsi, rdx                    # restore rsi
    ret

.L_fast_memcmp_count_nz_eq:
    xor     rax, rax                    # rax = 0 (buffers are equal)
    add     rdi, rdx                    # restore rdi
    add     rsi, rdx                    # restore rsi
    ret

# PARAMETERS
#   rdi: addr_a
#   rsi: addr_b
#   rdx: count (bytes)
#
# RESULT
#   rdx: updated with effective count (bytes)
#
# CLOBBERED REGS: rcx

get_memcmp_effective_count:

    mov     rcx, rdx                    # rcx = rdx (count)
    repe    cmpsb                       # Compare byte-by-byte; on mismatch, increments
                                        # rdi, rsi and decrements rcx

    jz      .L_get_memcmp_effective_count_eq  # Jump if all bytes were equal

    sub     rdx, rcx                    # rdx = rdx - rcx (*)
    add     rdi, rdx                    # restore rdi                   
    add     rsi, rdx                    # restore rsi
    ret

.L_get_memcmp_effective_count_eq:
    add     rdi, rdx                    # restore rdi
    add     rsi, rdx                    # restore rsi
    ret

# Mark stack as non-executable (required by modern linkers)
.section .note.GNU-stack,"",%progbits
