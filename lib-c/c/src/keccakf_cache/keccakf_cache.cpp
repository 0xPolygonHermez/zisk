#include "keccakf_cache.hpp"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Index the next executed Keccak-f must be cached under
uint64_t keccakf_cache_pending_index = KECCAKF_CACHE_INDEX_NOT_FOUND;

// One open-addressed slot; hash == 0 marks it empty
struct KeccakfCacheSlot
{
    uint64_t hash;   // fingerprint of the cached state, never 0 when the slot is occupied
    uint64_t index;  // index the state was cached under
    uint64_t offset; // offset of the state in the states array, in words
};

// Number of slots of a freshly allocated table. Power of two: the probe index is masked
#define KECCAKF_CACHE_INITIAL_SLOTS (1 << 18)

// Slot table, linearly probed
static struct KeccakfCacheSlot * slots = NULL;
static uint64_t slots_size = 0; // number of slots, 0 or a power of two
static uint64_t slots_used = 0; // number of occupied slots

// Cached states, KECCAKF_STATE_WORDS words each, addressed by KeccakfCacheSlot::offset
static uint64_t * states = NULL;
static uint64_t states_size = 0; // capacity, in words
static uint64_t states_used = 0; // used, in words

// Multipliers for mix(); odd 64-bit constants taken from wyhash
static const uint64_t MIX_KEYS[5] =
{
    0xa0761d6478bd642fULL,
    0xe7037ed1a0b428dbULL,
    0x8ebc6af09c88c6e3ULL,
    0x589965cc75374cc3ULL,
    0x1d8e4e27c47d124fULL
};

// Folds a 128-bit product back into 64 bits
static inline uint64_t mix (uint64_t a, uint64_t b)
{
    __uint128_t r = (__uint128_t)a * (__uint128_t)b;
    return (uint64_t)r ^ (uint64_t)(r >> 64);
}

// Fingerprints a 25-word state into a non-zero 64-bit value (0 marks an empty slot).
// The state is folded through four independent lanes so the multiplies pipeline instead of
// forming one 25-long dependency chain: this runs once per lookup and once per cached state
static inline uint64_t keccakf_cache_fingerprint (const uint64_t * state)
{
    uint64_t l0 = MIX_KEYS[0];
    uint64_t l1 = MIX_KEYS[1];
    uint64_t l2 = MIX_KEYS[2];
    uint64_t l3 = MIX_KEYS[3];

    for (uint64_t i = 0; i < (uint64_t)(KECCAKF_STATE_WORDS - 1); i += 4)
    {
        l0 = mix(l0 ^ state[i],     MIX_KEYS[1]);
        l1 = mix(l1 ^ state[i + 1], MIX_KEYS[2]);
        l2 = mix(l2 ^ state[i + 2], MIX_KEYS[3]);
        l3 = mix(l3 ^ state[i + 3], MIX_KEYS[4]);
    }

    uint64_t hash = mix(l0 ^ l1, l2 ^ l3) ^
                    mix(state[KECCAKF_STATE_WORDS - 1] ^ MIX_KEYS[0], MIX_KEYS[4]);

    return (hash == 0) ? 1 : hash;
}

// Doubles the slot table (allocating it on first use) and reinserts every entry
static void keccakf_cache_grow_slots (void)
{
    uint64_t new_size = (slots_size == 0) ? KECCAKF_CACHE_INITIAL_SLOTS : slots_size * 2;
    struct KeccakfCacheSlot * new_slots =
        (struct KeccakfCacheSlot *)calloc(new_size, sizeof(struct KeccakfCacheSlot));
    if (new_slots == NULL)
    {
        printf("keccakf_cache_grow_slots() failed calling calloc() for %lu slots\n",
            (unsigned long)new_size);
        exit(-1);
    }

    uint64_t mask = new_size - 1;
    for (uint64_t i = 0; i < slots_size; i++)
    {
        if (slots[i].hash == 0) continue;
        uint64_t s = slots[i].hash & mask;
        while (new_slots[s].hash != 0) s = (s + 1) & mask;
        new_slots[s] = slots[i];
    }

    free(slots);
    slots = new_slots;
    slots_size = new_size;
}

// Makes room in the states array for one more state
static void keccakf_cache_reserve_state (void)
{
    if (states_used + KECCAKF_STATE_WORDS <= states_size) return;

    // The table grows when it is half full, so it never holds more than slots_size/2 states
    uint64_t new_size = (states_size == 0)
        ? (KECCAKF_CACHE_INITIAL_SLOTS / 2) * KECCAKF_STATE_WORDS
        : states_size * 2;
    uint64_t * new_states = (uint64_t *)realloc(states, new_size * sizeof(uint64_t));
    if (new_states == NULL)
    {
        printf("keccakf_cache_reserve_state() failed calling realloc() for %lu words\n",
            (unsigned long)new_size);
        exit(-1);
    }

    states = new_states;
    states_size = new_size;
}

void keccakf_cache_store (const uint64_t * state, uint64_t index)
{
    // Keep the load factor at or below 1/2 so probe runs stay short
    if ((slots_used + 1) * 2 > slots_size) keccakf_cache_grow_slots();

    uint64_t hash = keccakf_cache_fingerprint(state);
    uint64_t mask = slots_size - 1;
    uint64_t s = hash & mask;
    while (slots[s].hash != 0)
    {
        if ((slots[s].hash == hash) &&
            (memcmp(&states[slots[s].offset], state, KECCAKF_STATE_WORDS * sizeof(uint64_t)) == 0))
        {
            // Already cached: keep the most recent index
            slots[s].index = index;
            return;
        }
        s = (s + 1) & mask;
    }

    keccakf_cache_reserve_state();
    memcpy(&states[states_used], state, KECCAKF_STATE_WORDS * sizeof(uint64_t));

    slots[s].hash = hash;
    slots[s].index = index;
    slots[s].offset = states_used;

    states_used += KECCAKF_STATE_WORDS;
    slots_used++;
}

uint64_t keccakf_cache_get_index (const uint64_t * state)
{
    if (slots_size == 0) return KECCAKF_CACHE_INDEX_NOT_FOUND;

    uint64_t hash = keccakf_cache_fingerprint(state);
    uint64_t mask = slots_size - 1;
    uint64_t s = hash & mask;
    while (slots[s].hash != 0)
    {
        if ((slots[s].hash == hash) &&
            (memcmp(&states[slots[s].offset], state, KECCAKF_STATE_WORDS * sizeof(uint64_t)) == 0))
        {
            return slots[s].index;
        }
        s = (s + 1) & mask;
    }

    return KECCAKF_CACHE_INDEX_NOT_FOUND;
}

void keccakf_cache_reset (void)
{
    if (slots != NULL) memset(slots, 0, slots_size * sizeof(struct KeccakfCacheSlot));
    slots_used = 0;
    states_used = 0;
    keccakf_cache_pending_index = KECCAKF_CACHE_INDEX_NOT_FOUND;
}

void keccakf_cache_free (void)
{
    free(slots);
    slots = NULL;
    slots_size = 0;
    slots_used = 0;

    free(states);
    states = NULL;
    states_size = 0;
    states_used = 0;

    keccakf_cache_pending_index = KECCAKF_CACHE_INDEX_NOT_FOUND;
}
