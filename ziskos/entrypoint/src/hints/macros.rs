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
                if !$crate::hints::HINT_BUFFER.is_enabled() {
                    return;
                }

                #[cfg(zisk_hints_single_thread)]
                if !$crate::hints::check_main_thread() { return; }

                let mut total_len = 0usize;
                $(
                    total_len += $len;
                )+

                let mut w = $crate::hints::HINT_BUFFER.begin_hint(
                    $hint_id,
                    total_len,
                    $is_result,
                );

                $(
                    w.write_hint_data_ptr($arg, $len);
                )+

                w.commit();
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
            pub unsafe extern "C" fn [<hint_ $name>](pairs: *const u8, num_pairs: usize) {
                if !$crate::hints::HINT_BUFFER.is_enabled() {
                    return;
                }

                #[cfg(zisk_hints_single_thread)]
                if !$crate::hints::check_main_thread() { return; }

                let total_len = 8 + (num_pairs * ($pair_len as usize));

                let mut w = $crate::hints::HINT_BUFFER.begin_hint(
                    $hint_id,
                    total_len,
                    $is_result,
                );

                let num_pairs_bytes: [u8; 8] = (num_pairs as u64).to_le_bytes();
                w.write_hint_data_slice(&num_pairs_bytes);

                w.write_hint_data_ptr(pairs, num_pairs * ($pair_len as usize));

                w.commit();
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
                if !$crate::hints::HINT_BUFFER.is_enabled() {
                    return;
                }

                #[cfg(zisk_hints_single_thread)]
                if !$crate::hints::check_main_thread() { return; }

                let pad = (8 - ([<$arg _len>] & 7)) & 7;

                let mut w = $crate::hints::HINT_BUFFER.begin_hint(
                    $hint_id,
                    [<$arg _len>],
                    $is_result,
                );

                w.write_hint_data_ptr([<$arg _ptr>], [<$arg _len>]);

                if pad > 0 {
                    const ZERO_PAD: [u8; 8] = [0; 8];
                    w.write_hint_data_slice(&ZERO_PAD[..pad]);
                }

                w.commit();
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
            pub unsafe extern "C" fn [<hint_ $name>](
                $( [<$arg _ptr>]: *const u8, [<$arg _len>]: usize ),+
            ) {
                if !$crate::hints::HINT_BUFFER.is_enabled() {
                    return;
                }

                #[cfg(zisk_hints_single_thread)]
                if !$crate::hints::check_main_thread() { return; }

                let mut total_len = 0usize;
                $(
                    total_len += 8 + [<$arg _len>];
                )+

                let pad = (8 - (total_len & 7)) & 7;

                let mut w = $crate::hints::HINT_BUFFER.begin_hint(
                    $hint_id,
                    total_len,
                    $is_result,
                );

                $(
                    {
                        let len_bytes: [u8; 8] = ([<$arg _len>] as u64).to_le_bytes();
                        w.write_hint_data_slice(&len_bytes);

                        w.write_hint_data_ptr([<$arg _ptr>], [<$arg _len>]);
                    }
                )+

                if pad > 0 {
                    const ZERO_PAD: [u8; 8] = [0; 8];
                    w.write_hint_data_slice(&ZERO_PAD[..pad]);
                }

                w.commit();
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
