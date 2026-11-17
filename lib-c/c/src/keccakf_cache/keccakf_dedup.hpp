#ifndef KECCAKF_DEDUP_HPP
#define KECCAKF_DEDUP_HPP

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Table of the distinct Keccak-f[1600] permutations of an execution.
//
// Every executed Keccak-f is looked up by its input state. The first time a state is seen it is
// registered, with its output, as a new UNIQUE permutation; its index is the number of unique
// permutations registered before it, i.e. unique permutations are numbered in order of first
// appearance. Any later Keccak-f with the same input state is a REPLICA of that unique
// permutation: its output is taken from the table instead of being computed, and the
// permutation's replica counter is increased.
//
// The prover proves a unique permutation once (Keccakf air) and every invocation, unique or
// replica, with one cheap row (KeccakfProxy air), so it needs to know, for every Keccak-f, which
// unique permutation it is and whether it is the first one. That is the dedup word below, which
// the minimal trace carries after the 25 input words of every Keccak-f.
//
// Same layout as keccakf_cache: an open-addressed table keyed by a cheap fingerprint, with the
// whole input state compared on a fingerprint match, so results are exact.

// Dedup word of a Keccak-f: index of its unique permutation, plus this flag when the Keccak-f is
// a replica, i.e. not the first one with this input state
#define KECCAKF_DEDUP_REPLICA_FLAG 0x8000000000000000ULL

// Most replicas one unique permutation can have: the KeccakfProxy air commits the count in a
// 24-bit column. A Keccak-f that repeats a permutation with this many replicas is not a replica of
// it: keccakf_dedup_find() does not report that permutation any more, so the Keccak-f is
// registered as a new unique permutation, which takes the replicas that follow
#define KECCAKF_DEDUP_MAX_REPLICAS 0xFFFFFFULL

// Returned by keccakf_dedup_find() when the input state has not been registered
#define KECCAKF_DEDUP_NOT_FOUND 0xFFFFFFFFFFFFFFFFULL

// Looks up the input state of a Keccak-f. Returns the index of its unique permutation, or
// KECCAKF_DEDUP_NOT_FOUND. fingerprint receives the state fingerprint, to be handed to
// keccakf_dedup_add() on a miss so it is not computed twice
uint64_t keccakf_dedup_find (const uint64_t * input, uint64_t * fingerprint);

// Accounts for a replica of the unique permutation index and copies its output state to output
// (not copied if output is NULL). Returns the dedup word of the replica
uint64_t keccakf_dedup_replica (uint64_t index, uint64_t * output);

// Registers a new unique permutation, which keccakf_dedup_find() has just reported as not found.
// Returns its dedup word, i.e. its index
uint64_t keccakf_dedup_add (const uint64_t * input, uint64_t fingerprint, const uint64_t * output);

// Accounts for a Keccak-f whose input and output states are both known. Returns its dedup word
uint64_t keccakf_dedup_register (const uint64_t * input, const uint64_t * output);

// Number of unique permutations registered, and number of replicas of the unique permutation
// index (0 if there are none)
uint64_t keccakf_dedup_unique_count (void);
uint64_t keccakf_dedup_replicas (uint64_t index);

// Total number of replicas, i.e. Keccak-f executions = unique count + replica count
uint64_t keccakf_dedup_replica_count (void);

// Empties the table, keeping the allocated memory for the next execution
void keccakf_dedup_reset (void);

// Empties the table and releases its memory
void keccakf_dedup_free (void);

#ifdef __cplusplus
} // extern "C"
#endif

#endif
