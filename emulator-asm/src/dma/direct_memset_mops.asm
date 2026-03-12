.intel_syntax noprefix
.code64

################################################################################
# direct_dma_xmemset_mops - Memory set with mops (memory operation) tracing
#
# This function fills a memory region with a byte value while recording all
# memory operation addresses to the mops buffer for verification.
#
# MAIN TASKS:
# 1. Record memory operation addresses (pre-reads for partial qwords, writes)
# 2. Perform the actual memset operation (via fast_memset)
#
# REGISTER USAGE:
#   Uses: rax, rcx, rdx, rdi, rsi, r9, r12, r13
#   Does NOT use XMM registers (caller doesn't need to save them)
#   Modifies: r13 (mops index output)
#
# PARAMETERS (non-standard ABI):
#   rdi = dst (u64)                     - Destination address to fill
#   rsi = value (u8 in low byte)        - Byte value to set (0-255)
#   rdx = count (usize)                 - Number of bytes to set
#   r12 = mops buffer base address      - Base pointer to mops buffer
#   r13 = mops buffer index             - Current index (updated on return)
#
# RETURN:
#   r13 = Updated mops index
#
# BRANCHES:
#   FAST: dst aligned + count multiple of 8 → only write block entry
#   BRANCH 1: dst aligned + count NOT multiple of 8 → 1 pre-read (post) + write
#   BRANCH 2.1: dst unaligned + fits single range → 1 pre-read (pre) + write
#   BRANCH 2.2: dst unaligned + spans qwords → 2 pre-reads (pre/post) + write
################################################################################

.global direct_dma_xmemset_mops
.extern fast_memset

.include "dma_constants.inc"

.section .text

################################################################################
# direct_dma_xmemset_mops - Direct entry point (non-standard ABI)
#
# Called directly from generated assembly code without ABI overhead.
# More efficient when caller manages register preservation.
#
# PARAMETERS:
#   rdi = destination address
#   rsi = byte value (0-255)
#   rdx = byte count
#   r12 = mops buffer base
#   r13 = mops buffer index (input/output)
################################################################################

direct_dma_xmemset_mops:
   
    # Modified registers (caller must handle): 
    #   r9  = scratch for mops address calculation
    #   rcx = scratch for calculations
    #   r13 = mops index (updated on return)

    # Early exit if count = 0
    test    rdx, rdx
    jz      .L_xmemset_mops_done

    # Check if dst is 8-byte aligned
    test    rdi, 0x7
    jnz     .L_xmemset_mops_rdi_unaligned

    # Check if count is multiple of 8
    test    rdx, 0x07
    jnz     .L_memset_mops_count_remain

    # ========== FAST PATH ==========
    # dst is aligned AND count is multiple of 8
    # => No pre-reads needed, only one write block entry

    mov     rax, rdx
    shr     rax, 3                            # 1 cycle - rax = count / 8 (qwords)
    shl     rax, MOPS_BLOCK_WORDS_RS          # 1 cycle - format for mops block
    mov     r9, MOPS_ALIGNED_BLOCK_WRITE      # 1 cycle - write block flag
    add     r9, rax                           # 1 cycle - add qword count
    add     r9, rdi                           # 1 cycle - add dst (already aligned)

    mov     [r12 + r13 * 8], r9               # ~4 cycles - write mops entry
    inc     r13                               # 1 cycle - advance mops index

    jmp     fast_memset                       # tail call to fast_memset

    # fast_memset "execute" the return, memset set rax = rdi

    # ========== BRANCH 1 ==========
    # dst aligned, count NOT multiple of 8
    # => 1 pre-read (post qword) + 1 write block

