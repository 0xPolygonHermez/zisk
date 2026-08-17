#ifndef EMULATOR_ASM_TRACE_LOGS_HPP
#define EMULATOR_ASM_TRACE_LOGS_HPP

#include <stdint.h>

void log_minimal_trace(void);
void log_histogram(void);
void log_mem_op(void);
void save_mem_op_to_files(void);

#endif // EMULATOR_ASM_TRACE_LOGS_HPP