#include <stdint.h>
#include <stdio.h>

#include "test_mock.hpp"

extern "C" {
    uint64_t trace_address_threshold = 0;
    uint64_t fcall_ctx[FCALL_CTX_LENGTH];
    uint64_t MEM_FREE_INPUT = 0;
}

extern "C" void _realloc_trace(void) {

}


