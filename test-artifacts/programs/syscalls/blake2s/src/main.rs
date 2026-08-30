#![no_main]
ziskos::entrypoint!(main);

use rand::Rng;

use zisk_precomp_helpers::blake2s_round;
use ziskos::syscalls::{syscall_blake2s_round, SyscallBlake2sRoundParams};

fn main() {
    let num_rounds: u64 = ziskos::io::read();

    let mut rng = rand::thread_rng();

    println!("Running BLAKE2s round tests {} times", num_rounds);

    for _ in 0..num_rounds {
        blake2s_apply(&mut rng);
    }
}

/// Drive one BLAKE2s round through the precompile and check it against the
/// software reference. Words are 32-bit but travel one per 64-bit slot with a
/// zero high half — the layout the Blake2sr AIR enforces through its memory
/// argument, so a non-zero high half would be unprovable.
fn blake2s_apply(rng: &mut rand::rngs::ThreadRng) {
    let index: u64 = rng.gen_range(0..10);

    let mut state = [0u64; 16];
    for s in state.iter_mut() {
        *s = rng.gen::<u32>() as u64;
    }

    let mut input = [0u64; 16];
    for m in input.iter_mut() {
        *m = rng.gen::<u32>() as u64;
    }

    // Software reference over the same inputs
    let mut expected_v = [0u32; 16];
    let mut expected_m = [0u32; 16];
    for i in 0..16 {
        expected_v[i] = state[i] as u32;
        expected_m[i] = input[i] as u32;
    }
    blake2s_round(&mut expected_v, &expected_m, index as u32);

    // The precompile
    let mut params = SyscallBlake2sRoundParams { index, state: &mut state, input: &input };
    assert!(syscall_blake2s_round(&mut params));

    for i in 0..16 {
        assert_eq!(
            state[i], expected_v[i] as u64,
            "blake2s round {index} word {i}: precompile {} != reference {}",
            state[i], expected_v[i]
        );
    }
}
