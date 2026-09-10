//! Aggregator/verifier guest. Reads two proof byte streams and the hash family they were
//! proven under from stdin, and verifies each via
//! `ziskos::zisklib::verify_zisk_proof_with_hash_c`. Panics if either proof fails.

#![no_main]
ziskos::entrypoint!(main);

fn main() {
    let proof1 = ziskos::io::read_slice();
    let proof2 = ziskos::io::read_slice();
    // A proof carries no family tag, so the host names the proving key's family here.
    // Verifying under the wrong one fails every proof.
    let hash = ziskos::io::read_slice();

    let valid_proof1 = unsafe {
        ziskos::zisklib::verify_zisk_proof_with_hash_c(
            proof1.as_ptr(),
            proof1.len(),
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
            hash.as_ptr(),
            hash.len(),
        )
    };
    if !valid_proof2 {
        panic!("Proof 2 verification failed");
    }
}
