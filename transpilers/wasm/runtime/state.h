#ifndef NITRO_RUNTIME_STATE_H
#define NITRO_RUNTIME_STATE_H
#include <stdint.h>
#define STATE_MAX_PREIMAGES 8192u
#define STATE_MAX_MESSAGES 1024u
uint64_t runtime_state(uint64_t op, const uint64_t args[8], uint64_t mem_end);
#endif
