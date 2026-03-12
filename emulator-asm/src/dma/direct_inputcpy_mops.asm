.intel_syntax noprefix
.code64

################################################################################
# inputcpy_mops - Optimized inputcpy with memory ops tracing
#
# This function performs two main tasks:
# 1. Records all addresses of memory operations (read and write addresses)
# 2. Performs the actual inputcpy operation filling with free-inputs
#
# REGISTER USAGE:
# Uses general-purpose registers: rax, rbx, rcx, rdx, rdi, rsi, r8, r9, r12, r13
# Does NOT use XMM registers (caller doesn't need to save them)
# Preserves callee-saved registers (rbx, r12, r13 saved/restored in wrapper)
#
# PARAMETERS (NON System V AMD64 ABI):
#   rdi = dst (u64)                     - Destination address to fill
#   rsi = value (u8 in low byte)        - Byte value to set (0-255)
#   rdx = count (usize)                 - Number of bytes to set
#   r12 = mops_base_addr (u64*)         - Pointer to memory ops trace buffer base
#   r13 = mops_index (usize)            - Current index in mops buffer (input/output)
#
################################################################################

.global direct_dma_inputcpy_mops
.extern fast_memcpy
.extern fcall_ctx

.include "dma_constants.inc"

.section .text

# Direct entry point for assembly callers (no ABI overhead)
# More efficient when caller manages register preservation

# arguments:
# rdi: destination adress
# rdx: count (bytes)
# r12 + r13: mops trace

direct_dma_inputcpy_mops:
   
    # Modified registers (caller must handle): 
    #       r9  = scratch for mops address calculation
    #       rcx = mops index (incremented, output)

    # test count = 0
    test    rdx, rdx
    jz      .L_inputcpy_mops_done

    # test dst aligned
    test    rdi, 0x7
    jnz     .L_inputcpy_mops_rdi_unaligned

    # test count multiple of 8
    test    rdx, 0x07
    jnz     .L_inputcpy_mops_count_remain

    # FAST BRANCH
    # dst is aligned, count is a multiple of 8 and greater than zero
    # => no pre-reads, only one MOPS write block

    # FAST BRANCH - MOPS (MOPS_ALIGNED_BLOCK_WRITE)

    mov     rax, rdx
    shr     rax, 3                       
    shl     rax, MOPS_BLOCK_WORDS_RS      # 1 cycle - shift to block words field position
    mov     r9, MOPS_ALIGNED_BLOCK_WRITE  # 1 cycle - rcx = block write flags
    add     r9, rax
    add     r9, rdi                       # rdi aligned 

    mov     [r12 + r13 * 8], r9           # ~4 cycles - write mops entry (block write)
    inc     r13                           # 1 cycle - advance mops index

    jmp     fast_inputcpy
    # fast_inputcpy "execute" the return


.L_inputcpy_mops_count_remain:
    # BRANCH 1
    # dst is aligned, but count is NOT a multiple of 8,
    # => one pre-read (post) before one MOPS write block
    # NOTE: if count < 8 no problem, because you need to do read and write.

    # BRANCH 1 - MOPS (MOPS_ALIGNED_READ (POST) + MOPS_ALIGNED_BLOCK_WRITE)

    # BRANCH 1 - common MOPS part

    lea     r9, [rdx + 7]
    shr     r9, 3                       

    # BRANCH 1 - specific MOPS pre-read part

    lea     rcx, [rdi + r9 * 8 - 8]
    mov     rax, MOPS_ALIGNED_READ
    add     rcx, rax
    mov     [r12 + r13 * 8], rcx

    # BRANCH 1 - specific MOPS block write
    # set rcx = qwords to write

    shl     r9, MOPS_BLOCK_WORDS_RS        # 1 cycle - shift to block words field position
    mov     rax, MOPS_ALIGNED_BLOCK_WRITE  # 1 cycle - rcx = block write flags
    add     rax, r9
    add     rax, rdi                       # rdi is aligned in this path

    mov     [r12 + r13 * 8 + 8], rax        # ~4 cycles - write mops entry (block write)
    add     r13, 2

    jmp     fast_inputcpy
    # fast_inputcpy "execute" the return

.L_inputcpy_mops_rdi_unaligned:
    # BRANCH 2 - worse
    # dst is NOT aligned 
    # => BRANCH 2.1 one pre-read (pre) + no post
    # => BRANCH 2.2 one pre-read (pre) + second post pre-read
    
    # [EC] only PRE but [rdi + rdx] & 0x07 !== 0
    mov     rcx, rdi
    and     rcx, 0x07
    lea     rcx, [rcx + rdx + 7]     # optimization to be used in this branch
    test    rcx, 0xFFFFFFFFFFFFFFF0  # (rcx + rdx) > 8 => (rcx + rdx + 7) > 15 => 
                                     # (rcx + rdx + 7) & 0xF..F0 != 0
    jnz     .L_pre_branch_2_2
    jmp     .L_branch_2_1

.L_pre_branch_2_2:
    lea     rax, [rcx - 7]
    test    rax, 0x7 
    jnz     .L_branch_2_2

