#include "runtime.h"

/* Each import starts fresh; free/coalescing also bounds temporary encoder work. */
#define ARENA_SIZE (8 * 1024 * 1024)
typedef struct Block { size_t size; struct Block *next; size_t free; size_t pad; } Block;
_Alignas(16) static uint8_t arena[ARENA_SIZE];
static size_t live_bytes, peak_bytes;
void runtime_arena_reset(void) {
    Block *first = (Block *)arena;
    first->size = ARENA_SIZE - sizeof(Block);
    first->next = 0;
    first->free = 1;
    live_bytes = peak_bytes = 0;
}
size_t runtime_arena_highwater(void) { return peak_bytes; }
void *runtime_alloc(void *opaque, size_t len) {
    (void)opaque;
    if (len > ARENA_SIZE - sizeof(Block) - 15) runtime_trap();
    len = (len + 15) & ~(size_t)15;
    if (!len) len = 16;
    for (Block *b = (Block *)arena; b; b = b->next) {
        if (!b->free || b->size < len) continue;
        if (b->size >= len + sizeof(Block) + 16) {
            Block *rest = (Block *)((uint8_t *)(b + 1) + len);
            rest->size = b->size - len - sizeof(Block);
            rest->next = b->next;
            rest->free = 1;
            b->next = rest;
            b->size = len;
        }
        b->free = 0;
        live_bytes += b->size + sizeof(Block);
        if (live_bytes > peak_bytes) peak_bytes = live_bytes;
        return b + 1;
    }
    runtime_trap();
}
void runtime_free(void *opaque, void *ptr) {
    (void)opaque;
    if (!ptr) return;
    Block *b = (Block *)ptr - 1;
    live_bytes -= b->size + sizeof(Block);
    b->free = 1;
    for (b = (Block *)arena; b && b->next;) {
        if (b->free && b->next->free) {
            b->size += sizeof(Block) + b->next->size;
            b->next = b->next->next;
        } else b = b->next;
    }
}
#ifndef NITRO_RUNTIME_HOST_TEST
void *malloc(size_t n) { return runtime_alloc(0, n); }
void free(void *p) { runtime_free(0, p); }
_Noreturn void exit(int code) { (void)code; runtime_trap(); }
void *memcpy(void *dst, const void *src, size_t len) {
    uint8_t *d = dst; const uint8_t *s = src;
    for (size_t i = 0; i < len; i++) d[i] = s[i];
    return dst;
}
void *memset(void *dst, int value, size_t len) {
    uint8_t *d = dst;
    for (size_t i = 0; i < len; i++) d[i] = value;
    return dst;
}
int memcmp(const void *a, const void *b, size_t len) {
    const uint8_t *x = a, *y = b;
    for (size_t i = 0; i < len; i++) if (x[i] != y[i]) return (int)x[i] - y[i];
    return 0;
}
void *memmove(void *dst, const void *src, size_t len) {
    uint8_t *d = dst; const uint8_t *s = src;
    if ((uintptr_t)d < (uintptr_t)s) return memcpy(dst, src, len);
    while (len) { len--; d[len] = s[len]; }
    return dst;
}
#endif
