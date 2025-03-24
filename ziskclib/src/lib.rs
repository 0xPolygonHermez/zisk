use tiny_keccak::keccakf;

#[no_mangle]
pub extern "C" fn zisk_keccakf(data: &mut [u64; 25]) {
    //println!("zisk_keccakf() starting...");
    keccakf(data);
    //println!("zisk_keccakf() ...done");
}
