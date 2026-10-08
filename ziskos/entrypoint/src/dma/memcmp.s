        .section ".note.GNU-stack","",@progbits
        .text
        .attribute      4, 16
        .attribute      5, "rv64im"
        .globl  memcmp
        .p2align        4
        .type   memcmp,@function
memcmp:
        csrrs   a0,0x814, a1  # DMA memcmp: result -> a0, src -> a1
        add	x0,a0,a2      # Marker: dst (a0), count (a2)       
        ret
               
        .size memcmp, .-memcmp
        .section .text.hot,"ax",@progbits