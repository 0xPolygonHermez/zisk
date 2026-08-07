.intel_syntax noprefix
.code64

################################################################################
# direct_jump_dest_mops - JUMPDEST bitmap with mops recording
#
# Emits, in order:
#
#   1. one aligned read of EXTRA_PARAMETER_ADDR (the opcode reads count there)
#   2. one aligned block read per maximal run of consecutive loaded source
#      words — only the words the walk actually loads, not the whole range
#   3. one aligned block write covering every bitmap word (always consecutive)
#
# Grouping runs matters: a run of k words costs one entry instead of k, and the
# entries are what the mops buffer holds. Reads are emitted as blocks even when
# a run is one word long (to the memory counters a one-word block is exactly an
# aligned read) so that extending a run is a single unconditional add to the
# previous entry's word count.
#
# BUFFER SPACE: the worst case is a run of one word followed by a skipped word,
# repeating — any two adjacent loaded words merge into one entry — so the reads
# cost at most ceil(ceil(count/8)/2) entries, and the whole op at most
#
#     2 + ceil(count/16) entries = 16 + 8*ceil(count/16) bytes < count/2 + 24
#
# which is why this variant, unlike the DMA ones, has to check the buffer at
# all: the DMA mops are bounded at 6 entries regardless of count.
#
# PARAMETERS (non-standard ABI):
#   rdi = dst (u64*)                  - bitmap base, 8-byte aligned
#   rsi = src (u8*)                   - bytecode base, 8-byte aligned
#   rdx = count (usize)               - bytecode bytes
#   r12 = mops buffer base
#   r13 = mops index (input/output)
#
# RETURN:
#   rax = dst, r13 = updated mops index
################################################################################

.global direct_jump_dest_mops
.global direct_jump_dest_mops_with_count_check

.extern check_dynamic_mtrace

.include "dma_constants.inc"
.include "jump_dest_macro.inc"

.section .text

direct_jump_dest_mops_with_count_check:
    cmp     rdx, MAX_DMA_BYTES_DIRECT_MTRACE
    ja      .L_jdb_mops_check_dynamic
    jmp     direct_jump_dest_mops

.L_jdb_mops_check_dynamic:
    # check_dynamic_mtrace bills R_COUNT (= rdx = count) bytes, comfortably over
    # the count/2 + 24 the mops actually need.
    call    check_dynamic_mtrace

direct_jump_dest_mops:
    push    rbx
    push    r8
    push    r10
    push    r11
    push    r15
    push    rdi                           # original dst, for the write block

    # The opcode reads count from EXTRA_PARAMETER_ADDR.
    mov     rax, (MOPS_ALIGNED_READ + EXTRA_PARAMETER_ADDR)
    mov     [r12 + r13 * 8], rax
    inc     r13

    test    rdx, rdx
    jz      .L_jdb_mops_done              # count == 0: no reads and no bitmap

    mov     r15, -1                       # no open read run yet

    JUMP_DEST_WALK JD_RECORD_MOPS

    # Bitmap write block: ceil(count/64) consecutive aligned words, always one
    # entry however long the bitmap is.
    mov     rdi, [rsp]                    # original dst
    lea     rax, [rdx + 63]
    shr     rax, 6
    shl     rax, MOPS_BLOCK_WORDS_RS
    mov     rcx, MOPS_ALIGNED_BLOCK_WRITE
    add     rax, rcx
    add     rax, rdi
    mov     [r12 + r13 * 8], rax
    inc     r13

.L_jdb_mops_done:
    pop     rax                           # rax = dst
    pop     r15
    pop     r11
    pop     r10
    pop     r8
    pop     rbx
    ret

.section .note.GNU-stack,"",%progbits
