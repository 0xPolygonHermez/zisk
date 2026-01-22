macro_rules! concat_hint_bytes {
    ($len:expr; $($src:expr),+ $(,)?) => {{
        let mut buf = [0u8; $len];
        let mut offset = 0;
        $(
            let part = $src;
            let part_len = part.len();
            unsafe {
                core::ptr::copy_nonoverlapping(part.as_ptr(), buf.as_mut_ptr().add(offset), part_len);
            }
            offset += part_len;
        )+

        // Avoid unused variable warning
        let _ = offset;

        buf
    }};
}

macro_rules! define_hint {
    (
        $name:ident => {
            hint_id: $hint_id:expr,
            params: ( $( $arg:ident : $len:literal ),+ $(,)? ),
            is_result: $is_result:expr,
        }
    ) => {
        paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<hint_ $name>]($( $arg: *const u8 ),+) {
                $(
                    let $arg: &[u8; $len] = &*($arg as *const [u8; $len]);
                )+

                let slice_bytes = $crate::hints::macros::concat_hint_bytes!(0 $(+ $len)+; $( $arg ),+);

                $crate::hints::hint::hint_slice($hint_id, &slice_bytes, $is_result);
            }

            #[cfg(zisk_hints_metrics)]
            #[ctor::ctor]
            fn [<$name _register_meta>]() {
                $crate::hints::register_hint($hint_id, stringify!($name).to_string());
            }
        }
    };
}

// macros_rules! hint_asserts {
//     ($hint_name:ident

//     )
// }

// macro_rules! param_len {
//     ($field_slice:ident, 0) => {
//         $field_slice.len()
//     };
//     ($field_slice:ident, $len:literal) => {
//         $len
//     };
// }

// macro_rules! param_to_slice {
//     ($field_name:ident, 0) => {
//         unsafe { core::slice::from_raw_parts($field_name as *const u8, 0) }
//     };
//     ($field_name:ident, $len:literal) => {
//         unsafe { &*($field_name as *const [u8; $len]); }
//     };
// }

// macro_rules! define_hint {
//     (
//         variant $variant:ident {
//             $( $field_name:ident : [ $len:literal ] ),+ $(,)?
//         }
//         hint_id $hint_id:expr
//     ) => {
//         paste::paste! {
//             #[no_mangle]
//             pub unsafe extern "C" fn [<hint_ $variant:lower>]($( $field_name: *const u8 ),+) {
//                 if $crate::hints::HINT_QUEUE.is_paused() {
//                     return;
//                 }

//                 $crate::hints::check_main_thread();

//                 $(
//                     let [<$field_name _slice>] = $crate::hints::macros::param_to_slice!($field_name, $len);
//                 )+

//                 #[cfg(zisk_hints_debug)]
//                 println!(
//                     concat!(stringify!($variant), " params: ", $( stringify!($field_name), "={:?}; ", )+),
//                     $( [<$field_name _slice>], )+
//                 );

//                 let mut total_len_bytes: usize = 0;
//                 // Assert that length of each param is aligned with u64
//                 $(
//                     let param_len: usize = $crate::hints::macros::param_len!([<$field_name _slice>], $len);

//                     if param_len % 8 != 0 {
//                         panic!(
//                             "param {}.{} length in bytes must be multiple of 8, current length: {}",
//                             stringify!($variant),
//                             stringify!($field_name),
//                             param_len
//                         );
//                     }

//                     // Accumulate total length
//                     total_len_bytes += param_len;
//                 )+

//                 // Assert that total length of all params does not exceed MAX_HINT_DATA_LEN
//                 if total_len_bytes >= $crate::hints::hint::MAX_HINT_DATA_LEN {
//                     panic!(
//                         "Hint {} total length exceeds MAX_HINT_DATA_LEN, current total length: {}",
//                         stringify!($variant),
//                         total_len_bytes
//                     );
//                 }

//                 let header : u64 = (0x8000000 | ($hint_id as u64) << 32) | (total_len_bytes as u64);
//                 let mut hint = $crate::hints::hint::Hint {
//                     header,
//                     data: [0u8; crate::hints::hint::MAX_HINT_DATA_LEN],
//                     len: 0,
//                 };

//                 unsafe {
//                     let mut offset = 0usize;
//                     let dst = hint.data.as_mut_ptr().cast::<u8>();

//                     $(
//                         let param_len = $crate::hints::macros::param_len!([<$field_name _slice>], $len);
//                         core::ptr::copy_nonoverlapping(
//                             [<$field_name _slice>].as_ptr(),
//                             dst.add(offset),
//                             param_len,
//                         );
//                         offset += param_len;
//                     )+

//                     hint.len = total_len_bytes;

//                     // Prevent unused variable warning
//                     let _ = offset;
//                 }

//                 $crate::hints::HINT_QUEUE.push(hint);
//             }

//             #[cfg(zisk_hints_metrics)]
//             #[ctor::ctor]
//             fn [<$variant:lower _register_meta>]() {
//                 $crate::hints::register_hint($hint_id, stringify!($variant).to_string().to_lowercase() );
//             }
//         }
//     };
// }

// pub(crate) use define_hint;
// pub(crate) use param_len;
// pub(crate) use param_to_slice;
pub(crate) use concat_hint_bytes;
pub(crate) use define_hint;