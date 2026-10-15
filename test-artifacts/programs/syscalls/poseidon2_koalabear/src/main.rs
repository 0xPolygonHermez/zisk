#![no_main]
ziskos::entrypoint!(main);

use rand::Rng;

use zisk_precomp_helpers::POSEIDON2_KOALABEAR_SP1;
use ziskos::syscalls::syscall_poseidon2_koalabear;

fn main() {
    // Get the input from ziskos
    let num_perms: u64 = ziskos::io::read();
    println!("Running Poseidon2 KoalaBear random tests for {} times", num_perms);

    let p = POSEIDON2_KOALABEAR_SP1.p;
    let mut rng = rand::thread_rng();
    for k in 0..num_perms {
        // The first two states are the boundaries 0 and P - 1
        let lane = |rng: &mut rand::rngs::ThreadRng| match k {
            0 => 0,
            1 => u64::from(p - 1),
            _ => u64::from(rng.gen_range(0..p)),
        };
        let mut state = [0u64; 8];
        for word in state.iter_mut() {
            *word = lane(&mut rng) | (lane(&mut rng) << 32);
        }

        let mut expected = state;
        POSEIDON2_KOALABEAR_SP1.permute_words(&mut expected).unwrap();

        unsafe { syscall_poseidon2_koalabear(&mut state) };

        assert!(
            state == expected,
            "Poseidon2 KoalaBear state mismatch: \n  expected: {:x?}\n     found: {:x?}",
            expected,
            state
        );
    }
    println!("Poseidon2 KoalaBear tests passed");
}
