/*
 * zkvm_io.h — Ethereum Foundation zkVM I/O C interface. Mirrors the standard at:
 *   github.com/eth-act/zkevm-standards
 *
 * The private-input and public-output channels. Note these two symbols are NOT
 * `zkvm_`-prefixed: the standard names them plainly, and elf2rom's REDIRECTS table
 * matches those exact names.
 *
 * ZisK implements both with no C marshalling layer, exactly like the accelerators:
 * src/zkvm_stubs.c carries an exported placeholder body per symbol, and at transpile
 * time elf2rom redirects `read_input` / `write_output` DIRECTLY to
 * `zisklib_read_input` / `zisklib_write_output` in ziskasm/zisklib/zkvm_io.zisk.
 * A guest compiles against this header and links src/zkvm_stubs.c (built into
 * `zisklib_c` by CMakeLists.txt).
 *
 * If a placeholder body ever executes, the redirect did not fire; it prints a
 * diagnostic naming the symbol and faults rather than returning silently, which for
 * these two would otherwise look like "empty input" or "output discarded".
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
