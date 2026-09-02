/*
 * zkvm_accelerators.h — Ethereum Foundation zkVM cryptographic-accelerator C
 * interface. Mirrors the standard at:
 *   github.com/eth-act/zkevm-standards/standards/c-interface-accelerators/zkvm_accelerators.h
 *
 * This is the STANDARD, vendor-neutral surface. The ZisK implementation lives in
 * src/zkvm_accelerators.c, which marshals the EF byte-array ABI to the ziskasm
 * flat bindings (zisklib.h -> ziskos_* stubs, redirected to the hand-written
 * .zisk routines by elf2rom). A guest that follows the EF standard links this
 * header + that .c and transparently runs the ziskasm crypto.
 */
#ifndef ZKVM_ACCELERATORS_H
#define ZKVM_ACCELERATORS_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#if defined(__cplusplus) && __cplusplus >= 201103L
  #define ALIGN8 alignas(8)
#elif defined(__STDC_VERSION__) && __STDC_VERSION__ >= 201112L
  #define ALIGN8 _Alignas(8)
#else
  #define ALIGN8
#endif

#ifdef __cplusplus
extern "C" {
#endif

/* ---- status ------------------------------------------------------------ */
typedef enum {
    ZKVM_EOK   = 0,
    ZKVM_EFAIL = -1
} zkvm_status;

/* ---- fixed-size byte arrays (8-byte aligned) --------------------------- */
typedef struct { ALIGN8 uint8_t data[16];  } zkvm_bytes_16;
typedef struct { ALIGN8 uint8_t data[32];  } zkvm_bytes_32;
typedef struct { ALIGN8 uint8_t data[48];  } zkvm_bytes_48;
typedef struct { ALIGN8 uint8_t data[64];  } zkvm_bytes_64;
typedef struct { ALIGN8 uint8_t data[96];  } zkvm_bytes_96;
typedef struct { ALIGN8 uint8_t data[128]; } zkvm_bytes_128;
typedef struct { ALIGN8 uint8_t data[192]; } zkvm_bytes_192;

/* ---- hashes ------------------------------------------------------------ */
typedef zkvm_bytes_32 zkvm_keccak256_hash;
typedef zkvm_bytes_32 zkvm_sha256_hash;
typedef zkvm_bytes_32 zkvm_ripemd160_hash;

/* ---- secp256k1 --------------------------------------------------------- */
typedef zkvm_bytes_32 zkvm_secp256k1_hash;
typedef zkvm_bytes_64 zkvm_secp256k1_signature;   /* r(32) || s(32), big-endian */
typedef zkvm_bytes_64 zkvm_secp256k1_pubkey;      /* x(32) || y(32), big-endian */

/* ---- secp256r1 (P-256) ------------------------------------------------- */
typedef zkvm_bytes_32 zkvm_secp256r1_hash;
typedef zkvm_bytes_64 zkvm_secp256r1_signature;
typedef zkvm_bytes_64 zkvm_secp256r1_pubkey;

/* ---- BN254 (alt_bn128) ------------------------------------------------- */
typedef zkvm_bytes_64  zkvm_bn254_g1_point;
typedef zkvm_bytes_128 zkvm_bn254_g2_point;
typedef zkvm_bytes_32  zkvm_bn254_scalar;
typedef struct { zkvm_bn254_g1_point g1; zkvm_bn254_g2_point g2; } zkvm_bn254_pairing_pair;

/* ---- BLS12-381 --------------------------------------------------------- */
typedef zkvm_bytes_96  zkvm_bls12_381_g1_point;
typedef zkvm_bytes_192 zkvm_bls12_381_g2_point;
typedef zkvm_bytes_32  zkvm_bls12_381_scalar;
typedef zkvm_bytes_48  zkvm_bls12_381_fp;
typedef zkvm_bytes_96  zkvm_bls12_381_fp2;
typedef struct { zkvm_bls12_381_g1_point point; zkvm_bls12_381_scalar scalar; } zkvm_bls12_381_g1_msm_pair;
typedef struct { zkvm_bls12_381_g2_point point; zkvm_bls12_381_scalar scalar; } zkvm_bls12_381_g2_msm_pair;
typedef struct { zkvm_bls12_381_g1_point g1; zkvm_bls12_381_g2_point g2; } zkvm_bls12_381_pairing_pair;

/* ---- BLAKE2f ----------------------------------------------------------- */
typedef zkvm_bytes_64  zkvm_blake2f_state;
typedef zkvm_bytes_128 zkvm_blake2f_message;
typedef zkvm_bytes_16  zkvm_blake2f_offset;

/* ---- KZG (EIP-4844) ---------------------------------------------------- */
typedef zkvm_bytes_48 zkvm_kzg_commitment;
typedef zkvm_bytes_48 zkvm_kzg_proof;
typedef zkvm_bytes_32 zkvm_kzg_field_element;

/* ---- functions --------------------------------------------------------- */
zkvm_status zkvm_keccak256(const uint8_t* data, size_t len, zkvm_keccak256_hash* output);
zkvm_status zkvm_sha256(const uint8_t* data, size_t len, zkvm_sha256_hash* output);
zkvm_status zkvm_ripemd160(const uint8_t* data, size_t len, zkvm_ripemd160_hash* output);

zkvm_status zkvm_secp256k1_verify(const zkvm_secp256k1_hash* msg,
                                  const zkvm_secp256k1_signature* sig,
                                  const zkvm_secp256k1_pubkey* pubkey, bool* verified);
zkvm_status zkvm_secp256k1_ecrecover(const zkvm_secp256k1_hash* msg,
                                     const zkvm_secp256k1_signature* sig,
                                     uint8_t recid, zkvm_secp256k1_pubkey* output);
zkvm_status zkvm_secp256r1_verify(const zkvm_secp256r1_hash* msg,
                                  const zkvm_secp256r1_signature* sig,
                                  const zkvm_secp256r1_pubkey* pubkey, bool* verified);

zkvm_status zkvm_modexp(const uint8_t* base, size_t base_len,
                        const uint8_t* exp, size_t exp_len,
                        const uint8_t* modulus, size_t mod_len, uint8_t* output);

zkvm_status zkvm_bn254_g1_add(const zkvm_bn254_g1_point* p1, const zkvm_bn254_g1_point* p2,
                              zkvm_bn254_g1_point* result);
zkvm_status zkvm_bn254_g1_mul(const zkvm_bn254_g1_point* point, const zkvm_bn254_scalar* scalar,
                              zkvm_bn254_g1_point* result);
zkvm_status zkvm_bn254_pairing(const zkvm_bn254_pairing_pair* pairs, size_t num_pairs, bool* verified);

zkvm_status zkvm_blake2f(uint32_t rounds, zkvm_blake2f_state* h,
                         const zkvm_blake2f_message* m, const zkvm_blake2f_offset* t, uint8_t f);

zkvm_status zkvm_kzg_point_eval(const zkvm_kzg_commitment* commitment,
                                const zkvm_kzg_field_element* z, const zkvm_kzg_field_element* y,
                                const zkvm_kzg_proof* proof, bool* verified);

zkvm_status zkvm_bls12_g1_add(const zkvm_bls12_381_g1_point* p1, const zkvm_bls12_381_g1_point* p2,
                              zkvm_bls12_381_g1_point* result);
zkvm_status zkvm_bls12_g1_msm(const zkvm_bls12_381_g1_msm_pair* pairs, size_t num_pairs,
                              zkvm_bls12_381_g1_point* result);
zkvm_status zkvm_bls12_g2_add(const zkvm_bls12_381_g2_point* p1, const zkvm_bls12_381_g2_point* p2,
                              zkvm_bls12_381_g2_point* result);
zkvm_status zkvm_bls12_g2_msm(const zkvm_bls12_381_g2_msm_pair* pairs, size_t num_pairs,
                              zkvm_bls12_381_g2_point* result);
zkvm_status zkvm_bls12_pairing(const zkvm_bls12_381_pairing_pair* pairs, size_t num_pairs, bool* verified);
zkvm_status zkvm_bls12_map_fp_to_g1(const zkvm_bls12_381_fp* field_element,
                                    zkvm_bls12_381_g1_point* result);
zkvm_status zkvm_bls12_map_fp2_to_g2(const zkvm_bls12_381_fp2* field_element,
                                     zkvm_bls12_381_g2_point* result);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* ZKVM_ACCELERATORS_H */
