use crate::hints::{HINT_QUEUE, check_main_thread, hint::{Hint, MAX_HINT_DATA_LEN}, macros::register_hint_meta};

const SHA256_HINT_ID: u32 = 0x0100;

#[no_mangle]
pub unsafe extern "C" fn hint_sha256(f: *const u8, len: usize) {
    if HINT_QUEUE.is_paused() {
        return;
    }

    check_main_thread();

    assert!(
        len <= MAX_HINT_DATA_LEN,
        "sha256 hint param length exceeds MAX_HINT_DATA_LEN"
    );

    let f_slice: &[u8] = unsafe { core::slice::from_raw_parts(f, len) };

    // #[cfg(zisk_hints_debug)]
    // {
    //     println!(
    //         concat!(stringify!(SHA2), " params: ", stringify!(f), "={:?}; ",),
    //         f_slice,
    //     );
    // };

    let mut total_len_bytes: usize = 0;
    let param_len: usize = f_slice.len();
    if param_len % 8 != 0 {
        {
            panic!(
                "param {}.{} length in bytes must be multiple of 8, current length: {}",
                stringify!(SHA2),
                stringify!(f),
                param_len
            );
        };
    }

    total_len_bytes += param_len;
    if total_len_bytes >= crate::hints::hint::MAX_HINT_DATA_LEN {
        {
            panic!(
                "Hint {} total length exceeds MAX_HINT_DATA_LEN, current total length: {}",
                stringify!(SHA2),
                total_len_bytes
            );
        };
    }

    HINT_QUEUE.push(
        Hint::new(SHA256_HINT_ID, f_slice, f_slice.len(), false)
    );
}

register_hint_meta!(sha256, SHA256_HINT_ID);