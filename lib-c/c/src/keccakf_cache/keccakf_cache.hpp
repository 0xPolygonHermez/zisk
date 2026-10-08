#ifndef KECCAKF_CACHE_HPP
#define KECCAKF_CACHE_HPP

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Cache of Keccak-f[1600] input states, mapping a state to the index it was registered under.
// It backs the fcall_set_keccakf_cache_index() / fcall_get_keccakf_cache_index() free calls: a
// program can ask whether a permutation it is about to run has already been executed and, if so,
// reuse the index instead of running it again.
//
// The map is an open-addressed table keyed by a cheap 64-bit fingerprint of the 25-word state;
// the full 25 words are only compared when a fingerprint matches, so a lookup costs one hash
// plus (almost always) a single probe.
//
// This is the assembly emulator's copy of the cache; the Rust emulator keeps an equivalent one
// per instruction context (see ziskos zisklib::keccakf_cache).

// Number of 64-bit words of a Keccak-f[1600] state
#define KECCAKF_STATE_WORDS 25

// Returned by keccakf_cache_get_index() when the state has not been cached. Reserved: it cannot
// be used as a cache index
#define KECCAKF_CACHE_INDEX_NOT_FOUND 0xFFFFFFFFFFFFFFFFULL

// Index the next executed Keccak-f must be cached under, or KECCAKF_CACHE_INDEX_NOT_FOUND if
// none was requested. Exposed so the Keccak-f entry point can skip the call in the common case
extern uint64_t keccakf_cache_pending_index;

// Caches state (the 25 words as read before the permutation) under index. If the state is
// already cached, its index is updated
void keccakf_cache_store (const uint64_t * state, uint64_t index);

// Returns the index state was cached under, or KECCAKF_CACHE_INDEX_NOT_FOUND
uint64_t keccakf_cache_get_index (const uint64_t * state);

// Empties the cache, keeping the allocated memory for the next execution
void keccakf_cache_reset (void);

// Empties the cache and releases its memory
void keccakf_cache_free (void);

// Caches state under the pending index, if fcall_set_keccakf_cache_index() requested one.
// Must be called with the state before the permutation
static inline void keccakf_cache_on_keccakf (const uint64_t * state)
{
    if (keccakf_cache_pending_index != KECCAKF_CACHE_INDEX_NOT_FOUND)
    {
        keccakf_cache_store(state, keccakf_cache_pending_index);
        keccakf_cache_pending_index = KECCAKF_CACHE_INDEX_NOT_FOUND;
    }
}

#ifdef __cplusplus
} // extern "C"
#endif

#endif
