#![no_main]
ziskos::entrypoint!(main);

use ziskos::syscalls::{syscall_sha256_f, SyscallSha256Params};

#[allow(deprecated)]
use generic_array::{typenum::U64, GenericArray};
use rand::Rng;
use sha2::compress256;

const ACTIVATE_CONSISTENCY_TEST: bool = false;

fn main() {
    // Get the input from ziskos
    let num_sha256fs: u64 = ziskos::io::read();

    let mut rng = rand::thread_rng();

    if ACTIVATE_CONSISTENCY_TEST {
        println!("Running SHA256F consistency test for {} times", num_sha256fs);
    } else {
        println!("Running SHA256F random tests for {} times", num_sha256fs);
    }

    for _ in 0..num_sha256fs {
        if ACTIVATE_CONSISTENCY_TEST {
            run_consistency_test();
        } else {
            sha256f_apply(&mut rng);
        }
    }
}

// Take any number and apply the sha256f function
#[allow(deprecated)]
fn sha256f_apply(rng: &mut rand::rngs::ThreadRng) {
    let mut state_u32 = [0u32; 8];
    for i in 0..8 {
        state_u32[i] = rng.gen();
    }

    let mut input_u8 = [0u8; 64];
    for i in 0..64 {
        input_u8[i] = rng.gen();
    }

    let mut state_u32_copy = state_u32.clone();

    let state: &mut [u64; 4] = unsafe { &mut *(state_u32.as_mut_ptr() as *mut [u64; 4]) };
    let input: &[u64; 8] = unsafe { &*(input_u8.as_ptr() as *const [u64; 8]) };
    let mut params = SyscallSha256Params { state, input };
    syscall_sha256_f(&mut params);

    // Compare against an audited sha256f implementation
    let input_ga: GenericArray<u8, U64> = GenericArray::clone_from_slice(&input_u8);
    compress256(&mut state_u32_copy, &[input_ga]);
    let expected_result: &[u64; 4] = unsafe { &*(state_u32_copy.as_ptr() as *const [u64; 4]) };

    assert!(
        state == expected_result,
        "SHA256F state mismatch: \n  expected: {:x?}\n     found: {:x?}",
        expected_result,
        state
    );
}

fn run_consistency_test() {
    const SHA256F_INITIAL_HASH_STATE: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    const SHA256F_INPUT: [u8; 64] = {
        let mut a = [0u8; 64];
        a[0] = 0x80;
        a
    };

    let mut state: [u64; 4] = unsafe { *(SHA256F_INITIAL_HASH_STATE.as_ptr() as *const [u64; 4]) };
    let input: [u64; 8] = unsafe { *(SHA256F_INPUT.as_ptr() as *const [u64; 8]) };

    let mut params = SyscallSha256Params { state: &mut state, input: &input };
    syscall_sha256_f(&mut params);

    const EXPECTED_RESULT: [u32; 8] = [
        0xe3b0c442, 0x98fc1c14, 0x9afbf4c8, 0x996fb924, 0x27ae41e4, 0x649b934c, 0xa495991b,
        0x7852b855,
    ];
    let expected_result: [u64; 4] = unsafe { *(EXPECTED_RESULT.as_ptr() as *const [u64; 4]) };

    assert!(
        state == expected_result,
        "SHA256F state mismatch: \n  expected: {:x?}\n     found: {:x?}",
        expected_result,
        state
    );
}
