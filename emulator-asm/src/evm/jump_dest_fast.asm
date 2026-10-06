.intel_syntax noprefix
.code64

################################################################################
# jump_dest_fast - JUMPDEST bitmap with no tracing
#
# Used by the fast and rom-histogram emulation methods, which need the memory
# effect of the precompile but produce no variable trace.
#
# PARAMETERS (non-standard ABI):
#   rdi = dst (u64*)    - bitmap base, 8-byte aligned
#   rsi = src (u8*)     - bytecode base, 8-byte aligned
#   rdx = count (usize) - bytecode bytes
#
# RETURN:
#   rax = dst (the original bitmap base)
#
# Writes exactly ceil(count/64) aligned 64-bit words at dst, zeros included.
################################################################################

.global jump_dest_fast

.include "dma_constants.inc"
.include "jump_dest_macro.inc"

.section .text

jump_dest_fast:
    push    rbx
    push    r8
    push    r10
    push    r11
    push    rdi                           # original dst, returned in rax

    test    rdx, rdx
    jz      .L_jdb_fast_done              # count == 0: nothing read, nothing written

    JUMP_DEST_WALK JD_RECORD_NONE

.L_jdb_fast_done:
    pop     rax                           # rax = dst
    pop     r11
    pop     r10
    pop     r8
    pop     rbx
    ret

.section .note.GNU-stack,"",%progbits
