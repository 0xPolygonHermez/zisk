/* Runs the RISC-V instructions whose ZisK expansion uses the transpiler's scratch
   registers r32 / r33: atomics (AMO*) and the Zbb / Zbs / Zbkb bit-manipulation
   instructions, over a spread of operands. (The CSR instructions use them too, but
   ziskemu faults on mscratch accesses, so they are not covered.) Every result is
   folded into a checksum per group; check.sh compares the output with a golden
   snapshot. Build with -march=rv64ima_zicsr_zbb_zbs_zbkb. */
#include <stdint.h>

static uint64_t acc[8];
static void fold(int k, uint64_t v) { acc[k] = (acc[k] ^ v) * 0x100000001b3ULL + (acc[k] >> 29); }

#define R1(op, x)    ({ uint64_t _r; __asm__ volatile(op " %0, %1" : "=r"(_r) : "r"(x)); _r; })
#define R2(op, x, y) ({ uint64_t _r; __asm__ volatile(op " %0, %1, %2" : "=r"(_r) : "r"(x), "r"(y)); _r; })
#define RI(op, x, i) ({ uint64_t _r; __asm__ volatile(op " %0, %1, " #i : "=r"(_r) : "r"(x)); _r; })

static volatile uint64_t mem64;
static volatile uint32_t mem32;

int main(void) {
    uint64_t x = 0x0123456789abcdefULL, y = 0xfedcba9876543210ULL;
    for (int n = 0; n < 200; n++) {
        uint64_t s = (uint64_t)n;
        // --- atomics, 64 and 32 bit ---
        mem64 = x; mem32 = (uint32_t)y;
        fold(0, __atomic_fetch_add(&mem64, y, __ATOMIC_SEQ_CST));
        fold(0, __atomic_fetch_xor(&mem64, x, __ATOMIC_SEQ_CST));
        fold(0, __atomic_fetch_and(&mem64, y | s, __ATOMIC_SEQ_CST));
        fold(0, __atomic_fetch_or(&mem64, x >> 3, __ATOMIC_SEQ_CST));
        fold(0, __atomic_exchange_n(&mem64, y, __ATOMIC_SEQ_CST));
        fold(0, mem64);
        fold(1, __atomic_fetch_add(&mem32, (uint32_t)x, __ATOMIC_SEQ_CST));
        fold(1, __atomic_fetch_xor(&mem32, (uint32_t)(y >> 7), __ATOMIC_SEQ_CST));
        fold(1, __atomic_exchange_n(&mem32, (uint32_t)s, __ATOMIC_SEQ_CST));
        { uint64_t r; __asm__ volatile("amomax.d %0, %1, (%2)" : "=r"(r) : "r"(x), "r"(&mem64) : "memory"); fold(1, r); }
        { uint64_t r; __asm__ volatile("amominu.d %0, %1, (%2)" : "=r"(r) : "r"(y), "r"(&mem64) : "memory"); fold(1, r); }
        { uint64_t r; __asm__ volatile("amomin.w %0, %1, (%2)" : "=r"(r) : "r"(x), "r"(&mem32) : "memory"); fold(1, r); }
        { uint64_t r; __asm__ volatile("amomaxu.w %0, %1, (%2)" : "=r"(r) : "r"(y), "r"(&mem32) : "memory"); fold(1, r); }
        fold(1, mem64 + mem32);
        // --- Zbb ---
        fold(3, R2("andn", x, y)); fold(3, R2("orn", x, y)); fold(3, R2("xnor", x, y));
        fold(3, R2("rol", x, y)); fold(3, R2("ror", x, y)); fold(3, RI("rori", x, 13));
        fold(3, R2("rolw", x, y)); fold(3, R2("rorw", x, y)); fold(3, RI("roriw", x, 7));
        fold(4, R1("clz", x >> (s & 63))); fold(4, R1("ctz", x << (s & 63)));
        fold(4, R1("clzw", y >> (s & 31))); fold(4, R1("ctzw", y << (s & 31)));
        fold(4, R1("cpop", x)); fold(4, R1("cpopw", y)); fold(4, R1("orc.b", x & (y << 5)));
        fold(4, R1("rev8", x)); fold(4, R2("min", x, y)); fold(4, R2("maxu", x, y));
        fold(4, R1("sext.b", y)); fold(4, R1("sext.h", x)); fold(4, R1("zext.h", y));
        // --- Zbs ---
        fold(5, R2("bclr", x, y)); fold(5, R2("bset", x, y)); fold(5, R2("binv", x, y)); fold(5, R2("bext", x, y));
        fold(5, RI("bclri", x, 17)); fold(5, RI("bseti", y, 41)); fold(5, RI("binvi", x, 63)); fold(5, RI("bexti", y, 3));
        // --- Zbkb ---
        fold(6, R2("pack", x, y)); fold(6, R2("packh", x, y)); fold(6, R2("packw", x, y)); fold(6, R1("brev8", x));
        // next operands
        x = x * 6364136223846793005ULL + 1442695040888963407ULL;
        y ^= (y << 13); y ^= (y >> 7); y ^= (y << 17);
        if ((n & 15) == 0) { x &= 0xffff; y |= 0x8000000000000000ULL; }
    }
    volatile uint64_t *o = (volatile uint64_t *)0xA0410000ULL;
    for (int i = 0; i < 8; i++) o[i] = acc[i];
    return 0;
}
