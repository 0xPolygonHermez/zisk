#ifndef FNEC_ELEMENT_HPP
#define FNEC_ELEMENT_HPP

#include <cstdint>

#define Fnec_N64 4
#define Fnec_SHORT           0x00000000
#define Fnec_MONTGOMERY      0x40000000
#define Fnec_SHORTMONTGOMERY 0x40000000
#define Fnec_LONG            0x80000000
#define Fnec_LONGMONTGOMERY  0xC0000000

typedef uint64_t FnecRawElement[Fnec_N64];

typedef struct __attribute__((__packed__)) {
    int32_t shortVal;
    uint32_t type;
    FnecRawElement longVal;
} FnecElement;

typedef FnecElement *PFnecElement;

#endif // FNEC_ELEMENT_HPP
