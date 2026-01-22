// use crate::hints::{HINT_QUEUE, check_main_thread, hint::hint_slice};

// #[no_mangle]
// pub unsafe extern "C" fn hint_sha256(f: *const u8, len: usize) {
//     if HINT_QUEUE.is_paused() {
//         return;
//     }

//     check_main_thread();

//     let f_slice = unsafe { core::slice::from_raw_parts(f as *const u8, len) };

//     // #[cfg(zisk_hints_debug)]
//     // {
//     //     println!(
//     //         concat!(stringify!(SHA2), " params: ", stringify!(f), "={:?}; ",),
//     //         f_slice,
//     //     );
//     // };

//     let mut total_len_bytes: usize = 0;
//     let param_len: usize = f_slice.len();
//     if param_len % 8 != 0 {
//         {
//             panic!(
//                 "param {}.{} length in bytes must be multiple of 8, current length: {}",
//                 stringify!(SHA2),
//                 stringify!(f),
//                 param_len
//             );
//         };
//     }

//     total_len_bytes += param_len;
//     if total_len_bytes >= crate::hints::hint::MAX_HINT_DATA_LEN {
//         {
//             panic!(
//                 "Hint {} total length exceeds MAX_HINT_DATA_LEN, current total length: {}",
//                 stringify!(SHA2),
//                 total_len_bytes
//             );
//         };
//     }

//     hint_slice(0x0100, f_slice, true);
// }

// #[cfg(zisk_hints_metrics)]
// #[ctor::ctor]
// fn sha2_register_meta() {
//     crate::hints::register_hint(0x0100, stringify!(sha2).to_string());
// }

crate::hints::macros::define_hint! {
    sha256 => {
        hint_id: 0x0100,
        params: (output: 32),
        is_result: true,
    }
}