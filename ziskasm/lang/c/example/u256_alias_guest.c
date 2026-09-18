/*
 * u256_alias_guest.c - aliasing conformance test for the EF U256 ABI.
 *
 * zkvm_u256.h promises "The result pointer MAY alias any input pointer". Every
 * ziskasm_zkvm_u256_* routine in zkvm/u256.zisk must therefore finish reading its
 * operands before it writes any result word -- a routine that cleared or filled the
 * result first would silently corrupt an in-place call such as add(&x, &y, &x),
 * which is exactly how an EVM interpreter uses these: the operands are popped and
 * the result pushed over the same stack slot.
 *
 * For each op: compute into a distinct buffer, then recompute with the result
 * aliased onto each input in turn, and require byte equality. Unlike the other
 * guests here, this one is SELF-CHECKING and needs no golden vector -- it emits one
 * byte per case, 00 = pass, 01 = MISMATCH, so the expected output is all zeros
 * followed by the 01 00 negative control (see the end of main).
 *
 * Build/run: see README.md ("U256 aliasing conformance").
 */
#include "zkvm_u256.h"

static void cp(zkvm_u256 *d, const zkvm_u256 *s){ for(int i=0;i<32;i++) d->data[i]=s->data[i]; }
static int ne(const zkvm_u256 *x, const zkvm_u256 *y){ for(int i=0;i<32;i++) if(x->data[i]!=y->data[i]) return 1; return 0; }

/* A: large odd-ish value; B: smaller non-zero; N: modulus */
static const zkvm_u256 A = {{0x9f,0x8f,0x5f,0xfb,0xa5,0xf8,0x0a,0x0a,0x58,0x99,0x49,0x53,0x04,0x0e,0x1e,0x30,
                             0xc9,0xed,0x02,0x48,0xfc,0x97,0x99,0xa7,0x07,0xe3,0x6d,0x60,0x04,0x76,0x2a,0x23}};
static const zkvm_u256 B = {{0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
                             0x00,0x00,0x00,0x15,0xb1,0x6e,0x2d,0x5c,0xab,0xeb,0x95,0x92,0x08,0xf0,0xeb,0xd5}};
static const zkvm_u256 N = {{0x7f,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,
                             0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xed}};

static zkvm_u256 R, R2, T, T2;
static volatile uint8_t *O;
static unsigned k;
static void rec(int bad){ O[k++] = bad ? 0x01 : 0x00; }

typedef zkvm_status (*bin_t)(const zkvm_u256*, const zkvm_u256*, zkvm_u256*);
typedef zkvm_status (*un_t)(const zkvm_u256*, zkvm_u256*);
typedef zkvm_status (*ter_t)(const zkvm_u256*, const zkvm_u256*, const zkvm_u256*, zkvm_u256*);

/* binary: check result==a and result==b */
static void chk_bin(bin_t f, const zkvm_u256 *x, const zkvm_u256 *y){
  f(x,y,&R);
  cp(&T,x); f(&T,y,&T); rec(ne(&T,&R));
  cp(&T,y); f(x,&T,&T); rec(ne(&T,&R));
}
static void chk_un(un_t f, const zkvm_u256 *x){
  f(x,&R);
  cp(&T,x); f(&T,&T); rec(ne(&T,&R));
}
static void chk_ter(ter_t f, const zkvm_u256 *x, const zkvm_u256 *y, const zkvm_u256 *z){
  f(x,y,z,&R);
  cp(&T,x); f(&T,y,z,&T); rec(ne(&T,&R));
  cp(&T,y); f(x,&T,z,&T); rec(ne(&T,&R));
  cp(&T,z); f(x,y,&T,&T); rec(ne(&T,&R));
}

static uint8_t g_bss[32];
static volatile uint32_t g_data = 0x600df00d;
int main(void){
  __asm__ volatile("" : : "r"(&g_bss[0]), "r"(&g_data) : "memory");
  O = (volatile uint8_t *)(0xA0410000ULL); k = 0;

  chk_bin(zkvm_u256_add,&A,&B);   chk_bin(zkvm_u256_sub,&A,&B);
  chk_bin(zkvm_u256_mul,&A,&B);   chk_bin(zkvm_u256_div,&A,&B);
  chk_bin(zkvm_u256_mod,&A,&B);   chk_bin(zkvm_u256_sdiv,&A,&B);
  chk_bin(zkvm_u256_smod,&A,&B);  chk_bin(zkvm_u256_exp,&A,&B);
  chk_bin(zkvm_u256_lt,&A,&B);    chk_bin(zkvm_u256_gt,&A,&B);
  chk_bin(zkvm_u256_slt,&A,&B);   chk_bin(zkvm_u256_sgt,&A,&B);
  chk_bin(zkvm_u256_eq,&A,&B);    chk_bin(zkvm_u256_eq,&A,&A);
  chk_bin(zkvm_u256_and,&A,&B);   chk_bin(zkvm_u256_or,&A,&B);
  chk_bin(zkvm_u256_xor,&A,&B);   chk_bin(zkvm_u256_byte,&B,&A);
  chk_bin(zkvm_u256_shl,&B,&A);   chk_bin(zkvm_u256_shr,&B,&A);
  chk_bin(zkvm_u256_sar,&B,&A);   chk_bin(zkvm_u256_signextend,&B,&A);
  chk_un(zkvm_u256_not,&A);       chk_un(zkvm_u256_iszero,&A);
  chk_ter(zkvm_u256_addmod,&A,&B,&N);
  chk_ter(zkvm_u256_mulmod,&A,&B,&N);

  /* divmod / sdivmod: two outputs, alias each onto each input */
  zkvm_u256_divmod(&A,&B,&R,&R2);
  cp(&T,&A); zkvm_u256_divmod(&T,&B,&T,&T2); rec(ne(&T,&R)||ne(&T2,&R2));
  cp(&T,&B); zkvm_u256_divmod(&A,&T,&T2,&T); rec(ne(&T2,&R)||ne(&T,&R2));
  zkvm_u256_sdivmod(&A,&B,&R,&R2);
  cp(&T,&A); zkvm_u256_sdivmod(&T,&B,&T,&T2); rec(ne(&T,&R)||ne(&T2,&R2));
  cp(&T,&B); zkvm_u256_sdivmod(&A,&T,&T2,&T); rec(ne(&T2,&R)||ne(&T,&R2));
  /* Negative control: prove the comparison machinery actually fires.
     Expect 01 then 00 as the final two bytes. */
  rec(ne(&A,&B));
  rec(ne(&A,&A));
  return 0;
}
