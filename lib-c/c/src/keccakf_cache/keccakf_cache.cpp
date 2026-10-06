#include "keccakf_cache.hpp"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Index the next executed Keccak-f must be cached under
uint64_t keccakf_cache_pending_index = KECCAKF_CACHE_INDEX_NOT_FOUND;

// The cache is an open-addressed, linearly probed hash table split in three arrays, so that the
// common operations touch as little memory as possible:
//   - tags: one byte per slot, 0 when empty, otherwise 7 bits of the state fingerprint (with the
//     top bit set). Most lookups are misses, and a miss usually reads only this array, which is
//     small enough to stay in the CPU caches
//   - slot_entries: per slot, the number of the entry it holds; only read on a tag match
//   - entries: the cached states, in insertion order, with their fingerprint and index; a hit
//     compares the whole state before returning, so results are exact

// A cached state
struct KeccakfCacheEntry
{
    uint64_t hash;  // fingerprint of the state, kept to rehash when the table grows
    uint64_t index; // index the state was cached under
    uint64_t state[KECCAKF_STATE_WORDS];
};

// Number of slots of a freshly allocated table. Power of two: the probe index is masked
#define KECCAKF_CACHE_INITIAL_SLOTS (1 << 18)

// Slot table: slots_size tags and entry numbers
static uint8_t * tags = NULL;
static uint32_t * slot_entries = NULL;
static uint64_t slots_size = 0; // number of slots, 0 or a power of two

// Cached states
static struct KeccakfCacheEntry * entries = NULL;
static uint64_t entries_size = 0; // capacity
static uint64_t entries_used = 0; // number of cached states

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

// Fingerprints a 25-word state into a 64-bit value. The state is folded through four
// independent lanes so the multiplies pipeline instead of forming one 25-long dependency chain.
// The low bits select the first probed slot, and the top 7 bits make the tag
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

    return mix(l0 ^ l1, l2 ^ l3) ^
           mix(state[KECCAKF_STATE_WORDS - 1] ^ MIX_KEYS[0], MIX_KEYS[4]);
}

// Tag of a fingerprint: its top 7 bits, with the top bit set so that it is never 0 (empty)
static inline uint8_t keccakf_cache_tag (uint64_t hash)
{
    return (uint8_t)((hash >> 57) | 0x80);
}

// Returns the slot holding state, or the empty slot where it would be inserted
static inline uint64_t keccakf_cache_find (const uint64_t * state, uint64_t hash, bool * found)
{
    uint64_t mask = slots_size - 1;
    uint8_t tag = keccakf_cache_tag(hash);
    uint64_t s = hash & mask;
    for (;;)
    {
        uint8_t t = tags[s];
        if (t == 0)
        {
            *found = false;
            return s;
        }
        if (t == tag)
        {
            const struct KeccakfCacheEntry * e = &entries[slot_entries[s]];
            if ((e->hash == hash) &&
                (memcmp(e->state, state, KECCAKF_STATE_WORDS * sizeof(uint64_t)) == 0))
            {
                *found = true;
                return s;
            }
        }
        s = (s + 1) & mask;
    }
}

// Doubles the slot table (allocating it on first use) and reinserts every entry
static void keccakf_cache_grow_slots (void)
{
    uint64_t new_size = (slots_size == 0) ? KECCAKF_CACHE_INITIAL_SLOTS : slots_size * 2;
    uint8_t * new_tags = (uint8_t *)calloc(new_size, sizeof(uint8_t));
    uint32_t * new_slot_entries = (uint32_t *)malloc(new_size * sizeof(uint32_t));
    if ((new_tags == NULL) || (new_slot_entries == NULL))
    {
        printf("keccakf_cache_grow_slots() failed allocating %lu slots\n", (unsigned long)new_size);
        exit(-1);
    }

    free(tags);
    free(slot_entries);
    tags = new_tags;
    slot_entries = new_slot_entries;
    slots_size = new_size;

    // Entries are unique, so they are reinserted without comparing states
    uint64_t mask = new_size - 1;
    for (uint64_t i = 0; i < entries_used; i++)
    {
        uint64_t s = entries[i].hash & mask;
        while (tags[s] != 0) s = (s + 1) & mask;
        tags[s] = keccakf_cache_tag(entries[i].hash);
        slot_entries[s] = (uint32_t)i;
    }
}

// Makes room in the entries array for one more state
static void keccakf_cache_reserve_entry (void)
{
    if (entries_used < entries_size) return;

    // The table grows when it is half full, so it never holds more than slots_size/2 states
    uint64_t new_size = (entries_size == 0) ? (KECCAKF_CACHE_INITIAL_SLOTS / 2) : entries_size * 2;
    if (new_size > 0xFFFFFFFFULL)
    {
        printf("keccakf_cache_reserve_entry() cannot hold more than 2^32 states\n");
        exit(-1);
    }
    struct KeccakfCacheEntry * new_entries =
        (struct KeccakfCacheEntry *)realloc(entries, new_size * sizeof(struct KeccakfCacheEntry));
    if (new_entries == NULL)
    {
        printf("keccakf_cache_reserve_entry() failed calling realloc() for %lu entries\n",
            (unsigned long)new_size);
        exit(-1);
    }

    entries = new_entries;
    entries_size = new_size;
}

void keccakf_cache_store (const uint64_t * state, uint64_t index)
{
    // Keep the load factor at or below 1/2 so probe runs stay short
    if ((entries_used + 1) * 2 > slots_size) keccakf_cache_grow_slots();

    uint64_t hash = keccakf_cache_fingerprint(state);
    bool found;
    uint64_t s = keccakf_cache_find(state, hash, &found);
    if (found)
    {
        // Already cached: keep the most recent index
        entries[slot_entries[s]].index = index;
        return;
    }

    keccakf_cache_reserve_entry();
    struct KeccakfCacheEntry * e = &entries[entries_used];
    e->hash = hash;
    e->index = index;
    memcpy(e->state, state, KECCAKF_STATE_WORDS * sizeof(uint64_t));

    tags[s] = keccakf_cache_tag(hash);
    slot_entries[s] = (uint32_t)entries_used;
    entries_used++;
}

uint64_t keccakf_cache_get_index (const uint64_t * state)
{
    if (slots_size == 0) return KECCAKF_CACHE_INDEX_NOT_FOUND;

    uint64_t hash = keccakf_cache_fingerprint(state);
    bool found;
    uint64_t s = keccakf_cache_find(state, hash, &found);
    return found ? entries[slot_entries[s]].index : KECCAKF_CACHE_INDEX_NOT_FOUND;
}

void keccakf_cache_reset (void)
{
    if (tags != NULL) memset(tags, 0, slots_size * sizeof(uint8_t));
    entries_used = 0;
    keccakf_cache_pending_index = KECCAKF_CACHE_INDEX_NOT_FOUND;
}

void keccakf_cache_free (void)
{
    free(tags);
    tags = NULL;
    free(slot_entries);
    slot_entries = NULL;
    slots_size = 0;

    free(entries);
    entries = NULL;
    entries_size = 0;
    entries_used = 0;

    keccakf_cache_pending_index = KECCAKF_CACHE_INDEX_NOT_FOUND;
}
