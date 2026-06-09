#ifndef NSECP256R1_ELEMENT_HPP
#define NSECP256R1_ELEMENT_HPP

#include <cstdint>

#define nSecp256r1_N64 4
#define nSecp256r1_SHORT           0x00000000
#define nSecp256r1_MONTGOMERY      0x40000000
#define nSecp256r1_SHORTMONTGOMERY 0x40000000
#define nSecp256r1_LONG            0x80000000
#define nSecp256r1_LONGMONTGOMERY  0xC0000000

typedef uint64_t nSecp256r1RawElement[nSecp256r1_N64];

typedef struct __attribute__((__packed__)) {
    int32_t shortVal;
    uint32_t type;
    nSecp256r1RawElement longVal;
} nSecp256r1Element;

typedef nSecp256r1Element *PnSecp256r1Element;

#endif // NSECP256R1_ELEMENT_HPP
