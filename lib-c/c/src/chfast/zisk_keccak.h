#ifndef ZISK_KECCAK_H
#define ZISK_KECCAK_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

void zisk_keccakf1600(uint64_t state[25]);

#ifdef __cplusplus
}
#endif

#endif // ZISK_KECCAK_H