#ifndef __TEST_MOCK__HPP__
#define __TEST_MOCK__HPP__

#include <stdint.h>
#include <stdio.h>

#define FCALL_PARAMS_LENGTH 386
#define FCALL_RESULT_LENGTH 8193
#define FCALL_FUNCTION_ID 0
#define FCALL_PARAMS_CAPACITY (FCALL_FUNCTION_ID + 1)
#define FCALL_PARAMS_SIZE (FCALL_PARAMS_CAPACITY + 1)
#define FCALL_PARAMS (FCALL_PARAMS_SIZE + 1)
#define FCALL_RESULT_CAPACITY (FCALL_PARAMS + FCALL_PARAMS_LENGTH)
#define FCALL_RESULT_SIZE (FCALL_RESULT_CAPACITY + 1)
#define FCALL_RESULT (FCALL_RESULT_SIZE + 1)            // 391
#define FCALL_RESULT_GOT (FCALL_RESULT + FCALL_RESULT_LENGTH) // 8584
#define FCALL_CTX_LENGTH (FCALL_RESULT_GOT + 1)         // 8585

extern "C" {
    extern uint64_t trace_address_threshold;
    extern uint64_t fcall_ctx[FCALL_CTX_LENGTH];
    extern uint64_t MEM_FREE_INPUT;
}

extern "C" void _realloc_trace(void);
#endif