.intel_syntax noprefix
.code64

################################################################################
# JUMP_DEST_WALK - shared core of the three jump_dest variants
#
# Walks `count` bytes of EVM bytecode and writes the JUMPDEST bitmap: bit i is
# set iff byte i is a real JUMPDEST (0x5b that is not PUSH immediate data).
#
# The walk is driven one **aligned 64-bit source word at a time**, not byte by
# byte, because that is the granularity the AIR proves: one row per source word,
# one bitmap byte per row, one aligned 64-bit bitmap word written per 8 rows.
#
# A word is loaded only when the instruction cursor is inside it. When a PUSH
# covers a whole word, no byte of that word is ever examined and the machine
# does not read it (sel_mem_load = 0 in the AIR) — which is why the set of
# memory reads depends on the bytecode content and not just on its length, and
# why the trace has to be produced while walking. The `record` hook below is
# invoked exactly once per loaded word, in increasing address order.
#
# Both pointers must be 8-byte aligned: the machine reads and writes whole
# aligned words, and an unaligned run is not arithmetizable. The guest checks
# this and falls back to a software walk, so this code may assume it.
#
# PARAMETERS (non-standard ABI, shared with the DMA routines):
#   rsi = src   - bytecode base, 8-byte aligned
#   rdi = dst   - bitmap base, 8-byte aligned; advanced past the bitmap on exit
#   rdx = count - bytecode bytes, must be > 0 (callers test it first)
#
# CLOBBERS: rax, rcx, rbx, r8, r9, r10, r11, rdi (callers save what they need)
#
# REGISTERS:
#   r8  = p, the byte cursor into the bytecode (a monotonic instruction cursor)
#   r9  = scan limit, reused for the group bound and the in-word bound
#   r10 = bitmap word being accumulated for the current 64-byte group
#   r11 = aligned_end, the cursor value at which the walk is over
#   rbx = end offset of the current 64-byte group
################################################################################

.macro JUMP_DEST_WALK record

    lea     r11, [rdx + 7]
    and     r11, ALIGN_MASK              # r11 = aligned_end = (count + 7) & ~7

    xor     r8, r8                       # r8 = p = 0
    mov     rbx, 64                      # rbx = end of group 0

.L_jd_group\@:
    xor     r10, r10                     # a group always writes its word, zeros included

.L_jd_word\@:
    mov     r9, rbx
    cmp     r9, r11
    cmova   r9, r11                      # r9 = min(group_end, aligned_end)
    cmp     r8, r9
    jae     .L_jd_group_done\@

    mov     rax, r8
    and     rax, ALIGN_MASK              # rax = offset of the word holding the cursor
    lea     r9, [rax + 8]
    cmp     r9, rdx
    cmova   r9, rdx                      # r9 = min(word_end, count), the in-word bound

    \record                              # one loaded word: rax = its offset from src

.L_jd_byte\@:
    cmp     r8, r9
    jae     .L_jd_word_done\@
    movzx   eax, byte ptr [rsi + r8]
    cmp     al, 0x5b
    je      .L_jd_mark\@

    # Not a JUMPDEST: advance by the instruction width. PUSH1..PUSH32 is the
    # 0x60..0x7f range, i.e. top three bits 011, and spans opcode - 0x5e bytes
    # (1 opcode + 1..32 immediates); everything else is a single byte.
    mov     ecx, eax
    sub     ecx, 0x5e
    and     eax, 0xe0
    cmp     eax, 0x60
    mov     eax, 1
    cmove   eax, ecx
    add     r8, rax
    jmp     .L_jd_byte\@

.L_jd_mark\@:
    bts     r10, r8                      # bts indexes mod 64: the bit inside the group
    inc     r8
    jmp     .L_jd_byte\@

.L_jd_word_done\@:
    # A bound of `count` rather than `word_end` means this was the last source
    # word, so jump the cursor to aligned_end: the scan stopped short of the
    # word end and would otherwise loop on the same word forever.
    cmp     r9, rdx
    cmove   r8, r11
    jmp     .L_jd_word\@

.L_jd_group_done\@:
    mov     [rdi], r10
    add     rdi, 8
    add     rbx, 64

    # Keep going while another group exists. This is driven by the group bound,
    # not by the cursor: a PUSH straddling the end of the bytecode can leave the
    # cursor past aligned_end while bitmap words are still owed.
    lea     rax, [rbx - 64]
    cmp     rax, r11
    jb      .L_jd_group\@
.endm

################################################################################
# Record hooks. Each is invoked with rax = the loaded word's offset from rsi,
# and may clobber rax and rcx only.
################################################################################

# No recording: used by the fast variant, and by the minimal-trace one, which
# captures the whole source range up front with a single rep movsq instead of
# word by word (its payload is the contiguous range, not the loaded subset).
.macro JD_RECORD_NONE
.endm

# Mops: append the loaded word's address, merging consecutive words into one
# aligned block. r15 holds the address the open run would continue at, so a
# contiguous word only has to bump the previous entry's word count. Reads are
# always emitted as blocks (a one-word block is exactly an aligned read to the
# memory counters) so that the bump is a single unconditional add.
.macro JD_RECORD_MOPS
    lea     rax, [rsi + rax]
    cmp     rax, r15
    je      .L_jd_mops_extend\@

    mov     rcx, MOPS_ALIGNED_BLOCK_READ + MOPS_BLOCK_ONE_WORD
    add     rcx, rax
    mov     [r12 + r13 * 8], rcx
    inc     r13
    jmp     .L_jd_mops_done\@

.L_jd_mops_extend\@:
    mov     rcx, MOPS_BLOCK_ONE_WORD
    add     [r12 + r13 * 8 - 8], rcx

.L_jd_mops_done\@:
    lea     r15, [rax + 8]
.endm
