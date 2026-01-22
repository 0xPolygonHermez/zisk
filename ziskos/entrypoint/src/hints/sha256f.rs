use crate::hints::{HINT_QUEUE, check_main_thread, hint::{Hint, MAX_HINT_DATA_LEN}, macros::register_hint_meta};

const SHA256_HINT_ID: u32 = 0x0100;

#[no_mangle]
pub unsafe extern "C" fn hint_sha256(f: *const u8, len: usize) {
    if HINT_QUEUE.is_paused() {
        return;
    }

    check_main_thread();

    assert!(
        len as usize <= MAX_HINT_DATA_LEN,
        "sha256 hint data length exceeds MAX_HINT_DATA_LEN"
    );

    let f_slice: &[u8] = unsafe { core::slice::from_raw_parts(f, len) };


    HINT_QUEUE.push(
        Hint::new(SHA256_HINT_ID, f_slice, f_slice.len(), false)
    );
}

register_hint_meta!(sha256, SHA256_HINT_ID);