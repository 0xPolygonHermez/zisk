        .section ".note.GNU-stack","",@progbits
        .text
        .attribute      4, 16
        .attribute      5, "rv64im"
        .globl  memcpy
        .p2align        4
        .type   memcpy,@function
memcpy:
        csrs    0x813, a2                  # Marker: Write count (a2) to CSR 0x813
        add	x0,a0,a1
        ret

        .size memcpy, .-memcpy
        .section .text.hot,"ax",@progbits