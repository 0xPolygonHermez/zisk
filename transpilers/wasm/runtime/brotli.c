#include "runtime.h"
#include <brotli/decode.h>
#include <brotli/encode.h>

void runtime_arena_reset(void);
void *runtime_alloc(void *opaque, size_t len);
void runtime_free(void *opaque, void *ptr);

static uint32_t read_u32(const uint8_t *p) {
    return (uint32_t)p[0] | (uint32_t)p[1] << 8 | (uint32_t)p[2] << 16 | (uint32_t)p[3] << 24;
}

static int grow_output(uint8_t **buffer, size_t *size, size_t capacity,
                       size_t written, uint8_t **out, size_t *left) {
    if (*size == capacity) return 0;
    size_t next_size = *size ? *size * 2 : 1;
    if (next_size > capacity) next_size = capacity;
    uint8_t *next = runtime_alloc(0, next_size);
    memcpy(next, *buffer, written);
    runtime_free(0, *buffer);
    *buffer = next;
    *size = next_size;
    *out = next + written;
    *left = next_size - written;
    return 1;
}

uint64_t runtime_brotli(uint64_t op, const uint64_t args[8], uint64_t mem_end) {
    uint32_t input_len = args[1];
    uint8_t dictionary = (uint8_t)(uint32_t)args[op == 20 ? 4 : 6];
    if (op != 20 && op != 21) runtime_trap();
    /* shortcut: only Nitro's empty dictionary is linked; add Stylus bytes before enabling it. */
    if (dictionary != 0) runtime_trap();
    const uint8_t *source = runtime_checked_range((uint32_t)args[0], input_len, mem_end);
    uint8_t *length_ptr = runtime_checked_range((uint32_t)args[3], 4, mem_end);
    uint32_t capacity = read_u32(length_ptr);
    runtime_arena_reset();
    uint8_t *input = runtime_alloc(0, input_len);
    size_t scratch_size = capacity < 65536 ? capacity : 65536;
    uint8_t *output = runtime_alloc(0, scratch_size);
    if (!input || !output) runtime_trap();
    memcpy(input, source, input_len);
    size_t in_left = input_len, out_left = scratch_size, out_len = 0;
    const uint8_t *in = input;
    uint8_t *out = output;
    int success = 0;
    if (op == 20) {
        BrotliDecoderState *state = BrotliDecoderCreateInstance(runtime_alloc, runtime_free, 0);
        if (state) {
            for (;;) {
                BrotliDecoderResult status = BrotliDecoderDecompressStream(state, &in_left, &in, &out_left, &out, &out_len);
                if (status == BROTLI_DECODER_RESULT_SUCCESS) {
                    success = BrotliDecoderIsFinished(state);
                    break;
                }
                if (status != BROTLI_DECODER_RESULT_NEEDS_MORE_OUTPUT ||
                    !grow_output(&output, &scratch_size, capacity, out_len, &out, &out_left)) break;
            }
            BrotliDecoderDestroyInstance(state);
        }
    } else {
        BrotliEncoderState *state = BrotliEncoderCreateInstance(runtime_alloc, runtime_free, 0);
        if (state) {
            if (BrotliEncoderSetParameter(state, BROTLI_PARAM_QUALITY, (uint32_t)args[4]) &&
                BrotliEncoderSetParameter(state, BROTLI_PARAM_LGWIN, (uint32_t)args[5])) {
                for (;;) {
                    if (!BrotliEncoderCompressStream(state, BROTLI_OPERATION_FINISH, &in_left, &in, &out_left, &out, &out_len)) break;
                    if (BrotliEncoderIsFinished(state)) { success = 1; break; }
                    if (out_left || !grow_output(&output, &scratch_size, capacity, out_len, &out, &out_left)) break;
                }
            }
            BrotliEncoderDestroyInstance(state);
        }
    }
    if (!success) return 0;
    uint8_t *destination = runtime_checked_range((uint32_t)args[2], out_len, mem_end);
    memcpy(destination, output, out_len);
    for (unsigned i = 0; i < 4; i++) length_ptr[i] = (uint8_t)(out_len >> (8 * i));
    return 1;
}
