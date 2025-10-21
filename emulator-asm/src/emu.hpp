#ifndef EMU_ASM_HPP
#define EMU_ASM_HPP

#include <stdint.h>

#ifdef DEBUG
extern bool emu_verbose;
#endif

//#define ASM_PRECOMPILE_CACHE

#ifdef ASM_PRECOMPILE_CACHE
void precompile_cache_store_init(void);
void precompile_cache_load_init(void);
void precompile_cache_cleanup(void);
#endif

//#define ASM_CALL_METRICS

#ifdef ASM_CALL_METRICS

typedef struct {

    uint64_t keccak_counter;
    uint64_t keccak_duration;

    uint64_t sha256_counter;
    uint64_t sha256_duration;

    uint64_t arith256_counter;
    uint64_t arith256_duration;

    uint64_t arith256_mod_counter;
    uint64_t arith256_mod_duration;

    uint64_t arith384_mod_counter;
    uint64_t arith384_mod_duration;

    uint64_t secp256k1_add_counter;
    uint64_t secp256k1_add_duration;

    uint64_t secp256k1_dbl_counter;
    uint64_t secp256k1_dbl_duration;

    uint64_t fcall_counter;
    uint64_t fcall_duration;

    uint64_t inverse_fp_ec_counter;
    uint64_t inverse_fp_ec_duration;

    uint64_t inverse_fn_ec_counter;
    uint64_t inverse_fn_ec_duration;

    uint64_t sqrt_fp_ec_parity_counter;
    uint64_t sqrt_fp_ec_parity_duration;

    uint64_t bn254_curve_add_counter;
    uint64_t bn254_curve_add_duration;

    uint64_t bn254_curve_dbl_counter;
    uint64_t bn254_curve_dbl_duration;

    uint64_t bn254_complex_add_counter;
    uint64_t bn254_complex_add_duration;

    uint64_t bn254_complex_sub_counter;
    uint64_t bn254_complex_sub_duration;

    uint64_t bn254_complex_mul_counter;
    uint64_t bn254_complex_mul_duration;

    uint64_t bls12_381_curve_add_counter;
    uint64_t bls12_381_curve_add_duration;

    uint64_t bls12_381_curve_dbl_counter;
    uint64_t bls12_381_curve_dbl_duration;

    uint64_t bls12_381_complex_add_counter;
    uint64_t bls12_381_complex_add_duration;

    uint64_t bls12_381_complex_sub_counter;
    uint64_t bls12_381_complex_sub_duration;

    uint64_t bls12_381_complex_mul_counter;
    uint64_t bls12_381_complex_mul_duration;

    uint64_t add256_counter;    
    uint64_t add256_duration;

} AsmCallMetrics;

extern AsmCallMetrics asm_call_metrics;

void reset_asm_call_metrics (void);
void print_asm_call_metrics (uint64_t total_duration);

#endif // ASM_CALL_METRICS

#endif // EMU_ASM_HPP