.L_memset_mops_count_remain:

    # Calculate qwords needed: ceil(count / 8) = (count + 7) / 8
    lea     r9, [rdx + 7]
    shr     r9, 3                             # 1 cycle - r9 = qwords to write

    # BRANCH 1 - POST pre-read: read last qword (partial overwrite)
    lea     rcx, [rdi + r9 * 8 - 8]           # 1 cycle - address of last qword
    mov     rax, MOPS_ALIGNED_READ            # 1 cycle - read flag
    add     rcx, rax                          # 1 cycle - combine
    mov     [r12 + r13 * 8], rcx              # ~4 cycles - write pre-read entry

    # BRANCH 1 - Write block entry
    shl     r9, MOPS_BLOCK_WORDS_RS           # 1 cycle - format qwords for mops
    mov     rax, MOPS_ALIGNED_BLOCK_WRITE     # 1 cycle - write block flag
    add     rax, r9                           # 1 cycle - add qword count
    add     rax, rdi                          # 1 cycle - add dst (aligned)

    mov     [r12 + r13 * 8 + 8], rax          # ~4 cycles - write block entry
    add     r13, 2  
                                              # 1 cycle - advance index by 2
    jmp     fast_memset                       # tail call to fast_memset

    # fast_memset "execute" the return, memset set rax = rdi

    # ========== BRANCH 2 ==========
    # dst NOT aligned
    # Must determine if we need 1 or 2 pre-reads

.L_xmemset_mops_rdi_unaligned:

    # Calculate total span: (rdi & 0x7) + count
    # If span <= 8: only PRE read needed (BRANCH 2.1)
    # If span > 8 and end is aligned: only PRE read needed (BRANCH 2.1)
    # If span > 8 and end is unaligned: PRE + POST reads needed (BRANCH 2.2)
    
    mov     rcx, rdi
    and     rcx, 0x07                         # 1 cycle - offset within qword
    lea     rcx, [rcx + rdx + 7]              # 1 cycle - rcx = offset + count + 7
    test    rcx, 0xFFFFFFFFFFFFFFF0           # 1 cycle - check if (offset + count + 7) > 15
                                              # => (offset + count) > 8 => spans qwords
    jnz     .L_pre_branch_2_2                 # 2 cycles (predicted)
    jmp     .L_branch_2_1                     # single qword span

    # Check if end is unaligned (needs POST read)
.L_pre_branch_2_2:
    lea     rax, [rcx - 7]                    # 1 cycle - rax = offset + count
    test    rax, 0x7                          # 1 cycle - check if end is aligned
    jnz     .L_branch_2_2                     # 2 cycles - end unaligned, need POST

    # ========== BRANCH 2.1 ==========
    # dst unaligned, but end IS aligned (or fits in one qword)
    # => 1 pre-read (PRE) + 1 write block

.L_branch_2_1:

    # Calculate aligned base and qword count
    # rcx = (rdi & 0x7) + rdx + 7  →  (rcx >> 3) = qwords needed
    mov     rax, rdi
    and     rax, ALIGN_MASK                   # 1 cycle - rax = aligned dst
    shr     rcx, 3                            # 1 cycle - rcx = qwords
    shl     rcx, MOPS_BLOCK_WORDS_RS          # 1 cycle - format for mops
    mov     r9, MOPS_ALIGNED_BLOCK_WRITE      # 1 cycle - write block flag
    add     rcx, r9                           # 1 cycle - add flag
    add     rcx, rax                          # 1 cycle - add aligned address

    mov     [r12 + r13 * 8 + 8], rcx          # ~4 cycles - write block entry

    # PRE read entry (first qword contains unaligned start)
    mov     rcx, MOPS_ALIGNED_READ            # 1 cycle - read flag
    add     rcx, rax                          # 1 cycle - add aligned address
    mov     [r12 + r13 * 8], rcx              # ~4 cycles - write pre-read entry
    add     r13, 2                            # 1 cycle - advance index by 2

    jmp     fast_memset                       # tail call to fast_memset

    # fast_memset "execute" the return, memset set rax = rdi

    # ========== BRANCH 2.2 ==========
    # dst unaligned AND end unaligned (spans multiple partial qwords)
    # => 2 pre-reads (PRE + POST) + 1 write block

