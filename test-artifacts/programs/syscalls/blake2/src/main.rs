#![no_main]
ziskos::entrypoint!(main);

use rand::Rng;

use precompiles_helpers::blake2b_round;
use ziskos::syscalls::{syscall_blake2b_round, SyscallBlake2bRoundParams};

const ACTIVATE_CONSISTENCY_TEST: bool = false;

fn main() {
    // Get the input from ziskos
    let num_blake2s: u64 = ziskos::io::read();

    let mut rng = rand::thread_rng();

    if ACTIVATE_CONSISTENCY_TEST {
        println!("Running BLAKE2 consistency test for {} times", num_blake2s);
    } else {
        println!("Running BLAKE2 random tests for {} times", num_blake2s);
    }

    for _ in 0..num_blake2s {
        if ACTIVATE_CONSISTENCY_TEST {
            run_consistency_test();
        } else {
            blake2_apply(&mut rng);
        }
    }
}

// Take any number and apply the blake2 function
#[allow(deprecated)]
fn blake2_apply(rng: &mut rand::rngs::ThreadRng) {
    let index: u64 = rng.gen_range(0..10);

    let mut state = [0u64; 16];
    for i in 0..state.len() {
        state[i] = rng.gen();
    }

    let mut input = [0u64; 16];
    for i in 0..input.len() {
        input[i] = rng.gen();
    }

    let mut state_copy = state.clone();

    let mut params = SyscallBlake2bRoundParams { index, state: &mut state, input: &input };
    syscall_blake2b_round(&mut params);

    // Compare against a tested blake2 implementation
    blake2b_round(&mut state_copy, &input, index as u32);

    assert!(
        state == state_copy,
        "BLAKE2 state mismatch: \n  expected: {:x?}\n     found: {:x?}",
        state_copy,
        state
    );
}

fn run_consistency_test() {
    let index = 0;
    let mut state: [u64; 16] = [
        0x6a09e667f2bdc948,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d2,
        0x9b05688c2b3e6c1f,
        0xe07c265404be4294,
        0x5be0cd19137e2179,
    ];
    let input: [u64; 16] = [0x636261, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    let mut params = SyscallBlake2bRoundParams { index, state: &mut state, input: &input };
    syscall_blake2b_round(&mut params);

    const EXPECTED_RESULT: [u64; 16] = [
        0x86b7c1568029bb79,
        0xc12cbcc809ff59f3,
        0xc6a5214cc0eaca8e,
        0xc87cd524c14cc5d,
        0x44ee6039bd86a9f7,
        0xa447c850aa694a7e,
        0xde080f1bb1c0f84b,
        0x595cb8a9a1aca66c,
        0xbec3ae837eac4887,
        0x6267fc79df9d6ad1,
        0xfa87b01273fa6dbe,
        0x521a715c63e08d8a,
        0xe02d0975b8d37a83,
        0x1c7b754f08b7d193,
        0x8f885a76b6e578fe,
        0x2318a24e2140fc64,
    ];
    assert!(
        state == EXPECTED_RESULT,
        "BLAKE2 state mismatch: \n  expected: {:x?}\n     found: {:x?}",
        EXPECTED_RESULT,
        state
    );
}
