#ifndef DOT_F8_H
#define DOT_F8_H

#include <stdint.h>

// Returned in rax (total) and rdx (words appended to the minimal trace)
typedef struct
{
    int64_t total;
    uint64_t words;
} dot_f8_mtrace_ret;

// Computes the dot product described by the parameter block p (see dot_f8.c), writes the block
// sums and returns the total.
int64_t dot_f8_compute(const uint64_t *p);

// dot_f8_compute, also writing the minimal trace of the call at trace.
dot_f8_mtrace_ret dot_f8_mtrace_c(const uint64_t *p, uint64_t *trace);

#endif
