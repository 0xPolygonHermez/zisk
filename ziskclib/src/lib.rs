use tiny_keccak::keccakf;

mod helpers;
use helpers::sha256f;

#[no_mangle]
pub extern "C" fn zisk_keccakf(data: &mut [u64; 25]) {
    keccakf(data);
}

#[no_mangle]
pub extern "C" fn zisk_sha256(state: &mut [u64; 4], input: &[u64; 8]) {
    sha256f(state, input);
}
