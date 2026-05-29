#![no_main]
ziskos::entrypoint!(main);

use rand::Rng;

use ziskos::syscalls::syscall_poseidon2;

fn main() {
    // Get the input from ziskos
    let num_poseidon2s: u64 = ziskos::io::read();

    let mut rng = rand::thread_rng();

    println!("Running POSEIDON2 random tests for {} times", num_poseidon2s);

    for _ in 0..num_poseidon2s {
        poseidon2_apply(&mut rng);
    }
}

// Generate a random input and apply the poseidon2 function to it
fn poseidon2_apply(rng: &mut rand::rngs::ThreadRng) {
    let mut state = [0u64; 16];
    for i in 0..16 {
        state[i] = rng.gen();
    }

    // // Make a copy of the state to compare results later
    // let mut state_copy = state.clone();

    // Call the syscall implementation of poseidon2
    unsafe {
        syscall_poseidon2(&mut state);
    }

    // TODO!
    // // Compare against an audited poseidon2 implementation
    // poseidon2(&mut state_copy);

    // assert!(
    //     state == state_copy,
    //     "POSEIDON2 state mismatch: \n  expected: {:x?}\n     found: {:x?}",
    //     state_copy,
    //     state
    // );
}
