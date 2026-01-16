macro_rules! define_hint {
    (
        variant $variant:ident {
            $( $field_name:ident : $type:ty ; $len:literal ),+ $(,)?
        }
        hint(
            fn $hint_fn:ident,
            ty = $hint_type_const:ident
        );
    ) => {
        paste::paste! {
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

            #[repr(C, align(8))]
            #[derive(Clone, Debug, Eq, PartialEq)]
            pub struct $variant {
                $( pub $field_name: [$type; $len], )+
            }

            impl $variant {
                pub fn new($( $field_name: [$type; $len] ),+) -> Self {
                    Self { $( $field_name ),+ }
                }
            }

            impl Default for $variant {
                fn default() -> Self {
                    Self {
                        $( $field_name: [0 as $type; $len], )+
                    }
                }
            }

            pub const [<$variant:upper _BYTES>]: usize = core::mem::size_of::<$variant>();
            pub const [<$variant:upper _LEN_U64>]: u64 = ([<$variant:upper _BYTES>] as u64) / 8;
            pub const [<HEADER_ $variant:upper>]: [u8; 8] =
                ((( $hint_type_const as u64) << 32) | [<$variant:upper _LEN_U64>]).to_le_bytes();

            impl $crate::hints::types::HintData for $variant {
                #[inline(always)]
                fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
                    let bytes = unsafe {
                        core::slice::from_raw_parts(
                            (self as *const $variant).cast::<u8>(),
                            [<$variant:upper _BYTES>],
                        )
                    };

                    ([<HEADER_ $variant:upper>], bytes)
                }
            }

            #[inline(always)]
            pub fn $hint_fn($( $field_name: &[$type; $len] ),+) {
                $crate::hints::check_main_thread();

                #[cfg(feature = "hints-debug")]
                println!(
                    concat!(
                        stringify!($variant),
                        " params: ",
                        $( stringify!($field_name), "={:?}; ", )+
                    ),
                    $( $field_name, )+
                );

                let hint = $crate::hints::hint::Hint::$variant($variant::new($( *$field_name ),+));
                $crate::hints::HINT_QUEUE.push(hint);
            }
        }
    };
}

pub(crate) use define_hint;