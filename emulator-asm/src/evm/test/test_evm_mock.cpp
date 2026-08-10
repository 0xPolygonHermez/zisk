// Globals the shared check_dynamic_mtrace refers to. The threshold is set out
// of reach so its fast path always returns without touching the realloc path,
// which is the emulator's job and not under test here.

#include <cstdint>

extern "C" {
uint64_t trace_address_threshold = UINT64_MAX;
}

extern "C" void _realloc_trace(void) {}
