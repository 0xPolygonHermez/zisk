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
                if !crate::hints::HINT_BUFFER.is_enabled() {
                    return;
                }

                #[cfg(zisk_hints_single_thread)]
                crate::hints::check_main_thread();

                let mut total_len = 0;
                $(
                    total_len += $len;
                )+

                crate::hints::HINT_BUFFER.write_hint_header(
                    $hint_id,
                    total_len,
                    $is_result,
                );

                $(
                    $crate::hints::HINT_BUFFER.write_hint_data($arg, $len);
                )+
            }

            $crate::hints::macros::register_hint_meta!($name, $hint_id);
        }
    };
}

macro_rules! define_hint_pairs {
    (
        $name:ident => {
            hint_id: $hint_id:expr,
            pair_len: $pair_len:expr,
            is_result: $is_result:expr,
        }
    ) => {
        paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<hint_ $name>]( pairs: *const u8, num_pairs: usize) {
                if !crate::hints::HINT_BUFFER.is_enabled() {
                    return;
                }

                #[cfg(zisk_hints_single_thread)]
                crate::hints::check_main_thread();

                crate::hints::HINT_BUFFER.write_hint_header(
                    $hint_id,
                    8 + (num_pairs * $pair_len),
                    false,
                );

                let num_pairs_bytes: [u8; 8] = (num_pairs as u64).to_le_bytes();
                crate::hints::HINT_BUFFER.write_hint_data(num_pairs_bytes.as_ptr(), num_pairs_bytes.len());

                crate::hints::HINT_BUFFER.write_hint_data(pairs, num_pairs * $pair_len);
            }

            $crate::hints::macros::register_hint_meta!($name, $hint_id);
        }
    };
}

macro_rules! define_hint_ptr {
    (
        $name:ident => {
            hint_id: $hint_id:expr,
            param: $arg:ident,
            is_result: $is_result:expr,
        }
    ) => {
        paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<hint_ $name>]([<$arg _ptr>]: *const u8, [<$arg _len>]: usize) {
                if !crate::hints::HINT_BUFFER.is_enabled() {
                    return;
                }

                #[cfg(zisk_hints_single_thread)]
                crate::hints::check_main_thread();

                let pad = (8 - ([<$arg _len>] & 7)) & 7;

                crate::hints::HINT_BUFFER.write_hint_header(
                    $hint_id,
                    [<$arg _len>],
                    $is_result,
                );

                crate::hints::HINT_BUFFER.write_hint_data([<$arg _ptr>], [<$arg _len>]);
                if pad > 0 {
                    const ZERO_PAD: [u8; 8] = [0; 8];
                    crate::hints::HINT_BUFFER.write_hint_data(ZERO_PAD.as_ptr(), pad);
                }
            }

            $crate::hints::macros::register_hint_meta!($name, $hint_id);
        }
    };
    (
        $name:ident => {
            hint_id: $hint_id:expr,
            params: ( $( $arg:ident ),+ $(,)? ),
            is_result: $is_result:expr,
        }
    ) => {
        paste::paste! {
            #[no_mangle]
            pub unsafe extern "C" fn [<hint_ $name>]($( [<$arg _ptr>]: *const u8, [<$arg _len>]: usize ),+
            ) {
                if !crate::hints::HINT_BUFFER.is_enabled() {
                    return;
                }

                #[cfg(zisk_hints_single_thread)]
                crate::hints::check_main_thread();

                let mut total_len = 0;
                $(
                    total_len += 8 + [<$arg _len>];
                )+

                let pad = (8 - (total_len & 7)) & 7;

                crate::hints::HINT_BUFFER.write_hint_header(
                    $hint_id,
                    total_len,
                    $is_result,
                );

                $(
                    {
                        let len_bytes: [u8; 8] = ([<$arg _len>] as u64).to_le_bytes();
                        crate::hints::HINT_BUFFER.write_hint_data(len_bytes.as_ptr(), len_bytes.len());
                        crate::hints::HINT_BUFFER.write_hint_data([<$arg _ptr>], [<$arg _len>]);
                    }
                )+

                if pad > 0 {
                    const ZERO_PAD: [u8; 8] = [0; 8];
                    crate::hints::HINT_BUFFER.write_hint_data(ZERO_PAD.as_ptr(), pad);
                }
            }

            $crate::hints::macros::register_hint_meta!($name, $hint_id);
        }
    };
}

macro_rules! register_hint_meta {
    ($name:ident, $hint_id:expr) => {
        paste::paste! {
            #[cfg(zisk_hints_metrics)]
            #[ctor::ctor]
            fn [<$name _register_meta>]() {
                $crate::hints::metrics::register_hint($hint_id, stringify!($name).to_string());
            }
        }
    };
}

pub(crate) use define_hint;
pub(crate) use define_hint_pairs;
pub(crate) use define_hint_ptr;
pub(crate) use register_hint_meta;
