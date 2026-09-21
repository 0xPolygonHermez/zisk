#![no_main]
ziskos::entrypoint!(main);

use rand::Rng;

use zisk_precomp_helpers::blake3_f;
use ziskos::syscalls::{syscall_blake3f, SyscallBlake3fParams};

const ACTIVATE_CONSISTENCY_TEST: bool = false;

fn main() {
    // Get the input from ziskos
    let num_blake3s: u64 = ziskos::io::read();

    let mut rng = rand::thread_rng();

    if ACTIVATE_CONSISTENCY_TEST {
        println!("Running BLAKE3 consistency test for {} times", num_blake3s);
    } else {
        println!("Running BLAKE3 random tests for {} times", num_blake3s);
    }

    for _ in 0..num_blake3s {
        if ACTIVATE_CONSISTENCY_TEST {
            run_consistency_test();
        } else {
            blake3_apply(&mut rng);
        }
    }
}

// Take any number and apply the blake3 permutation
fn blake3_apply(rng: &mut rand::rngs::ThreadRng) {
    let mut state = [0u64; 8];
    for i in 0..state.len() {
        state[i] = rng.gen();
    }

    let mut input = [0u64; 8];
    for i in 0..input.len() {
        input[i] = rng.gen();
    }

    let state_copy = state.clone();

    let mut params = SyscallBlake3fParams { state: &mut state, input: &input };
    syscall_blake3f(&mut params);

    // Compare against the reference blake3 implementation
    let mut expected = state_copy;
    {
        let expected_u32: &mut [u32; 16] =
            unsafe { &mut *(expected.as_mut_ptr() as *mut [u32; 16]) };
        let input_u32: &[u32; 16] = unsafe { &*(input.as_ptr() as *const [u32; 16]) };
        blake3_f(expected_u32, input_u32);
    }

    assert!(
        state == expected,
        "BLAKE3 state mismatch: \n  expected: {:x?}\n     found: {:x?}",
        expected,
        state
    );
}

fn run_consistency_test() {
    // The single-chunk root compression of "abc" (block_len = 3,
    // flags = CHUNK_START | CHUNK_END | ROOT): the raw 7-round permutation output,
    // before the compression feed-forward
    let mut state: [u64; 8] = [
        0xbb67ae856a09e667,
        0xa54ff53a3c6ef372,
        0x9b05688c510e527f,
        0x5be0cd191f83d9ab,
        0xbb67ae856a09e667,
        0xa54ff53a3c6ef372,
        0x0000000000000000,
        0x0000000b00000003,
    ];
    let input: [u64; 8] = [0x636261, 0, 0, 0, 0, 0, 0, 0];

    let mut params = SyscallBlake3fParams { state: &mut state, input: &input };
    syscall_blake3f(&mut params);

    const EXPECTED_RESULT: [u64; 8] = [
        0x58c37bce68ea631c,
        0x59cfd54f14e356a5,
        0xd4a1df268bf60c1a,
        0xd5c204ff811f35a9,
        0x6b923df6c4595478,
        0xec42ef6861d8e05a,
        0xd77aa67bcdaec952,
        0x505fb92aed830054,
    ];
    assert!(
        state == EXPECTED_RESULT,
        "BLAKE3 state mismatch: \n  expected: {:x?}\n     found: {:x?}",
        EXPECTED_RESULT,
        state
    );
}
