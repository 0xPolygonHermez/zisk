// The thunk form of write_log, for write_log_guest.c: with ZKVM_IO_CALLS,
// zkvm_io.h only declares write_log, which is then called through its thunk in
// src/zkvm_calls.s.
#define ZKVM_IO_CALLS
#include "zkvm_io.h"

void log_through_thunk(const uint8_t* utf8_bytes, size_t utf8_len) {
    write_log(utf8_bytes, utf8_len);
}
