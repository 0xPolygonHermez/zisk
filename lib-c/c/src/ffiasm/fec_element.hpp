#ifndef FEC_ELEMENT_HPP
#define FEC_ELEMENT_HPP

#include <cstdint>

#define Fec_N64 4
#define Fec_SHORT           0x00000000
#define Fec_MONTGOMERY      0x40000000
#define Fec_SHORTMONTGOMERY 0x40000000
#define Fec_LONG            0x80000000
#define Fec_LONGMONTGOMERY  0xC0000000

typedef uint64_t FecRawElement[Fec_N64];

typedef struct __attribute__((__packed__)) {
    int32_t shortVal;
    uint32_t type;
    FecRawElement longVal;
} FecElement;

typedef FecElement *PFecElement;

#endif // FEC_ELEMENT_HPP