.L_branch_2_1:

    # BRANCH 2.1 - MOPS (MOPS_ALIGNED_READ (PRE) + MOPS_ALIGNED_BLOCK_WRITE)

    # BRANCH 2.1 - specific MOPS block write
    # NOTE: at least one qword because count > 0
    # rcx = (rdi & 0x7) + rdx + 7  ==> (rcx >> 3) qwords
    mov     rax, rdi
    and     rax, ALIGN_MASK
    shr     rcx, 3
    shl     rcx, MOPS_BLOCK_WORDS_RS        # 1 cycle - shift to block words field position
    mov     r9, MOPS_ALIGNED_BLOCK_WRITE    # 1 cycle - rcx = block write flags
    add     rcx, r9
    add     rcx, rax

    mov     [r12 + r13 * 8 + 8], rcx        # ~4 cycles - write mops entry (block write)

    # BRANCH 2.1 - specific MOPS pre-read part PRE
    # rax = rdi & ALIGN_MASK

    mov     rcx, MOPS_ALIGNED_READ
    add     rcx, rax
    mov     [r12 + r13 * 8], rcx
    add     r13, 2

    jmp    fast_inputcpy
    # fast_inputcpy "execute" the return

.L_branch_2_2:

    # BRANCH 2.2 - MOPS (2xMOPS_ALIGNED_READ (PRE/POST) + MOPS_ALIGNED_BLOCK_WRITE)
    # BRANCH 2.2 - specific MOPS pre-read part PRE
    # rcx = (rdi & 0x7) + rdx + 7  ==> (rcx >> 3) qwords

    shr     rcx, 3   
    shl     rcx, MOPS_BLOCK_WORDS_RS       # 1 cycle - shift to block words field position
    mov     rax, MOPS_ALIGNED_BLOCK_WRITE  # 1 cycle - rcx = block write flags
    add     rax, rcx                       # rax = MOPS_ALIGNED_BLOCK_WRITE | (count_q <<  MOPS_BLOCK_WORDS_RS)
    mov     rcx, rdi
    and     rcx, ALIGN_MASK
    add     rax, rcx                       # rax |= rdi & ALIGN MASK

    mov     [r12 + r13 * 8 + 16], rax      # ~4 cycles - write mops entry (block write)

    # rcx = rdi & ALIGN_MASK
    # BRANCH 2.2 - PRE write

    mov     rax, MOPS_ALIGNED_READ

    add     rcx, rax                    # rcx = rdi & ALIGN_MASK
    mov     [r12 + r13 * 8], rcx

    # BRANCH 2.2 - POST write

    lea     r9, [rdi + rdx]
    and     r9, ALIGN_MASK
    add     r9, rax
    mov     [r12 + r13 * 8 + 8], r9

    add     r13, 3

    # rsi = input + mops
    # incr mops

    jmp     fast_inputcpy

    # fast_inputcpy "execute" the return

.L_inputcpy_mops_done:
    ret



# Performance estimate (Modern x86-64, Intel Skylake/AMD Zen+, L1 cache hits):
#
# MEMSET OPERATION WITH MOPS TRACING:
# - fast_dma_encode call:           ~15-20 cycles (function call + table lookup)
# - Pre-read mops entry:            ~8-10 cycles (if pre_count > 0: calc + and + store + inc)
# - Post-read mops entry:           ~10-12 cycles (if post_count > 0: lea + and + add + store + inc)
# - Block write mops entry:         ~12-15 cycles (extract + shift + combine + store + inc)
# - Byte value expansion:           ~5-6 cycles (movzx + mov + imul)
# - Qword fill (rep stosq):         ~0.5-1.0 cycles per qword (ERMSB optimization)
# - Remaining bytes (rep stosb):    ~1.0-2.0 cycles per byte (0-7 bytes)
# - Function overhead:              ~3-5 cycles (branches, return)
#
# TOTAL (typical case, 64 bytes, aligned, no pre/post):
#   ~15 (encode) + ~15 (block mops) + ~6 (expand) + 8*0.75 (fill) + ~4 (overhead)
#   = ~46 cycles (~1.39 GB/s @ 3 GHz)
#
# TOTAL (misaligned case, 64 bytes with pre/post):
#   ~15 (encode) + ~10 (pre) + ~12 (post) + ~15 (block) + ~6 (expand) + 7*0.75 + 4*1.5 (fill) + ~4
#   = ~73 cycles (~0.88 GB/s @ 3 GHz)
#
# TOTAL (large fill, 4096 bytes, aligned):
#   ~15 (encode) + ~15 (mops) + ~6 (expand) + 512*0.5 (fill) + ~4 (overhead)
#   = ~296 cycles (~13.8 GB/s @ 3 GHz, approaching L1D bandwidth)
#
# NOTES:
# - Assumes L1D cache hits for all memory accesses (~4 cycle latency, ~64 GB/s bandwidth)
# - rep stosq/stosb uses Enhanced REP MOVSB/STOSB (ERMSB) on modern CPUs (post-2013)
# - ERMSB enables microcode to use wide stores (16-64 bytes per iteration internally)
# - For fills >256 bytes, performance approaches memory bandwidth limits
# - Actual cycles vary ±20-30% by microarchitecture (Skylake/Zen/Alder Lake)
# - Mops overhead: ~30-50 cycles base + minimal per-byte impact
# - No overlap handling needed for inputcpy (writes only, no read-modify-write hazards)

# Mark stack as non-executable (required by modern linkers)
.section .note.GNU-stack,"",%progbits
