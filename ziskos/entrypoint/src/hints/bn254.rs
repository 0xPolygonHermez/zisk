use crate::hints::{HINT_QUEUE, check_main_thread, hint::Hint, types::{HINTS_TYPE_ADD_BN254, HINTS_TYPE_IS_ON_CURVE_BN254, HINTS_TYPE_IS_ON_CURVE_TWIST_BN254, HINTS_TYPE_IS_ON_SUBGROUP_TWIST_BN254, HINTS_TYPE_MUL_BN254, HINTS_TYPE_PAIRING_BATCH_BN254, HINTS_TYPE_TO_AFFINE_BN254, HINTS_TYPE_TO_AFFINE_TWIST_BN254, HintData}};

macro_rules! define_bn254_hint {
    (
        variant $variant:ident {
            $( $field_name:ident : $len:literal ),+ $(,)?
        }
        hint(
            fn $hint_fn:ident,
            ty = $hint_type_const:ident
        );
    ) => {
        paste::paste! {
            #[repr(C, align(8))]
            #[derive(Clone, Debug, Eq, PartialEq)]
            pub struct $variant {
                $( pub $field_name: [u64; $len], )+
            }

            impl $variant {
                pub fn new($( $field_name: [u64; $len] ),+) -> Self {
                    Self { $( $field_name ),+ }
                }
            }

            impl Default for $variant {
                fn default() -> Self {
                    Self {
                        $( $field_name: [0u64; $len], )+
                    }
                }
            }

            pub const [<$variant:upper _BYTES>]: usize = core::mem::size_of::<$variant>();
            pub const [<$variant:upper _LEN_U64>]: u64 = ([<$variant:upper _BYTES>] as u64) / 8;
            pub const [<HEADER_ $variant:upper>]: [u8; 8] =
                ((( $hint_type_const as u64) << 32) | [<$variant:upper _LEN_U64>]).to_le_bytes();

            impl HintData for $variant {
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
            pub fn $hint_fn($( $field_name: &[u64; $len] ),+) {
                check_main_thread();

                let hint = Hint::$variant($variant::new($( *$field_name ),+));
                HINT_QUEUE.push(hint);
            }
        }
    };
}

// === is_on_curve_bn254 (p) ===

define_bn254_hint! {
    variant IsOnCurveBN254 { p: 8 }
    hint(
        fn hint_is_on_curve_bn254,
        ty = HINTS_TYPE_IS_ON_CURVE_BN254
    );
}

// === to_affine_bn254 (p) ===

define_bn254_hint! {
    variant ToAffineBN254 { p: 12 }
    hint(
        fn hint_to_affine_bn254,
        ty = HINTS_TYPE_TO_AFFINE_BN254
    );
}

// === add_bn254 (p1, p2) ===

define_bn254_hint! {
    variant AddBN254 { p1: 8, p2: 8 }
    hint(
        fn hint_add_bn254,
        ty = HINTS_TYPE_ADD_BN254
    );
}

// === mul_bn254 (p, k) ===

define_bn254_hint! {
    variant MulBN254 { p: 8, k: 4 }
    hint(
        fn hint_mul_bn254,
        ty = HINTS_TYPE_MUL_BN254
    );
}

// === to_affine_twist_bn254 (p) ===

define_bn254_hint! {
    variant ToAffineTwistBN254 { p: 24 }
    hint(
        fn hint_to_affine_twist_bn254,
        ty = HINTS_TYPE_TO_AFFINE_TWIST_BN254
    );
}

// === is_on_curve_twist_bn254 (p) ===

define_bn254_hint! {
    variant IsOnCurveTwistBN254 { p: 16 }
    hint(
        fn hint_is_on_curve_twist_bn254,
        ty = HINTS_TYPE_IS_ON_CURVE_TWIST_BN254
    );
}

// === is_on_subgroup_twist_bn254 (p) ===

define_bn254_hint! {
    variant IsOnSubgroupTwistBN254 { p: 16 }
    hint(
        fn hint_is_on_subgroup_twist_bn254,
        ty = HINTS_TYPE_IS_ON_SUBGROUP_TWIST_BN254
    );
}

// === pairing_batch_bn254 (g1_points, g2_points, num_points) ===

#[repr(C, align(8))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PairingBatchBN254 {
    pub payload: Vec<u64>, // format: [num_points:u64][g1_points:&[u64;8]][g2_points:&[u64;16]]
}

impl PairingBatchBN254 {
    pub fn new(g1_points: Vec<[u64;8]>, g2_points: Vec<[u64; 16]>) -> Self {
        assert_eq!(g1_points.len(), g2_points.len(), "g1 and g2 batches must have the same length");
        let num_points = g1_points.len();
        let mut payload = Vec::with_capacity(1 + num_points * 8 + num_points * 16);

        // Store the batch size first so the consumer knows how many points to read
        payload.push(num_points as u64);

        // Flatten the G1 points (each 8 limbs) into the payload
        for point in &g1_points {
            payload.extend_from_slice(point);
        }

        // Flatten the G2 points (each 16 limbs) into the payload
        for point in &g2_points {
            payload.extend_from_slice(point);
        }

        Self { payload }
    }
}

impl Default for PairingBatchBN254 {
    fn default() -> Self {
        Self {
            payload: Vec::new(),
        }
    }
}

impl HintData for PairingBatchBN254 {
    #[inline(always)]
    fn header_and_payload(&self) -> ([u8; 8], &[u8]) {
        let header: [u8; 8] =
            (((HINTS_TYPE_PAIRING_BATCH_BN254 as u64) << 32) | self.payload.len() as u64).to_le_bytes();

        // Convert payload to bytes
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self.payload.as_ptr() as *const u8,
                self.payload.len() * core::mem::size_of::<u64>(),
            )
        };

        (header, bytes)
    }
}

#[inline(always)]
pub fn hint_pairing_batch_bn254(g1_points: Vec<[u64; 8]>, g2_points: Vec<[u64; 16]>) {
    check_main_thread();

    let hint = Hint::PairingBatchBN254(PairingBatchBN254::new(g1_points, g2_points));
    HINT_QUEUE.push(hint);
}
