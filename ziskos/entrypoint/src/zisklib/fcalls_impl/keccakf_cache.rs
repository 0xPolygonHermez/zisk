//! Software implementation of the Keccak-f cache fcalls.
//!
//! Keeps a `state -> index` map of Keccak-f\[1600\] *input* states, so a program can ask the
//! executor whether a permutation it is about to run has already been executed and, if so,
//! reuse the index it was registered under instead of running it again.
//!
//! The map is an open-addressed table keyed by a cheap 64-bit fingerprint of the 25-word
//! state; the full 25 words are only compared when a fingerprint matches, so a lookup costs
//! one hash plus (almost always) a single probe.
//!
//! Two users share this code:
//! - the emulator, which owns one [`KeccakfCache`] per instruction context, and
//! - native (non-zkVM) execution, which uses the thread-local instance at the bottom of this
//!   module through [`keccakf_cache_set_index`], [`keccakf_cache_get_index`] and
//!   [`keccakf_cache_on_keccakf`].

use std::cell::RefCell;

use crate::zisklib::KECCAKF_CACHE_INDEX_NOT_FOUND;

/// Number of 64-bit words of a Keccak-f\[1600\] state.
pub const KECCAKF_STATE_WORDS: usize = 25;

/// Number of slots of a freshly grown table. Power of two: the probe index is masked.
const INITIAL_SLOTS: usize = 1 << 10;

/// Multipliers for [`mix`]; odd 64-bit constants taken from wyhash.
const MIX_KEYS: [u64; 5] = [
    0xa076_1d64_78bd_642f,
    0xe703_7ed1_a0b4_28db,
    0x8ebc_6af0_9c88_c6e3,
    0x5899_65cc_7537_4cc3,
    0x1d8e_4e27_c47d_124f,
];

/// Folds a 128-bit product back into 64 bits.
#[inline(always)]
fn mix(a: u64, b: u64) -> u64 {
    let r = (a as u128).wrapping_mul(b as u128);
    (r as u64) ^ ((r >> 64) as u64)
}

/// Fingerprints a 25-word state into a non-zero 64-bit value (0 marks an empty slot).
///
/// The state is folded through four independent lanes so the multiplies pipeline instead of
/// forming one 25-long dependency chain: this runs once per lookup and once per cached state.
#[inline]
fn fingerprint(state: &[u64]) -> u64 {
    debug_assert_eq!(state.len(), KECCAKF_STATE_WORDS);

    let mut l0 = MIX_KEYS[0];
    let mut l1 = MIX_KEYS[1];
    let mut l2 = MIX_KEYS[2];
    let mut l3 = MIX_KEYS[3];

    let mut i = 0;
    while i < KECCAKF_STATE_WORDS - 1 {
        l0 = mix(l0 ^ state[i], MIX_KEYS[1]);
        l1 = mix(l1 ^ state[i + 1], MIX_KEYS[2]);
        l2 = mix(l2 ^ state[i + 2], MIX_KEYS[3]);
        l3 = mix(l3 ^ state[i + 3], MIX_KEYS[4]);
        i += 4;
    }

    let hash =
        mix(l0 ^ l1, l2 ^ l3) ^ mix(state[KECCAKF_STATE_WORDS - 1] ^ MIX_KEYS[0], MIX_KEYS[4]);

    if hash == 0 {
        1
    } else {
        hash
    }
}

/// One open-addressed slot. `hash == 0` marks it empty.
#[derive(Clone, Copy, Debug, Default)]
struct Slot {
    /// Fingerprint of the cached state, never 0 when the slot is occupied.
    hash: u64,
    /// Index the state was cached under.
    index: u64,
    /// Offset of the state in [`KeccakfCache::states`], in words.
    offset: usize,
}

/// Maps Keccak-f input states to the index they were registered under.
#[derive(Debug, Default)]
pub struct KeccakfCache {
    /// Open-addressed slots, linearly probed. Empty or a power-of-two length.
    slots: Vec<Slot>,

    /// Cached states, [`KECCAKF_STATE_WORDS`] words each, addressed by [`Slot::offset`].
    states: Vec<u64>,

    /// Number of occupied slots.
    len: usize,

    /// Index the next executed Keccak-f must be cached under, set by
    /// `fcall_set_keccakf_cache_index`.
    pending_index: Option<u64>,
}

