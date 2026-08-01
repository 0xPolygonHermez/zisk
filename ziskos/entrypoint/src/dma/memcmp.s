        .section ".note.GNU-stack","",@progbits
        .text
        .attribute      4, 16
        .attribute      5, "rv64im"
        .globl  memcmp
        .p2align        4
        .type   memcmp,@function
memcmp:
        csrrs   a0,0x814, a1  # Marker: Write count (a2) to CSR 0x814
        add	x0,a0,a2        
        ret
               
        .size memcmp, .-memcmp
        .section .text.hot,"ax",@progbits