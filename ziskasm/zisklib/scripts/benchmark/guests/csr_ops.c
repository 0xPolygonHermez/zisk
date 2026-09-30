/* Self-checking test of the CSR instructions (csrrw, csrrs, csrrc and their
   immediate forms) in every register pattern the transpiler special-cases:
   rd = 0, rs1 = 0 (or imm = 0), rd = rs1, and all distinct. Each case checks the
   old CSR value returned in rd and the new CSR value against plain C. Output: one
   byte per case at 0xa0410000, 1 = wrong, and 1 in the byte after the last case
   once every case has run. check.sh compares it with a golden snapshot.
   Build with -march=rv64ima_zicsr. */
#include <stdint.h>

static volatile uint8_t *const O = (volatile uint8_t *)0xA0410000ULL;
static unsigned n;

static void set(uint64_t v) { __asm__ volatile("csrw mscratch, %0" ::"r"(v)); }
static uint64_t get(void) { uint64_t v; __asm__ volatile("csrr %0, mscratch" : "=r"(v)); return v; }
static void check(uint64_t got_rd, uint64_t want_rd, uint64_t want_csr) {
    O[n++] = (got_rd != want_rd) || (get() != want_csr);
}

/* rd, rs1 distinct */
#define OP_RR(op, v0, x, want)                                                        \
    { uint64_t r, s = (x); set(v0);                                                   \
      __asm__ volatile(op " %0, mscratch, %1" : "=&r"(r) : "r"(s)); check(r, v0, want); }
/* rd = rs1 (same register) */
#define OP_SAME(op, v0, x, want)                                                      \
    { uint64_t r = (x); set(v0);                                                      \
      __asm__ volatile(op " %0, mscratch, %0" : "+r"(r)); check(r, v0, want); }
/* rd = x0 */
#define OP_RD0(op, v0, x, want)                                                       \
    { uint64_t s = (x); set(v0);                                                      \
      __asm__ volatile(op " x0, mscratch, %0" ::"r"(s)); check(v0, v0, want); }
/* rs1 = x0 */
#define OP_RS0(op, v0, want)                                                          \
    { uint64_t r; set(v0);                                                            \
      __asm__ volatile(op " %0, mscratch, x0" : "=r"(r)); check(r, v0, want); }
/* immediate forms */
#define OP_I(op, v0, imm, want)                                                       \
    { uint64_t r; set(v0);                                                            \
      __asm__ volatile(op " %0, mscratch, " #imm : "=r"(r)); check(r, v0, want); }
#define OP_I_RD0(op, v0, imm, want)                                                   \
    { set(v0); __asm__ volatile(op " x0, mscratch, " #imm); check(v0, v0, want); }

int main(void) {
    const uint64_t v = 0x0123456789abcdefULL, x = 0xf0f0f0f00f0f0f0fULL;
    OP_RR("csrrw", v, x, x);        OP_SAME("csrrw", v, x, x);
    OP_RD0("csrrw", v, x, x);       OP_RS0("csrrw", v, 0);
    OP_RR("csrrs", v, x, v | x);    OP_SAME("csrrs", v, x, v | x);
    OP_RD0("csrrs", v, x, v | x);   OP_RS0("csrrs", v, v);
    OP_RR("csrrc", v, x, v & ~x);   OP_SAME("csrrc", v, x, v & ~x);
    OP_RD0("csrrc", v, x, v & ~x);  OP_RS0("csrrc", v, v);
    OP_I("csrrwi", v, 21, 21);      OP_I_RD0("csrrwi", v, 21, 21);  OP_I("csrrwi", v, 0, 0);
    OP_I("csrrsi", v, 18, v | 18);  OP_I_RD0("csrrsi", v, 18, v | 18);  OP_I("csrrsi", v, 0, v);
    OP_I("csrrci", v, 13, v & ~13ULL); OP_I_RD0("csrrci", v, 13, v & ~13ULL); OP_I("csrrci", v, 0, v);
    O[n] = 1;
    return 0;
}
