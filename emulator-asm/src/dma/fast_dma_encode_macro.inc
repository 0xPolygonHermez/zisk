.intel_syntax noprefix
.code64

.extern fast_dma_encode_no_src_table
.extern fast_dma_encode_table
.extern fast_dma_encode_memcmp_table

.macro FAST_DMA_ENCODE
    mov     rax, rdi
    and     rax, 0x07               # dst_offset (0-7)
    shl     rax, 7                  # dst_offset << 7

    mov     r8, rsi
    and     r8, 0x07                # src_offset (0-7)
    shl     r8, 4                   # src_offset << 4

    or      rax, r8                 # combine dst and src offsets

    # Calculate table_count
    mov     r8, rdx
    cmp     r8, 16
    jb      1f
    
    # count >= 16: table_count = (count & 0x07) | 0x08
    and     r8, 0x07
    or      r8, 0x08

1:
    or      rax, r8                 # rax = index = (dst<<7) + (src<<4) + table_count
    
    # Look up encoded value in table (direct access since it's in the same file)
    mov     rax, [fast_dma_encode_table + rax * 8]

    # Add (count >> 3) to result
    mov     r8, rdx
    shl     r8, DMA_LPRE_COUNT_RS   
    add     rax, r8
.endm

.macro FAST_DMA_ENCODE_NO_SRC
    mov     rax, rdi
    and     rax, 0x07               # dst_offset (0-7)
    shl     rax, 4                  # dst_offset << 4

    # Calculate table_count
    mov     r8, rdx
    cmp     r8, 16
    jb      1f
    
    # count >= 16: table_count = (count & 0x07) | 0x08
    and     r8, 0x07
    or      r8, 0x08

1:
    or      rax, r8                 # rax = index = (dst<<7) + (src<<4) + table_count
    
    mov     rax, [fast_dma_encode_no_src_table + rax * 8]

    # Add (count >> 3) to result
    mov     r8, rdx
    shl     r8, DMA_LPRE_COUNT_RS   
    add     rax, r8
.endm


.macro FAST_DMA_ENCODE_MEMCMP extra_table_offset=0
    mov     rax, rdi
    and     rax, 0x07               # dst_offset (0-7)
    shl     rax, 7                  # dst_offset << 7

    mov     r8, rsi
    and     r8, 0x07                # src_offset (0-7)
    shl     r8, 4                   # src_offset << 4

    or      rax, r8                 # combine dst and src offsets

    # Calculate table_count
    mov     r8, rdx
    cmp     r8, 16
    jb      1f
    
    # count >= 16: table_count = (count & 0x07) | 0x08
    and     r8, 0x07
    or      r8, 0x08

1:
    or      rax, r8                 # rax = index = (dst<<7) + (src<<4) + table_count
    
    # Look up encoded value in table (direct access since it's in the same file)
    mov     rax, [fast_dma_encode_memcmp_table + rax * 8 + \extra_table_offset * 8]

    # Add (count >> 3) to result
    mov     r8, rdx
    shl     r8, DMA_LPRE_COUNT_RS   
    add     rax, r8
.endm

# always a zero encoded requires a DMA machine, because It is the only machine 
# that can formally verify a DMA operation with count = 0.
.macro FAST_DMA_ENCODE_COUNT_ZERO use_src=1
    mov     rax, rdi
    and     rax, 0x07               # dst_offset (0-7)
    shl     rax, DMA_DST_OFFSET_RS  # dst_offset << 7
 
    .if \use_src
    mov     r8, rsi
    and     r8, 0x07                # src_offset (0-7)
    shl     r8, DMA_SRC_OFFSET_RS   # src_offset << 4
    lea     rax, [rax + r8 + DMA_REQUIRES_DMA_MASK]
    .else
    or      rax, DMA_REQUIRES_DMA_MASK
    .endif
.endm
   