        .section ".note.GNU-stack","",@progbits
        .text
        .attribute      4, 16
        .attribute      5, "rv64im"
        .globl  memmove
        .p2align        4
        .type   memmove,@function
# memmove(dst=a0, src=a1, count=a2) uses the same DMA op as memcpy (CSR 0x813).
# That op has memmove semantics: when src and dst overlap, the emulator copies via
# a temporary buffer (Mem::memcpy in core/src/mem.rs) and the prover constrains the
# overlapping case too, so a single op is correct for every overlap direction.
memmove:
        csrs    0x813, a1                  # DMA memcpy/memmove: src -> CSR 0x813
        add	x0,a0,a2                   # Marker: dst (a0), count (a2)
        ret        
        .size memmove, .-memmove
        .section .text.hot,"ax",@progbits