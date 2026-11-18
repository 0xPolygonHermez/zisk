//! The C ABI entry points that take a `(pointer, length)` input must accept a null pointer
//! when the length is 0: C callers commonly pass one for an empty buffer, and the empty
//! input is a valid call (the hash of the empty string, a pairing check of zero pairs).

use std::ptr::{self, NonNull};

use zisk_zkvm_interface::{
    zkvm_bn254_pairing_pair, zkvm_keccak256_hash, zkvm_ripemd160_hash, zkvm_sha256_hash,
    zkvm_status_ZKVM_EOK as ZKVM_EOK,
};
use ziskos::zisklib::native_keccak256;
use ziskos::zisklib::zkvm_accelerators::{
    zkvm_bn254_pairing, zkvm_keccak256, zkvm_ripemd160, zkvm_sha256,
};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

const KECCAK256_EMPTY: &str = "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470";
const SHA256_EMPTY: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
/// Right-aligned in 32 bytes, as the ABI returns it.
const RIPEMD160_EMPTY: &str = "0000000000000000000000009c1185a5c5e9fc54612808977ee8f548b2258d31";

#[test]
fn keccak256_hashes_a_null_empty_input() {
    let mut out = zkvm_keccak256_hash { data: [0xff; 32] };
    let status = unsafe { zkvm_keccak256(ptr::null(), 0, &mut out) };
    assert_eq!(status, ZKVM_EOK);
    assert_eq!(hex(&out.data), KECCAK256_EMPTY);

    let mut out = [0xffu8; 32];
    unsafe { native_keccak256(ptr::null(), 0, out.as_mut_ptr()) };
    assert_eq!(hex(&out), KECCAK256_EMPTY);
}

#[test]
fn sha256_hashes_a_null_empty_input() {
    let mut out = zkvm_sha256_hash { data: [0xff; 32] };
    let status = unsafe { zkvm_sha256(ptr::null(), 0, &mut out) };
    assert_eq!(status, ZKVM_EOK);
    assert_eq!(hex(&out.data), SHA256_EMPTY);
}

#[test]
fn ripemd160_hashes_a_null_empty_input() {
    let mut out = zkvm_ripemd160_hash { data: [0xff; 32] };
    let status = unsafe { zkvm_ripemd160(ptr::null(), 0, &mut out) };
    assert_eq!(status, ZKVM_EOK);
    assert_eq!(hex(&out.data), RIPEMD160_EMPTY);
}

#[test]
fn bn254_pairing_of_zero_null_pairs_matches_an_empty_slice() {
    let check = |pairs: *const zkvm_bn254_pairing_pair| {
        let mut verified = false;
        let status = unsafe { zkvm_bn254_pairing(pairs, 0, &mut verified) };
        (status, verified)
    };
    let empty = check(NonNull::<zkvm_bn254_pairing_pair>::dangling().as_ptr());
    assert_eq!(check(ptr::null()), empty);
    // EIP-197: the empty product of pairings is the identity, so the check holds.
    assert_eq!(empty, (ZKVM_EOK, true));
}
