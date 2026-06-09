#ifndef BLS12_381_ELEMENT_HPP
#define BLS12_381_ELEMENT_HPP

#include <cstdint>

#define BLS12_381_N64 4
#define BLS12_381_SHORT           0x00000000
#define BLS12_381_MONTGOMERY      0x40000000
#define BLS12_381_SHORTMONTGOMERY 0x40000000
#define BLS12_381_LONG            0x80000000
#define BLS12_381_LONGMONTGOMERY  0xC0000000

typedef uint64_t BLS12_381RawElement[BLS12_381_N64];

typedef struct __attribute__((__packed__)) {
    int32_t shortVal;
    uint32_t type;
    BLS12_381RawElement longVal;
} BLS12_381Element;

typedef BLS12_381Element *PBLS12_381Element;

#endif // BLS12_381_ELEMENT_HPP
