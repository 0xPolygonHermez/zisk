//! Aggregator/verifier guest: reads the two expected keys and two proofs from
//! stdin, verifies each via `verify_zisk_proof_c`, panics on failure. The hash
//! family travels inside each proof, so it is not an input.
//!
//! SECURITY: a real guest MUST hardcode both keys. Reading them from input, as
//! this cross-setup demo does, authenticates nothing -- see `verify_zisk_proof`.

#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let expected_setup_vk = ziskos::io::read_slice();
    let expected_program_vk = ziskos::io::read_slice();
    let proof1 = ziskos::io::read_slice();
    let proof2 = ziskos::io::read_slice();

    let valid_proof1 = unsafe {
        ziskos::zisklib::verify_zisk_proof_c(
            proof1.as_ptr(),
            proof1.len(),
            expected_setup_vk.as_ptr(),
            expected_setup_vk.len(),
            expected_program_vk.as_ptr(),
            expected_program_vk.len(),
        )
    };
    if !valid_proof1 {
        panic!("Proof 1 verification failed");
    }

    let valid_proof2 = unsafe {
        ziskos::zisklib::verify_zisk_proof_c(
            proof2.as_ptr(),
            proof2.len(),
            expected_setup_vk.as_ptr(),
            expected_setup_vk.len(),
            expected_program_vk.as_ptr(),
            expected_program_vk.len(),
        )
    };
    if !valid_proof2 {
        panic!("Proof 2 verification failed");
    }
}
