/**
 * zkVM IO C Interface
 *
 * This header defines the standard C interface for guest programs to access
 * private input and write public output.
 *
 * The functions follow:
 * https://github.com/eth-act/zkvm-standards/tree/main/standards/io-interface
 */

#ifndef ZKVM_IO_H
#define ZKVM_IO_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Return the private input buffer.
 *
 * The returned pointer is read-only from the guest's perspective. The function
 * is idempotent and may be called multiple times.
 *
 * On ZisK this exposes the first logical stdin record and does not advance
 * ZisK's streaming input cursor. Guest programs should use either this standard
 * IO interface or ZisK's streaming input APIs for a given input, not both.
 *
 * @param[out] buf_ptr Pointer receiving the input buffer address
 * @param[out] buf_size Pointer receiving the input buffer size in bytes
 */
void read_input(const uint8_t** buf_ptr, size_t* buf_size);

/**
 * Append bytes to the public output.
 *
 * Multiple calls are observed as if their byte buffers were concatenated.
 *
 * @param output Pointer to readable bytes
 * @param size Number of bytes to append
 */
void write_output(const uint8_t* output, size_t size);

#ifdef __cplusplus
}
#endif

#endif /* ZKVM_IO_H */
