#ifndef BLS12_381_384_ELEMENT_HPP
#define BLS12_381_384_ELEMENT_HPP

#include <cstdint>

#define BLS12_381_384_N64 6
#define BLS12_381_384_SHORT           0x00000000
#define BLS12_381_384_MONTGOMERY      0x40000000
#define BLS12_381_384_SHORTMONTGOMERY 0x40000000
#define BLS12_381_384_LONG            0x80000000
#define BLS12_381_384_LONGMONTGOMERY  0xC0000000

typedef uint64_t BLS12_381_384RawElement[BLS12_381_384_N64];

typedef struct __attribute__((__packed__)) {
    int32_t shortVal;
    uint32_t type;
    BLS12_381_384RawElement longVal;
} BLS12_381_384Element;

typedef BLS12_381_384Element *PBLS12_381_384Element;

#endif // BLS12_381_384_ELEMENT_HPP