impl KeccakfCache {
    /// Requests that the next executed Keccak-f be cached under `index`.
    ///
    /// Panics if `index` is [`KECCAKF_CACHE_INDEX_NOT_FOUND`], which is reserved to report a
    /// miss and would make the entry indistinguishable from one.
    pub fn set_pending_index(&mut self, index: u64) {
        if index == KECCAKF_CACHE_INDEX_NOT_FOUND {
            panic!(
                "KeccakfCache::set_pending_index() called with the reserved not-found index {index:#x}"
            );
        }
        self.pending_index = Some(index);
    }

    /// Consumes the index requested by [`Self::set_pending_index`], if any.
    pub fn take_pending_index(&mut self) -> Option<u64> {
        self.pending_index.take()
    }

    /// Caches `state` (the 25 words as read *before* the permutation) under `index`.
    ///
    /// If the state is already cached, its index is updated to `index`.
    pub fn store(&mut self, state: &[u64], index: u64) {
        assert_eq!(
            state.len(),
            KECCAKF_STATE_WORDS,
            "KeccakfCache::store() called with {} words",
            state.len()
        );

        // Keep the load factor at or below 1/2 so probe runs stay short
        if (self.len + 1) * 2 > self.slots.len() {
            self.grow();
        }

        let hash = fingerprint(state);
        let mask = self.slots.len() - 1;
        let mut slot = hash as usize & mask;
        while self.slots[slot].hash != 0 {
            if self.slots[slot].hash == hash && self.state_at(self.slots[slot].offset) == state {
                // Already cached: keep the most recent index
                self.slots[slot].index = index;
                return;
            }
            slot = (slot + 1) & mask;
        }

        let offset = self.states.len();
        self.states.extend_from_slice(state);
        self.slots[slot] = Slot { hash, index, offset };
        self.len += 1;
    }

    /// Returns the index `state` was cached under, or [`KECCAKF_CACHE_INDEX_NOT_FOUND`].
    pub fn get(&self, state: &[u64]) -> u64 {
        assert_eq!(
            state.len(),
            KECCAKF_STATE_WORDS,
            "KeccakfCache::get() called with {} words",
            state.len()
        );

        if self.slots.is_empty() {
            return KECCAKF_CACHE_INDEX_NOT_FOUND;
        }

        let hash = fingerprint(state);
        let mask = self.slots.len() - 1;
        let mut slot = hash as usize & mask;
        while self.slots[slot].hash != 0 {
            if self.slots[slot].hash == hash && self.state_at(self.slots[slot].offset) == state {
                return self.slots[slot].index;
            }
            slot = (slot + 1) & mask;
        }

        KECCAKF_CACHE_INDEX_NOT_FOUND
    }

    /// Empties the cache and releases its memory.
    pub fn clear(&mut self) {
        self.slots = Vec::new();
        self.states = Vec::new();
        self.len = 0;
        self.pending_index = None;
    }

    /// Number of cached states.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if no state has been cached yet.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The state stored at `offset` words into [`Self::states`].
    #[inline]
    fn state_at(&self, offset: usize) -> &[u64] {
        &self.states[offset..offset + KECCAKF_STATE_WORDS]
    }

    /// Doubles the slot table (allocating it on first use) and reinserts every entry.
    fn grow(&mut self) {
        let new_len = if self.slots.is_empty() { INITIAL_SLOTS } else { self.slots.len() * 2 };
        let old_slots = std::mem::replace(&mut self.slots, vec![Slot::default(); new_len]);

        let mask = new_len - 1;
        for entry in old_slots.iter().filter(|slot| slot.hash != 0) {
            let mut slot = entry.hash as usize & mask;
            while self.slots[slot].hash != 0 {
                slot = (slot + 1) & mask;
            }
            self.slots[slot] = *entry;
        }
    }
}

thread_local! {
    /// Cache used when a ZisK program runs natively, i.e. with no executor behind the fcalls.
    /// One per thread, mirroring the one-per-execution lifetime it has in the emulator.
    static NATIVE_CACHE: RefCell<KeccakfCache> = RefCell::new(KeccakfCache::default());
}

/// Native implementation of `fcall_set_keccakf_cache_index`.
pub fn keccakf_cache_set_index(index: u64) {
    NATIVE_CACHE.with(|cache| cache.borrow_mut().set_pending_index(index));
}

/// Native implementation of `fcall_get_keccakf_cache_index`.
pub fn keccakf_cache_get_index(state: &[u64; KECCAKF_STATE_WORDS]) -> u64 {
    NATIVE_CACHE.with(|cache| cache.borrow().get(state))
}

