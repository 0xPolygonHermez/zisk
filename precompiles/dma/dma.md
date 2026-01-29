## Translate

ELF

ADD X0, DST, SRC        ==>  DMA_MEMCPY DST, SRC (JMP +8)
CSRW XXX, COUNT         ==>  FLAG COUNT


DST_PRE_DATA
DST_POST_DATA
SRC_DATA ..... (without redundancy)


extra param count 