use crate::hints::{HINT_QUEUE, check_main_thread, hint::{Hint, MAX_HINT_DATA_LEN}, macros::register_hint_meta};

const KECCAK256_HINT_ID: u32 = 0x0700;

pub unsafe extern "C" fn hint_keccak256(input: *const u8, input_len: usize) {
    if HINT_QUEUE.is_paused() {
        return;
    }

    check_main_thread();

    assert!(
        input_len as usize <= MAX_HINT_DATA_LEN,
        "keccak256 hint data length exceeds MAX_HINT_DATA_LEN"
    );

    let input_slice: &[u8] = unsafe { core::slice::from_raw_parts(input, input_len) };

    HINT_QUEUE.push(
        Hint::new(KECCAK256_HINT_ID, input_slice, input_slice.len(), false)
    );
}

register_hint_meta!(keccak256, KECCAK256_HINT_ID);