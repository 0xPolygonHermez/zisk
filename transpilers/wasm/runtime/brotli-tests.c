/* Native control tests for the same C helper linked into the RISC-V ROM. */
#include "runtime.h"
#undef NDEBUG
#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <setjmp.h>
#include <brotli/encode.h>

static uint8_t *mem;
static jmp_buf trapped;
static int expect_trap;
void runtime_trap(void) { if (expect_trap) longjmp(trapped, 1); abort(); }
uint8_t *runtime_checked_range(uint64_t off, uint64_t n, uint64_t end) {
    off = (uint32_t)off;
    if (off > end - 0xa1000000 || n > end - 0xa1000000 - off) runtime_trap();
    return mem + off;
}
extern size_t runtime_arena_highwater(void);
static const uint64_t end = 0xa1100000;
static uint64_t call(uint64_t op, uint32_t input, uint32_t len, uint32_t output,
                     uint32_t outlen, uint32_t level, uint32_t window, uint32_t dict) {
    uint64_t args[8] = {input, len, output, outlen, level, window, dict};
    return runtime_brotli(op, args, end);
}
static void length(uint32_t n) { memcpy(mem + 16, &n, 4); }
static uint32_t result_length(void) { uint32_t n; memcpy(&n, mem + 16, 4); return n; }
static size_t reference_compress(const uint8_t *input, size_t len, uint8_t *output, size_t capacity, uint32_t quality) {
    BrotliEncoderState *state = BrotliEncoderCreateInstance(0, 0, 0);
    assert(state);
    assert(BrotliEncoderSetParameter(state, BROTLI_PARAM_QUALITY, quality));
    assert(BrotliEncoderSetParameter(state, BROTLI_PARAM_LGWIN, 22));
    const uint8_t *in = input;
    uint8_t *out = output;
    size_t count = 0;
    assert(BrotliEncoderCompressStream(state, BROTLI_OPERATION_FINISH, &len, &in, &capacity, &out, &count));
    assert(BrotliEncoderIsFinished(state));
    BrotliEncoderDestroyInstance(state);
    return count;
}
int main(void) {
    mem = mmap(0, 1048576, PROT_READ|PROT_WRITE,
               MAP_PRIVATE|MAP_ANON, -1, 0);
    assert(mem != MAP_FAILED);
    const uint8_t hello[] = {0x0b, 0x02, 0x80, 'h', 'e', 'l', 'l', 'o', 0x03};
    memcpy(mem + 128, hello, sizeof(hello));
    length(UINT32_MAX);
    assert(call(20, 128, sizeof(hello), 2048, 16, 0, 0, 0) == 1);
    assert(result_length() == 5 && memcmp(mem + 2048, "hello", 5) == 0);
    const char text[] = "Nitro Brotli native runtime: input input input input";
    memcpy(mem + 128, text, sizeof(text));
    length(512);
    assert(call(21, 128, sizeof(text), 1024, 16, 11, 22, 0) == 1);
    uint32_t compressed = result_length();
    assert(compressed > 0 && compressed < sizeof(text));
    size_t peak = runtime_arena_highwater();
    length(512);
    assert(call(20, 1024, compressed, 2048, 16, 0, 0, 0) == 1);
    assert(result_length() == sizeof(text));
    assert(memcmp(mem + 2048, text, sizeof(text)) == 0);
    /* Publishing through overlapping buffers must preserve the copied input. */
    length(512);
    assert(call(20, 1024, compressed, 1024, 16, 0, 0, 0) == 1);
    assert(memcmp(mem + 1024, text, sizeof(text)) == 0);
    length(512);
    assert(call(21, 128, sizeof(text), 1024, 16, 11, 22, 0) == 1);
    compressed = result_length();
    memset(mem + 2048, 0x5a, 512);
    length(1);
    assert(call(20, 1024, compressed, 2048, 16, 0, 0, 0) == 0);
    assert(result_length() == 1 && mem[2048] == 0x5a);
    length(512);
    assert(call(20, 1024, compressed - 1, 2048, 16, 0, 0, 0) == 0);
    assert(result_length() == 512 && mem[2048] == 0x5a);
    memset(mem + 1024, 0xff, compressed);
    assert(call(20, 1024, compressed, 2048, 16, 0, 0, 0) == 0);
    /* Failed decoding does not touch an invalid output pointer. */
    assert(call(20, 1024, compressed, UINT32_MAX, 16, 0, 0, 0) == 0);
    length(512);
    assert(call(21, 128, sizeof(text), 1024, 16, 11, 22, 0) == 1);
    compressed = result_length();
    length(512);
    expect_trap = 1;
    if (!setjmp(trapped)) {
        call(20, 1024, compressed, 1048570, 16, 0, 0, 0);
        assert(!"successful decoding must check output bounds");
    }
    expect_trap = 0;
    /* Large reserved capacity must not allocate bytes the codec never produces. */
    length(UINT32_MAX);
    assert(call(21, 128, sizeof(text), 2048, 16, 11, 22, 0) == 1);
    assert(result_length() == compressed);
    for (uint32_t i = 0, x = 7; i < 65536; i++) {
        x ^= x << 13; x ^= x >> 17; x ^= x << 5;
        mem[4096 + i] = (uint8_t)x;
    }
    length(70000);
    assert(call(21, 4096, 65536, 131072, 16, 11, 22, 0) == 1);
    size_t large_peak = runtime_arena_highwater();
    compressed = result_length();
    assert(compressed > 65536); /* Forces the encoder to grow beyond its first scratch buffer. */
    uint8_t *reference = malloc(70000);
    assert(reference);
    size_t reference_len = reference_compress(mem + 4096, 65536, reference, 70000, 11);
    assert(reference_len == compressed && !memcmp(reference, mem + 131072, compressed));
    free(reference);
    length(65536);
    assert(call(20, 131072, compressed, 524288, 16, 0, 0, 0) == 1);
    assert(result_length() == 65536 && !memcmp(mem + 4096, mem + 524288, 65536));
    for (uint32_t i = 0, x = 17; i < 131072; i++) {
        x ^= x << 13; x ^= x >> 17; x ^= x << 5;
        mem[4096 + i] = (uint8_t)x;
    }
    compressed = reference_compress(mem + 4096, 131072, mem + 200000, 150000, 0);
    length(UINT32_MAX);
    assert(call(20, 200000, compressed, 524288, 16, 0, 0, 0) == 1);
    assert(result_length() == 131072 && !memcmp(mem + 4096, mem + 524288, 131072));
    memset(mem + 524288, 0x5a, 131072);
    length(65537);
    assert(call(20, 200000, compressed, 524288, 16, 0, 0, 0) == 0);
    assert(result_length() == 65537);
    for (uint32_t i = 0; i < 131072; i++) assert(mem[524288 + i] == 0x5a);
    /* Actual encoder working memory can still exceed the bounded arena. */
    for (uint32_t i = 0, x = 23; i < 262144; i++) {
        x ^= x << 13; x ^= x >> 17; x ^= x << 5;
        mem[4096 + i] = (uint8_t)x;
    }
    length(300000);
    expect_trap = 1;
    if (!setjmp(trapped)) {
        call(21, 4096, 262144, 524288, 16, 11, 22, 0);
        assert(!"internal codec allocation exhaustion must trap");
    }
    expect_trap = 0;
    length(512);
    memcpy(mem + 128, hello, sizeof(hello));
    assert(call(20, 128, sizeof(hello), 2048, 16, 0, 0, 0) == 1);
    printf("brotli large quality11 peak arena %zu bytes\n", large_peak);
    printf("brotli controls passed; quality11 peak arena %zu bytes\n", peak);
    return 0;
}
