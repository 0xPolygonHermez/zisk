.intel_syntax noprefix
.code64

.section .text

# Calls a jump_dest routine from a System V caller, the way the DMA test
# api does: the trace pointer arrives in rcx, entries are written from index 1
# and slot 0 receives the number of entries written.
#
# PARAMETERS (System V AMD64 ABI):
#   rdi = dst (bitmap)
#   rsi = src (bytecode)
#   rdx = count
#   rcx = trace pointer
# RETURN: rax = dst

.macro ABI_WRAPPER abi_call asm_call
.global \abi_call
.extern \asm_call

\abi_call:
    push    r12
    push    r13
    push    r14
    push    rbx

    mov     r12, rcx                  # trace base
    mov     r13, 1                    # slot 0 holds the entry count
    mov     r14, 1024                 # steps remaining, read only by the realloc path
    call    \asm_call

    dec     r13
    mov     [r12], r13

    pop     rbx
    pop     r14
    pop     r13
    pop     r12
    ret
.endm

ABI_WRAPPER test_asm_jump_dest_fast   jump_dest_fast
ABI_WRAPPER test_asm_jump_dest_mtrace direct_jump_dest_mtrace
ABI_WRAPPER test_asm_jump_dest_mops   direct_jump_dest_mops

# The _with_count_check entries only add the shared mtrace space check in front
# of the same body; exercised to keep that path linked and callable.
ABI_WRAPPER test_asm_jump_dest_mtrace_checked direct_jump_dest_mtrace_with_count_check
ABI_WRAPPER test_asm_jump_dest_mops_checked   direct_jump_dest_mops_with_count_check

.section .note.GNU-stack,"",%progbits