/// Caches `state` under the pending index, if `fcall_set_keccakf_cache_index` requested one.
///
/// Called by the native `syscall_keccak_f` with the state *before* the permutation.
pub fn keccakf_cache_on_keccakf(state: &[u64; KECCAKF_STATE_WORDS]) {
    NATIVE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(index) = cache.take_pending_index() {
            cache.store(state, index);
        }
    });
}

/// Empties the native cache and releases its memory.
pub fn keccakf_cache_clear() {
    NATIVE_CACHE.with(|cache| cache.borrow_mut().clear());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A state whose words all derive from `seed`.
    fn state(seed: u64) -> [u64; KECCAKF_STATE_WORDS] {
        let mut state = [0u64; KECCAKF_STATE_WORDS];
        for (i, word) in state.iter_mut().enumerate() {
            *word = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(i as u64);
        }
        state
    }

    #[test]
    fn miss_on_empty_cache() {
        let cache = KeccakfCache::default();
        assert_eq!(cache.get(&state(1)), KECCAKF_CACHE_INDEX_NOT_FOUND);
        assert!(cache.is_empty());
    }

    #[test]
    fn stores_and_finds_states() {
        let mut cache = KeccakfCache::default();
        for i in 0..1000u64 {
            cache.store(&state(i), i * 7);
        }
        assert_eq!(cache.len(), 1000);
        for i in 0..1000u64 {
            assert_eq!(cache.get(&state(i)), i * 7);
        }
        assert_eq!(cache.get(&state(1000)), KECCAKF_CACHE_INDEX_NOT_FOUND);
    }

    #[test]
    fn distinguishes_states_differing_in_one_word() {
        let mut cache = KeccakfCache::default();
        let base = state(42);
        cache.store(&base, 5);
        for i in 0..KECCAKF_STATE_WORDS {
            let mut other = base;
            other[i] ^= 1;
            assert_eq!(cache.get(&other), KECCAKF_CACHE_INDEX_NOT_FOUND, "word {i}");
        }
        assert_eq!(cache.get(&base), 5);
    }

    #[test]
    fn restoring_a_state_updates_its_index() {
        let mut cache = KeccakfCache::default();
        cache.store(&state(3), 1);
        cache.store(&state(3), 2);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&state(3)), 2);
    }

    #[test]
    fn pending_index_is_consumed_once() {
        let mut cache = KeccakfCache::default();
        assert_eq!(cache.take_pending_index(), None);
        cache.set_pending_index(9);
        assert_eq!(cache.take_pending_index(), Some(9));
        assert_eq!(cache.take_pending_index(), None);
    }

    #[test]
    #[should_panic(expected = "reserved not-found index")]
    fn rejects_the_reserved_index() {
        KeccakfCache::default().set_pending_index(KECCAKF_CACHE_INDEX_NOT_FOUND);
    }

    /// Walks the native path end to end: the fcall wrappers and the `syscall_keccak_f` hook.
    #[test]
    fn native_fcalls_round_trip() {
        use crate::{
            syscalls::syscall_keccak_f,
            zisklib::{fcall_get_keccakf_cache_index, fcall_set_keccakf_cache_index},
        };

        #[cfg(feature = "hints")]
        let mut hints = Vec::new();

        keccakf_cache_clear();

        let mut permuted = state(7);
        let input = permuted;
        assert_eq!(fcall_get_keccakf_cache_index(&input), KECCAKF_CACHE_INDEX_NOT_FOUND);

        fcall_set_keccakf_cache_index(11);
        unsafe {
            syscall_keccak_f(
                &mut permuted,
                #[cfg(feature = "hints")]
                &mut hints,
            )
        };

        assert_eq!(fcall_get_keccakf_cache_index(&input), 11);
        assert_eq!(fcall_get_keccakf_cache_index(&permuted), KECCAKF_CACHE_INDEX_NOT_FOUND);

        // The registration was consumed, so this permutation is not cached
        let mut other = state(8);
        let other_input = other;
        unsafe {
            syscall_keccak_f(
                &mut other,
                #[cfg(feature = "hints")]
                &mut hints,
            )
        };
        assert_eq!(fcall_get_keccakf_cache_index(&other_input), KECCAKF_CACHE_INDEX_NOT_FOUND);

        keccakf_cache_clear();
    }

    #[test]
    fn clear_empties_the_cache() {
        let mut cache = KeccakfCache::default();
        cache.store(&state(1), 1);
        cache.set_pending_index(2);
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.take_pending_index(), None);
        assert_eq!(cache.get(&state(1)), KECCAKF_CACHE_INDEX_NOT_FOUND);
    }
}
