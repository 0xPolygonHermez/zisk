.intel_syntax noprefix
.code64

################################################################################
# direct_jump_dest_mtrace - JUMPDEST bitmap with minimal-trace recording
#
# Emits the trace the ConsumeMemReads replay needs:
#
#   [0]   count, the value the opcode reads from EXTRA_PARAMETER_ADDR
#   [1..] every source word the byte range spans, ceil(count/8) of them
#
# The payload is the whole contiguous source range, including the words the
# machine never loads because a PUSH covers them — the walk discards those. That
# is a few trace words more than the loaded set on push-heavy code, and in
# exchange the data_ext length follows from `count` alone, so the slice handed
# to the collectors is a plain contiguous range and the copy is one rep movsq,
# exactly like the DMA source capture. The mops, which claim memory operations
# rather than carry data, do list only the words actually loaded.
#
# PARAMETERS (non-standard ABI):
#   rdi = dst (u64*)                  - bitmap base, 8-byte aligned
#   rsi = src (u8*)                   - bytecode base, 8-byte aligned
#   rdx = count (usize)               - bytecode bytes
#   r12 = mtrace buffer base
#   r13 = mtrace index (input/output)
#
# RETURN:
#   rax = dst, r13 = updated mtrace index
#
# TRACE SIZE: 1 + ceil(count/8) qwords, under count + 16 bytes.
# `_with_count_check` reserves that through the shared check_dynamic_mtrace,
# whose MAX_DMA_MT_MARGIN covers the header slack.
################################################################################

.global direct_jump_dest_mtrace
.global direct_jump_dest_mtrace_with_count_check

.extern check_dynamic_mtrace

.include "dma_constants.inc"
.include "jump_dest_macro.inc"

.section .text

direct_jump_dest_mtrace_with_count_check:
    cmp     rdx, MAX_DMA_BYTES_DIRECT_MTRACE
    ja      .L_jdb_mt_check_dynamic
    jmp     direct_jump_dest_mtrace

.L_jdb_mt_check_dynamic:
    # check_dynamic_mtrace bills R_COUNT (= rdx = count) bytes plus its own
    # margin, which is the bound above with room to spare.
    call    check_dynamic_mtrace

direct_jump_dest_mtrace:
    push    rbx
    push    r8
    push    r10
    push    r11
    push    rdi                           # original dst, returned in rax

    mov     [r12 + r13 * 8], rdx          # header: count
    inc     r13

    test    rdx, rdx
    jz      .L_jdb_mt_done                # count == 0: header only

    # Copy the whole source range into the trace: ceil(count/8) qwords.
    push    rsi                           # rep movsq advances rsi and rdi
    push    rdi
    lea     rcx, [rdx + 7]
    shr     rcx, 3
    lea     rdi, [r12 + r13 * 8]
    add     r13, rcx
    rep movsq
    pop     rdi
    pop     rsi

    JUMP_DEST_WALK JD_RECORD_NONE

.L_jdb_mt_done:
    pop     rax                           # rax = dst
    pop     r11
    pop     r10
    pop     r8
    pop     rbx
    ret

.section .note.GNU-stack,"",%progbits
