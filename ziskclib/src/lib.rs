use tiny_keccak::keccakf;

mod helpers;
use helpers::sha256f;

#[no_mangle]
pub extern "C" fn zisk_keccakf(data: &mut [u64; 25]) {
    //println!("zisk_keccakf() starting...");
    keccakf(data);
    //println!("zisk_keccakf() ...done");
}

#[no_mangle]
pub extern "C" fn zisk_sha256(state: &mut [u64; 4], input: &[u64; 8]) {
    //println!("zisk_sha256f() starting...");
    sha256f(state, input);
    //println!("zisk_sha256f() ...done");
}
