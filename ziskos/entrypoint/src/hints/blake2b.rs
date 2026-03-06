#[no_mangle]
pub unsafe extern "C" fn hint_blake2b_compress(
    _rounds: u32,
    _state: *mut u64,
    _message: *const u64,
    _offset: *const u64,
    _final_block: u8,
) {
}
