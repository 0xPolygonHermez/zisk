macro_rules! define_hint {
    (
        variant $variant:ident {
            $( $field_name:ident : [ $type:ty ; $len:literal ] ),+ $(,)?
        }
        hint_id $hint_id:expr
    ) => {
        paste::paste! {
            pub const [<HINT_ $variant>]: u32 = $hint_id;
            pub const [<$variant:upper _BYTES>]: usize = 0 $(+ core::mem::size_of::<[$type; $len]>())+;
            pub const [<$variant:upper _LEN_U64>]: usize = ([<$variant:upper _BYTES>]) / 8;
            pub const [<HEADER_ $variant:upper>]: u64 =
                ((([<HINT_ $variant>] as u64) << 32) | [<$variant:upper _LEN_U64>] as u64);

            // Assert that length of each field is aligned with u64
            $(
                const _: () = {
                    let t = [0 as $type; $len];
                    if core::mem::size_of_val(&t) % 8 != 0 {
                        panic!(concat!(
                            "Field ",
                            stringify!($variant),
                            ".",
                            stringify!($field_name),
                            " length in bytes must be multiple of 8"
                        ));
                    }
                };
            )+

            // Assert that total length of all fields does not exceed MAX_SLICE_U64_LEN
            const _: () = {
                if [<$variant:upper _BYTES>] > ($crate::hints::hint::MAX_SLICE_U64_LEN * core::mem::size_of::<u64>()) {
                    panic!(concat!(
                        "Hint ",
                        stringify!($variant),
                        " total length exceeds MAX_SLICE_U64_LEN"
                    ));
                }
            };

            #[inline(always)]
            pub fn [<hint_ $variant:lower>]($( $field_name: &[$type; $len] ),+) {
                if $crate::hints::HINT_QUEUE.is_paused() {
                    return;
                }

                $crate::hints::check_main_thread();

                #[cfg(zisk_hints_debug)]
                println!(
                    concat!(
                        stringify!($variant),
                        " params: ",
                        $( stringify!($field_name), "={:?}; ", )+
                    ),
                    $( $field_name, )+
                );

                let mut hint = $crate::hints::hint::HintSliceU64 {
                    header: [<HEADER_ $variant:upper>],
                    data: [0u64; crate::hints::hint::MAX_SLICE_U64_LEN],
                    len: 0,
                };

                unsafe {
                    let mut offset = 0usize;
                    let dst = hint.data.as_mut_ptr().cast::<u8>();

                    $(
                        core::ptr::copy_nonoverlapping(
                            $field_name.as_ptr().cast::<u8>(),
                            dst.add(offset),
                            core::mem::size_of::<[$type; $len]>(),
                        );
                        offset += core::mem::size_of::<[$type; $len]>();
                    )+

                    hint.len = [<$variant:upper _LEN_U64>];

                    // Prevent unused variable warning
                    let _ = offset;
                }

                $crate::hints::HINT_QUEUE.push($crate::hints::hint::Hint::HintSliceU64(hint));
            }

            #[cfg(zisk_hints_metrics)]
            #[ctor::ctor]
            fn [<$variant:lower _register_meta>]() {
                $crate::hints::register_hint([<HINT_ $variant>], stringify!($variant).to_string().to_lowercase() );
            }
        }
    };
}

pub(crate) use define_hint;