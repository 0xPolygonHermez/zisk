#![no_main]
ziskos::entrypoint!(main);

use rand::Rng;

use zisk_precomp_helpers::blake2s_f;
use ziskos::syscalls::{syscall_blake2sf, SyscallBlake2sfParams};

const ACTIVATE_CONSISTENCY_TEST: bool = false;

fn main() {
    // Get the input from ziskos
    let num_blake2s: u64 = ziskos::io::read();

    let mut rng = rand::thread_rng();

    if ACTIVATE_CONSISTENCY_TEST {
        println!("Running BLAKE2s consistency test for {} times", num_blake2s);
    } else {
        println!("Running BLAKE2s random tests for {} times", num_blake2s);
    }

    for _ in 0..num_blake2s {
        if ACTIVATE_CONSISTENCY_TEST {
            run_consistency_test();
        } else {
            blake2s_apply(&mut rng);
        }
    }
}

// Take any number and apply the blake2s permutation
fn blake2s_apply(rng: &mut rand::rngs::ThreadRng) {
    let mut state = [0u64; 8];
    for i in 0..state.len() {
        state[i] = rng.gen();
    }

    let mut input = [0u64; 8];
    for i in 0..input.len() {
        input[i] = rng.gen();
    }

    let state_copy = state.clone();

    let mut params = SyscallBlake2sfParams { state: &mut state, input: &input };
    syscall_blake2sf(&mut params);

    // Compare against the reference blake2s implementation
    let mut expected = state_copy;
    {
        let expected_u32: &mut [u32; 16] =
            unsafe { &mut *(expected.as_mut_ptr() as *mut [u32; 16]) };
        let input_u32: &[u32; 16] = unsafe { &*(input.as_ptr() as *const [u32; 16]) };
        blake2s_f(expected_u32, input_u32);
    }

    assert!(
        state == expected,
        "BLAKE2s state mismatch: \n  expected: {:x?}\n     found: {:x?}",
        expected,
        state
    );
}

fn run_consistency_test() {
    // The final-block compression of "abc" for BLAKE2s-256 (RFC 7693, Appendix B):
    // h = IV with the parameter block folded into h[0], t = 3, f = true. This is the raw
    // 10-round permutation output, before the compression feed-forward
    let mut state: [u64; 8] = [
        0xbb67ae856b08e647,
        0xa54ff53a3c6ef372,
        0x9b05688c510e527f,
        0x5be0cd191f83d9ab,
        0xbb67ae856a09e667,
        0xa54ff53a3c6ef372,
        0x9b05688c510e527c,
        0x5be0cd19e07c2654,
    ];
    let input: [u64; 8] = [0x636261, 0, 0, 0, 0, 0, 0, 0];

    let mut params = SyscallBlake2sfParams { state: &mut state, input: &input };
    syscall_blake2sf(&mut params);

    const EXPECTED_RESULT: [u64; 8] = [
        0xcfec3aa6d9c994aa,
        0x2c38670e700d0ab2,
        0x1d023ef3af6a1f66,
        0x945357a51d9ec27d,
        0x969fe8113e9ffebd,
        0xa632797aef485e21,
        0xaf3d80e1deef082e,
        0x4deafd3a4e86829b,
    ];
    assert!(
        state == EXPECTED_RESULT,
        "BLAKE2s state mismatch: \n  expected: {:x?}\n     found: {:x?}",
        EXPECTED_RESULT,
        state
    );
}
