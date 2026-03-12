#ifndef EMULATOR_ASM_TRACE_HPP
#define EMULATOR_ASM_TRACE_HPP

#include <stdint.h>

extern uint64_t trace_total_mapped_size; // Total mapped trace size

void trace_map_initialize (void);
void trace_map_next_chunk (void);
void trace_cleanup (void);

#endif // EMULATOR_ASM_TRACE_HPP