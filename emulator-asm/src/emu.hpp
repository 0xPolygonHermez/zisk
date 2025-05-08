#ifndef EMU_ASM_HPP
#define EMU_ASM_HPP

#ifdef DEBUG
extern bool keccak_metrics;
extern uint64_t keccak_counter;
extern uint64_t keccak_duration;

extern bool arith256_metrics;
extern bool arith256_mod_metrics;
extern bool secp256k1_add_metrics;
extern bool secp256k1_dbl_metrics;
#endif

#endif