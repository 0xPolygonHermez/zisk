#ifndef LIB_C_BLAKE3_HPP
#define LIB_C_BLAKE3_HPP

#include <stdint.h> // uint32_t

#ifdef __cplusplus
extern "C" {
#endif

void blake3_f(uint32_t v[16], const uint32_t m[16]);

#ifdef __cplusplus
}
#endif

#endif // LIB_C_BLAKE3_HPP
