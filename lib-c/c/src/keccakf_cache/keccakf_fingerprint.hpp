#ifndef KECCAKF_FINGERPRINT_HPP
#define KECCAKF_FINGERPRINT_HPP

#include <stdint.h>

// Cheap, non-cryptographic 64-bit fingerprint of a Keccak-f[1600] state, shared by the hash
// tables keyed by a state (keccakf_cache, keccakf_dedup). A fingerprint only selects the slots
// to probe: the tables compare the whole state before reporting a match.

#ifndef KECCAKF_STATE_WORDS
// Number of 64-bit words of a Keccak-f[1600] state
#define KECCAKF_STATE_WORDS 25
#endif

// Multipliers for keccakf_fingerprint_mix(); odd 64-bit constants taken from wyhash
static const uint64_t KECCAKF_FINGERPRINT_KEYS[5] =
{
    0xa0761d6478bd642fULL,
    0xe7037ed1a0b428dbULL,
    0x8ebc6af09c88c6e3ULL,
    0x589965cc75374cc3ULL,
    0x1d8e4e27c47d124fULL
};

// Folds a 128-bit product back into 64 bits
static inline uint64_t keccakf_fingerprint_mix (uint64_t a, uint64_t b)
{
    __uint128_t r = (__uint128_t)a * (__uint128_t)b;
    return (uint64_t)r ^ (uint64_t)(r >> 64);
}

// Fingerprints a 25-word state into a 64-bit value. The state is folded through four
// independent lanes so the multiplies pipeline instead of forming one 25-long dependency chain.
// The low bits select the first probed slot, and the top 7 bits make the tag
static inline uint64_t keccakf_fingerprint (const uint64_t * state)
{
    uint64_t l0 = KECCAKF_FINGERPRINT_KEYS[0];
    uint64_t l1 = KECCAKF_FINGERPRINT_KEYS[1];
    uint64_t l2 = KECCAKF_FINGERPRINT_KEYS[2];
    uint64_t l3 = KECCAKF_FINGERPRINT_KEYS[3];

    for (uint64_t i = 0; i < (uint64_t)(KECCAKF_STATE_WORDS - 1); i += 4)
    {
        l0 = keccakf_fingerprint_mix(l0 ^ state[i],     KECCAKF_FINGERPRINT_KEYS[1]);
        l1 = keccakf_fingerprint_mix(l1 ^ state[i + 1], KECCAKF_FINGERPRINT_KEYS[2]);
        l2 = keccakf_fingerprint_mix(l2 ^ state[i + 2], KECCAKF_FINGERPRINT_KEYS[3]);
        l3 = keccakf_fingerprint_mix(l3 ^ state[i + 3], KECCAKF_FINGERPRINT_KEYS[4]);
    }

    return keccakf_fingerprint_mix(l0 ^ l1, l2 ^ l3) ^
           keccakf_fingerprint_mix(state[KECCAKF_STATE_WORDS - 1] ^ KECCAKF_FINGERPRINT_KEYS[0], KECCAKF_FINGERPRINT_KEYS[4]);
}

// Tag of a fingerprint: its top 7 bits, with the top bit set so that it is never 0 (empty)
static inline uint8_t keccakf_fingerprint_tag (uint64_t hash)
{
    return (uint8_t)((hash >> 57) | 0x80);
}

#endif
