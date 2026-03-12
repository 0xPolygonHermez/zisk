.intel_syntax noprefix
.code64

.section .text

.macro ABI_WRAPPER abi_call asm_call
.global \abi_call
.extern \asm_call

\abi_call:
    push    r12                       # 1 cycle - save callee-saved register
    push    r13                       # 1 cycle - save callee-saved register
    push    r9                        # 1 cycle - save caller-saved register (used internally)
    push    rbx                       # 1 cycle - save callee-saved register
    
    mov     r12, rcx                  # 1 cycle - setup mtrace address from count parameter
    mov     r13, 1                    # 1 cycle - initialize mtrace index to 1, first position for count
    call    \asm_call                 # ~3 cycles + function cost

    dec     r13
    mov     [r12], r13                # store in first position the length
    pop     rbx                       # 1 cycle - restore register
    pop     r9                        # 1 cycle - restore register
    pop     r13                       # 1 cycle - restore register
    pop     r12                       # 1 cycle - restore register

    ret      
.endm

# [memcpy]
# PARAMETERS (System V AMD64 ABI):
#   rdi = dst
#   rsi = src
#   rdx = count
#   rcx = mtrace_ptr
# RETURN: rax = dst

ABI_WRAPPER test_asm_dma_memcpy_mops direct_dma_memcpy_mops
ABI_WRAPPER test_asm_dma_memcpy_mtrace direct_dma_memcpy_mtrace

# [memcmp]
# PARAMETERS (System V AMD64 ABI):
#   rdi = dst 
#   rsi = src
#   rdx = count
#   rcx = mtrace_ptr
# RETURN: rax = result compare

ABI_WRAPPER test_asm_dma_memcmp_mops direct_dma_memcmp_mops
ABI_WRAPPER test_asm_dma_memcmp_mtrace direct_dma_memcmp_mtrace

# [memset]
# PARAMETERS (System V AMD64 ABI):
#   rdi = dst 
#   rsi = byte
#   rdx = count
#   rcx = mtrace_ptr
# RETURN: rax = result compare


ABI_WRAPPER test_asm_dma_memset_mops direct_dma_xmemset_mops
ABI_WRAPPER test_asm_dma_memset_mtrace direct_dma_xmemset_mtrace

# [inputcpy]
# PARAMETERS (System V AMD64 ABI):
#   rdi = dst 
#   rsi = 0
#   rdx = count
#   rcx = mtrace_ptr
# RETURN: rax = result compare

ABI_WRAPPER test_asm_dma_inputcpy_mops direct_dma_inputcpy_mops
ABI_WRAPPER test_asm_dma_inputcpy_mtrace direct_dma_inputcpy_mtrace

.section .note.GNU-stack,"",%progbits
