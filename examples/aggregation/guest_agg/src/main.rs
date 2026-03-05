// This example program takes a number `n` as input and computes the SHA-256 hash `n` times sequentially.

// Mark the main function as the entry point for ZisK
#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let proof1 = ziskos::io::read_proof();
    let proof2 = ziskos::io::read_proof();

    // Verify the first proof
    let valid_proof1 = ziskos::verify_zisk_proof(&proof1);
    if !valid_proof1 {
        panic!("Proof 1 verification failed");
    }

    // Verify the second proof
    let valid_proof2 = ziskos::verify_zisk_proof(&proof2);
    if !valid_proof2 {
        panic!("Proof 2 verification failed");
    }
}
