#pragma once
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Write public output.
 *
 * Appends `size` bytes from `output` to the public output stream.
 * May be called multiple times; successive calls concatenate their buffers.
 * Cannot fail.
 */
void write_output(const uint8_t* output, size_t size);

#ifdef __cplusplus
}
#endif
