#ifndef PSECP256R1_ELEMENT_HPP
#define PSECP256R1_ELEMENT_HPP

#include <cstdint>

#define pSecp256r1_N64 4
#define pSecp256r1_SHORT           0x00000000
#define pSecp256r1_MONTGOMERY      0x40000000
#define pSecp256r1_SHORTMONTGOMERY 0x40000000
#define pSecp256r1_LONG            0x80000000
#define pSecp256r1_LONGMONTGOMERY  0xC0000000

typedef uint64_t pSecp256r1RawElement[pSecp256r1_N64];

typedef struct __attribute__((__packed__)) {
    int32_t shortVal;
    uint32_t type;
    pSecp256r1RawElement longVal;
} pSecp256r1Element;

typedef pSecp256r1Element *PpSecp256r1Element;

#endif // PSECP256R1_ELEMENT_HPP
