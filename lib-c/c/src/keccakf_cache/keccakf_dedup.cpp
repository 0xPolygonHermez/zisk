#include "keccakf_dedup.hpp"
#include "keccakf_fingerprint.hpp"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Open-addressed, linearly probed hash table split in three arrays, as in keccakf_cache:
//   - tags: one byte per slot, 0 when empty, otherwise 7 bits of the input fingerprint
//   - slot_entries: per slot, the number of the entry it holds; only read on a tag match
//   - entries: the unique permutations, in order of first appearance, so the entry number is
//     the index of the unique permutation

// A unique permutation
struct KeccakfDedupEntry
{
    uint64_t hash;     // fingerprint of the input state, kept to rehash when the table grows
    uint64_t replicas; // number of later Keccak-f executions with the same input state
    uint64_t input[KECCAKF_STATE_WORDS];
    uint64_t output[KECCAKF_STATE_WORDS];
};

#define KECCAKF_STATE_BYTES (KECCAKF_STATE_WORDS * sizeof(uint64_t))

// Number of slots of a freshly allocated table. Power of two: the probe index is masked. It holds
// 2^19 unique permutations before growing, more than a large block needs (about 400K Keccak-f)
#define KECCAKF_DEDUP_INITIAL_SLOTS (1 << 20)

// Slot table: slots_size tags and entry numbers
static uint8_t * tags = NULL;
static uint32_t * slot_entries = NULL;
static uint64_t slots_size = 0; // number of slots, 0 or a power of two

// Unique permutations
static struct KeccakfDedupEntry * entries = NULL;
static uint64_t entries_size = 0; // capacity
static uint64_t entries_used = 0; // number of unique permutations

// Total number of replicas
static uint64_t replica_count = 0;

// Returns the slot holding input, or the empty slot where it would be inserted
static inline uint64_t keccakf_dedup_find_slot (const uint64_t * input, uint64_t hash, bool * found)
{
    uint64_t mask = slots_size - 1;
    uint8_t tag = keccakf_fingerprint_tag(hash);
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
            const struct KeccakfDedupEntry * e = &entries[slot_entries[s]];
            // A permutation that cannot take more replicas does not match: the probe goes on to
            // the one registered after it for the same state, or to the empty slot for a new one
            if ((e->hash == hash) && (e->replicas < KECCAKF_DEDUP_MAX_REPLICAS) &&
                (memcmp(e->input, input, KECCAKF_STATE_BYTES) == 0))
            {
                *found = true;
                return s;
            }
        }
        s = (s + 1) & mask;
    }
}

// Doubles the slot table (allocating it on first use) and reinserts every entry
static void keccakf_dedup_grow_slots (void)
{
    uint64_t new_size = (slots_size == 0) ? KECCAKF_DEDUP_INITIAL_SLOTS : slots_size * 2;
    uint8_t * new_tags = (uint8_t *)calloc(new_size, sizeof(uint8_t));
    uint32_t * new_slot_entries = (uint32_t *)malloc(new_size * sizeof(uint32_t));
    if ((new_tags == NULL) || (new_slot_entries == NULL))
    {
        printf("keccakf_dedup_grow_slots() failed allocating %lu slots\n", (unsigned long)new_size);
        exit(-1);
    }

    free(tags);
    free(slot_entries);
    tags = new_tags;
    slot_entries = new_slot_entries;
    slots_size = new_size;

    // Entries are reinserted without comparing states, in order of registration: of two entries
    // with the same state (see KECCAKF_DEDUP_MAX_REPLICAS) the full one stays first in the probe
    uint64_t mask = new_size - 1;
    for (uint64_t i = 0; i < entries_used; i++)
    {
        uint64_t s = entries[i].hash & mask;
        while (tags[s] != 0) s = (s + 1) & mask;
        tags[s] = keccakf_fingerprint_tag(entries[i].hash);
        slot_entries[s] = (uint32_t)i;
    }
}

