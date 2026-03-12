        .section ".note.GNU-stack","",@progbits
        .text
        .attribute      4, 16
        .attribute      5, "rv64im"
        .globl  inputcpy
        .p2align        4
        .type   inputcpy,@function
inputcpy:
        csrs    0x815, a1                  # Marker: Write count (a2) to CSR 0x813
        add	x0,a0,a2
        ret

        .size inputcpy, .-inputcpy
        .section .text.hot,"ax",@progbits