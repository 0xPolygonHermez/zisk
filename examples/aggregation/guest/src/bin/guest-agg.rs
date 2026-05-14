// This example program takes a number `n` as input and computes the SHA-256 hash `n` times sequentially.

// Mark the main function as the entry point for ZisK
#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let proof1 = ziskos::io::read_input_slice();
    let proof2 = ziskos::io::read_input_slice();

    // Verify the first proof
    let valid_proof1 =
        unsafe { ziskos::zisklib::verify_zisk_proof_c(proof1.as_ptr(), proof1.len()) };
    if !valid_proof1 {
        panic!("Proof 1 verification failed");
    }

    // Verify the second proof
    let valid_proof2 =
        unsafe { ziskos::zisklib::verify_zisk_proof_c(proof2.as_ptr(), proof2.len()) };
    if !valid_proof2 {
        panic!("Proof 2 verification failed");
    }
}