// Makes room in the entries array for one more unique permutation
static void keccakf_dedup_reserve_entry (void)
{
    if (entries_used < entries_size) return;

    // The table grows when it is half full, so it never holds more than slots_size/2 entries
    uint64_t new_size = (entries_size == 0) ? (KECCAKF_DEDUP_INITIAL_SLOTS / 2) : entries_size * 2;
    if (new_size > 0xFFFFFFFFULL)
    {
        printf("keccakf_dedup_reserve_entry() cannot hold more than 2^32 permutations\n");
        exit(-1);
    }
    struct KeccakfDedupEntry * new_entries =
        (struct KeccakfDedupEntry *)realloc(entries, new_size * sizeof(struct KeccakfDedupEntry));
    if (new_entries == NULL)
    {
        printf("keccakf_dedup_reserve_entry() failed calling realloc() for %lu entries\n",
            (unsigned long)new_size);
        exit(-1);
    }

    entries = new_entries;
    entries_size = new_size;
}

uint64_t keccakf_dedup_find (const uint64_t * input, uint64_t * fingerprint)
{
    uint64_t hash = keccakf_fingerprint(input);
    *fingerprint = hash;
    if (slots_size == 0) return KECCAKF_DEDUP_NOT_FOUND;

    bool found;
    uint64_t s = keccakf_dedup_find_slot(input, hash, &found);
    return found ? (uint64_t)slot_entries[s] : KECCAKF_DEDUP_NOT_FOUND;
}

uint64_t keccakf_dedup_replica (uint64_t index, uint64_t * output)
{
    struct KeccakfDedupEntry * e = &entries[index];
    e->replicas++;
    replica_count++;
    if (output != NULL) memcpy(output, e->output, KECCAKF_STATE_BYTES);
    return index | KECCAKF_DEDUP_REPLICA_FLAG;
}

uint64_t keccakf_dedup_add (const uint64_t * input, uint64_t fingerprint, const uint64_t * output)
{
    // Keep the load factor at or below 1/2 so probe runs stay short
    if ((entries_used + 1) * 2 > slots_size) keccakf_dedup_grow_slots();

    // The caller has just missed, so the probe ends at the empty slot to fill
    bool found;
    uint64_t s = keccakf_dedup_find_slot(input, fingerprint, &found);
    if (found)
    {
        printf("keccakf_dedup_add() called with an input state that is already registered\n");
        exit(-1);
    }

    keccakf_dedup_reserve_entry();
    uint64_t index = entries_used;
    struct KeccakfDedupEntry * e = &entries[index];
    e->hash = fingerprint;
    e->replicas = 0;
    memcpy(e->input, input, KECCAKF_STATE_BYTES);
    memcpy(e->output, output, KECCAKF_STATE_BYTES);

    tags[s] = keccakf_fingerprint_tag(fingerprint);
    slot_entries[s] = (uint32_t)index;
    entries_used++;

    return index;
}

uint64_t keccakf_dedup_register (const uint64_t * input, const uint64_t * output)
{
    uint64_t fingerprint;
    uint64_t index = keccakf_dedup_find(input, &fingerprint);
    if (index != KECCAKF_DEDUP_NOT_FOUND) return keccakf_dedup_replica(index, NULL);
    return keccakf_dedup_add(input, fingerprint, output);
}

uint64_t keccakf_dedup_unique_count (void)
{
    return entries_used;
}

uint64_t keccakf_dedup_replicas (uint64_t index)
{
    return (index < entries_used) ? entries[index].replicas : 0;
}

uint64_t keccakf_dedup_replica_count (void)
{
    return replica_count;
}

void keccakf_dedup_reset (void)
{
    if (tags != NULL) memset(tags, 0, slots_size * sizeof(uint8_t));
    entries_used = 0;
    replica_count = 0;
}

void keccakf_dedup_free (void)
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
    replica_count = 0;
}
