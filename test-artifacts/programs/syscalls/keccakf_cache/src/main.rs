#![no_main]
ziskos::entrypoint!(main);

use ziskos::{
    syscalls::syscall_keccak_f,
    zisklib::{
        fcall_get_keccakf_cache_index, fcall_set_keccakf_cache_index,
        KECCAKF_CACHE_INDEX_NOT_FOUND,
    },
};

/// Number of distinct Keccak-f states the test registers.
const STATES: u64 = 64;

/// A deterministic state derived from `seed`.
fn state_for(seed: u64) -> [u64; 25] {
    let mut state = [0u64; 25];
    for (i, word) in state.iter_mut().enumerate() {
        *word = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15).wrapping_add(i as u64);
    }
    state
}

fn main() {
    // Nothing is cached before anything is registered
    for seed in 0..STATES {
        let state = state_for(seed);
        assert_eq!(
            fcall_get_keccakf_cache_index(&state),
            KECCAKF_CACHE_INDEX_NOT_FOUND,
            "state {seed} was cached before being registered"
        );
    }

    // Register each state under an index while permuting it
    for seed in 0..STATES {
        let mut state = state_for(seed);
        fcall_set_keccakf_cache_index(seed * 7);
        unsafe { syscall_keccak_f(&mut state) };
    }

    // Every registered state is now a hit, and returns the index it was registered under
    for seed in 0..STATES {
        let state = state_for(seed);
        assert_eq!(
            fcall_get_keccakf_cache_index(&state),
            seed * 7,
            "state {seed} did not return the index it was registered under"
        );
    }

    // States that were never registered are still misses
    for seed in STATES..(2 * STATES) {
        let state = state_for(seed);
        assert_eq!(
            fcall_get_keccakf_cache_index(&state),
            KECCAKF_CACHE_INDEX_NOT_FOUND,
            "unregistered state {seed} was found in the cache"
        );
    }

    // The permuted output of a registered state is not itself cached
    let mut permuted = state_for(0);
    unsafe { syscall_keccak_f(&mut permuted) };
    assert_eq!(
        fcall_get_keccakf_cache_index(&permuted),
        KECCAKF_CACHE_INDEX_NOT_FOUND,
        "the output state of a registered Keccak-f was cached"
    );

    // A Keccak-f that is not preceded by a registration does not enter the cache: the run
    // above already consumed nothing, so this input must stay unknown
    let input = state_for(2 * STATES);
    let mut state = input;
    unsafe { syscall_keccak_f(&mut state) };
    assert_eq!(
        fcall_get_keccakf_cache_index(&input),
        KECCAKF_CACHE_INDEX_NOT_FOUND,
        "an unregistered Keccak-f entered the cache"
    );

    // A registration is consumed by the very next Keccak-f, and only by it
    let first = state_for(3 * STATES);
    let second = state_for(3 * STATES + 1);
    fcall_set_keccakf_cache_index(1234);
    let mut state = first;
    unsafe { syscall_keccak_f(&mut state) };
    let mut state = second;
    unsafe { syscall_keccak_f(&mut state) };
    assert_eq!(fcall_get_keccakf_cache_index(&first), 1234);
    assert_eq!(fcall_get_keccakf_cache_index(&second), KECCAKF_CACHE_INDEX_NOT_FOUND);

    println!("keccakf cache tests OK");
}
