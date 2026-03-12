.intel_syntax noprefix
.code64

################################################################################
# dma_memcpy_fast - Optimized memcpy using rep movsq (no tracing)
#
# Fast memory copy function optimized for performance using hardware-accelerated
# instructions. Handles overlapping memory regions correctly (like memmove).
#
# PARAMETERS (System V AMD64 ABI):
#   rdi = dst (u64)     - Destination address
#   rsi = src (u64)     - Source address  
#   rdx = count (usize) - Number of bytes to copy
#
# RETURN VALUE: None
#
# STRATEGY:
#   For non-overlapping regions:
#     1. Copy pre_count unaligned bytes (0-7 bytes to reach alignment)
#     2. Copy aligned qwords using rep movsq (~1-2 cycles/qword)
#     3. Copy post_count remaining bytes (0-7 bytes)
#   
#   For overlapping regions (dst between src and src+count):
#     - Copy backward byte-by-byte using rep movsb with std flag
#
# PERFORMANCE: ~10-20 cycles overhead + ~1-2 cycles per qword
#
# REGISTERS USED: rax, rcx, rdi, rsi, rdx, r8, r9
################################################################################

.global dma_memcpy_fast

.section .text

dma_memcpy_fast:
    # Check if count is 0
    test    rdx, rdx                # 1 cycle
    jz      .L_fast_done            # nothing to copy
    
    # Save original values
    mov     r8, rdi                 # r8 = dst
    mov     r9, rsi                 # r9 = src
    
    # Check for overlap: if dst < src or dst >= src+count, no overlap
    lea     rax, [rsi + rdx]        # rax = src + count
    cmp     rdi, rsi                # compare dst with src
    jb      .L_fast_forward         # dst < src, copy forward
    cmp     rdi, rax                # compare dst with src+count
    jae     .L_fast_forward         # dst >= src+count, no overlap
    
    # Overlap detected: copy backward
    lea     rsi, [r9 + rdx - 1]     # rsi = src + count - 1 (use r9, original src)
    lea     rdi, [r8 + rdx - 1]     # rdi = dst + count - 1 (use r8, original dst)
    mov     rcx, rdx                # rcx = count
    std                             # set direction flag (backward)
    rep movsb                       # copy backward
    cld                             # clear direction flag
    jmp     .L_fast_done
    
.L_fast_forward:
    # No overlap: optimized 3-phase copy
    # Calculate dst_offset and pre_count
    mov     rax, r8                 # rax = dst
    and     rax, 0x07               # rax = dst_offset
    test    rax, rax                # check if already aligned
    jz      .L_fast_aligned         # skip pre-copy if aligned
    
    # Copy pre_count bytes to align dst
    mov     rcx, 8                  # rcx = 8
    sub     rcx, rax                # rcx = 8 - dst_offset = pre_count
    cmp     rcx, rdx                # check if pre_count > count
    jbe     .L_fast_pre_ok          # pre_count <= count
    mov     rcx, rdx                # pre_count = count (copy all)
.L_fast_pre_ok:
    sub     rdx, rcx                # count -= pre_count
    rep movsb                       # copy pre_count bytes
    # rsi and rdi are now advanced and rdi is aligned
    
.L_fast_aligned:
    # Copy aligned qwords using rep movsq
    mov     rcx, rdx                # rcx = remaining count
    shr     rcx, 3                  # rcx = count / 8 (qword count)
    jz      .L_fast_post            # skip if no qwords to copy
    rep movsq                       # copy qwords (~1-2 cycles each)
    
.L_fast_post:
    # Copy remaining bytes (0-7)
    mov     rcx, rdx                # rcx = original count
    and     rcx, 0x07               # rcx = count % 8 (post_count)
    jz      .L_fast_done            # skip if no remaining bytes
    rep movsb                       # copy remaining bytes
    
.L_fast_done:
    ret

# Mark stack as non-executable (required by modern linkers)
.section .note.GNU-stack,"",%progbits
