#ifndef DOT_BF16_H
#define DOT_BF16_H

#include <stdint.h>

// Returned in rax (result, f32 bits) and rdx (words appended to the minimal trace)
typedef struct
{
    uint64_t result;
    uint64_t words;
} dot_bf16_mtrace_ret;

// Computes the dot product described by the parameter block p (see dot_bf16.c), writes the block
// results and returns the f32 bits of the total.
uint32_t dot_bf16_compute(const uint64_t *p);

// dot_bf16_compute, also writing the minimal trace of the call at trace.
dot_bf16_mtrace_ret dot_bf16_mtrace_c(const uint64_t *p, uint64_t *trace);

#endif
