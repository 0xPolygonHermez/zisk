//! Aggregator/verifier guest: reads the two expected keys, two proofs, and the hash family
//! they were proven under from stdin, verifies each via `verify_zisk_proof_with_hash_c`,
//! panics on failure.
//!
//! SECURITY: a real guest MUST hardcode both keys. Reading them from input, as this
//! cross-setup demo does, authenticates nothing -- see `verify_zisk_proof`.

#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let expected_setup_vk = ziskos::io::read_slice();
    let expected_program_vk = ziskos::io::read_slice();
    let proof1 = ziskos::io::read_slice();
    let proof2 = ziskos::io::read_slice();
    // A proof carries no family tag, so the host names the proving key's family here.
    // Verifying under the wrong one fails every proof.
    let hash = ziskos::io::read_slice();

    let valid_proof1 = unsafe {
        ziskos::zisklib::verify_zisk_proof_with_hash_c(
            proof1.as_ptr(),
            proof1.len(),
            expected_setup_vk.as_ptr(),
            expected_setup_vk.len(),
            expected_program_vk.as_ptr(),
            expected_program_vk.len(),
            hash.as_ptr(),
            hash.len(),
        )
    };
    if !valid_proof1 {
        panic!("Proof 1 verification failed");
    }

    let valid_proof2 = unsafe {
        ziskos::zisklib::verify_zisk_proof_with_hash_c(
            proof2.as_ptr(),
            proof2.len(),
            expected_setup_vk.as_ptr(),
            expected_setup_vk.len(),
            expected_program_vk.as_ptr(),
            expected_program_vk.len(),
            hash.as_ptr(),
            hash.len(),
        )
    };
    if !valid_proof2 {
        panic!("Proof 2 verification failed");
    }
}
