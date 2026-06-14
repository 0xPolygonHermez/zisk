#![no_main]
ziskos::entrypoint!(main);

use rand::Rng;

use ziskos::syscalls::syscall_poseidon1;

fn main() {
    // Get the input from ziskos
    let num_poseidon1s: u64 = ziskos::io::read();

    let mut rng = rand::thread_rng();

    println!("Running POSEIDON1 random tests for {} times", num_poseidon1s);

    for _ in 0..num_poseidon1s {
        poseidon1_apply(&mut rng);
    }
}

// Generate a random input and apply the poseidon1 function to it
fn poseidon1_apply(rng: &mut rand::rngs::ThreadRng) {
    let mut state = [0u64; 16];
    for i in 0..16 {
        state[i] = rng.gen();
    }

    // Call the syscall implementation of poseidon1
    unsafe {
        syscall_poseidon1(&mut state);
    }
}
