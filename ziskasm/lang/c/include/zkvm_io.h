/*
 * zkvm_io.h — Ethereum Foundation zkVM I/O C interface. Mirrors the standard at:
 *   github.com/eth-act/zkevm-standards
 *
 * The private-input and public-output channels. Note these two symbols are NOT
 * `zkvm_`-prefixed: the standard names them plainly.
 *
 * ZisK implements both with no C marshalling layer, exactly like the accelerators:
 * each is a zkvmcall thunk in src/zkvm_calls.s (`csrs <id>, x0; ret`), which the
 * transpiler turns into a jump to `zisklib_read_input` / `zisklib_write_output` in
 * ziskasm/zisklib/zkvm_io.zisk. A guest compiles against this header and links
 * `zisklib_c` (see CMakeLists.txt).
 *
 * Running such a guest needs ziskemu/cargo-zisk built with --features ziskasm.
 * Without it, the transpiler rejects the ELF, rather than letting these two look
 * like "empty input" or "output discarded".
 */
#ifndef ZKVM_IO_H
#define ZKVM_IO_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Reports the private input record: `*buf_ptr` = &input[0] (or NULL when the input
 * is empty) and `*buf_size` = its length in bytes. The buffer is read-only and
 * lives for the whole run. Never fails; idempotent — repeated calls report the same
 * record and have no side effects. */
void read_input(const uint8_t** buf_ptr, size_t* buf_size);

/* Appends `size` bytes to the public output. Successive calls concatenate into one
 * byte stream, so a partial trailing word is carried across calls. Never fails. */
void write_output(const uint8_t* output, size_t size);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* ZKVM_IO_H */