.L_branch_2_2:

    # rcx = (rdi & 0x7) + rdx + 7  →  (rcx >> 3) = qwords needed
    shr     rcx, 3                            # 1 cycle - rcx = qwords
    shl     rcx, MOPS_BLOCK_WORDS_RS          # 1 cycle - format for mops
    mov     rax, MOPS_ALIGNED_BLOCK_WRITE     # 1 cycle - write block flag
    add     rax, rcx                          # 1 cycle - add qword count
    mov     rcx, rdi
    and     rcx, ALIGN_MASK                   # 1 cycle - rcx = aligned dst
    add     rax, rcx                          # 1 cycle - add aligned address

    mov     [r12 + r13 * 8 + 16], rax         # ~4 cycles - write block entry (3rd slot)

    # PRE read entry (first partial qword)
    mov     rax, MOPS_ALIGNED_READ            # 1 cycle - read flag
    add     rcx, rax                          # 1 cycle - rcx = aligned dst + read flag
    mov     [r12 + r13 * 8], rcx              # ~4 cycles - write PRE read entry

    # POST read entry (last partial qword)
    lea     r9, [rdi + rdx]                   # 1 cycle - r9 = dst + count
    and     r9, ALIGN_MASK                    # 1 cycle - align to qword
    add     r9, rax                           # 1 cycle - add read flag
    mov     [r12 + r13 * 8 + 8], r9           # ~4 cycles - write POST read entry

    add     r13, 3                            # 1 cycle - advance index by 3

    jmp     fast_memset                       # tail call to fast_memset

    # fast_memset "execute" the return, memset set rax = rdi

.L_xmemset_mops_done:

    mov     rax, rdi
    ret                                       # ~3 cycles


################################################################################
# PERFORMANCE ESTIMATES (Modern x86-64, L1 cache hits)
#
# FAST PATH (aligned dst, count multiple of 8):
#   - Mops entry:                   ~8-10 cycles
#   - fast_memset overhead:         ~5-10 cycles
#   - Qword fill (rep stosq):       ~0.5-1.0 cycles/qword (ERMSB)
#   Total: ~15-20 cycles + ~0.75 cycles/qword
#
# BRANCH 1 (aligned dst, count NOT multiple of 8):
#   - Pre-read + block entries:     ~15-18 cycles
#   - fast_memset + fill:           ~5-10 cycles + ~0.75 cycles/qword
#   Total: ~25-30 cycles + ~0.75 cycles/qword
#
# BRANCH 2.1 (unaligned dst, end aligned or single qword):
#   - PRE read + block entries:     ~18-22 cycles
#   - fast_memset + fill:           ~5-10 cycles + ~0.75 cycles/qword
#   Total: ~28-35 cycles + ~0.75 cycles/qword
#
# BRANCH 2.2 (unaligned dst AND end):
#   - PRE + POST + block entries:   ~25-30 cycles
#   - fast_memset + fill:           ~5-10 cycles + ~0.75 cycles/qword
#   Total: ~35-45 cycles + ~0.75 cycles/qword
#
# EXAMPLE (64-byte aligned fill):
#   ~20 cycles mops + ~10 cycles setup + 8 qwords * 0.75 = ~36 cycles
#   Throughput: ~1.8 GB/s @ 3 GHz
#
# EXAMPLE (4096-byte aligned fill):
#   ~20 cycles mops + ~10 cycles setup + 512 qwords * 0.5 = ~286 cycles
#   Throughput: ~14.3 GB/s @ 3 GHz (approaching L1D bandwidth)
#
# NOTES:
#   - Assumes L1D cache hits (~4 cycle latency)
#   - rep stosq uses ERMSB optimization on modern CPUs (post-2013)
#   - For fills >256 bytes, approaches memory bandwidth limits
#   - Actual cycles vary ±20% by microarchitecture
################################################################################

# Mark stack as non-executable (required by modern linkers)
.section .note.GNU-stack,"",%progbits
