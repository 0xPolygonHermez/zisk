use crate::hints::macros::define_hint;

define_hint! {
    variant MUL_FP12_BLS12_381 { a: [u64;72], b: [u64;72] }
    hint_id 0x16
}

define_hint! {
    variant DECOMPRESS_BLS12_381 { input: [u8;48] }
    hint_id 0x17
}

define_hint! {
    variant IS_ON_CURVE_BLS12_381 { p: [u64;12] }
    hint_id 0x18
}

define_hint! {
    variant IS_ON_SUBGROUP_BLS12_381 { p: [u64;12] }
    hint_id 0x19
}

define_hint! {
    variant ADD_BLS12_381 { p1: [u64;12], p2: [u64;12] }
    hint_id 0x1A
}

define_hint! {
    variant SCALAR_MUL_BLS12_381 { p: [u64;12], k: [u64;6] }
    hint_id 0x1B
}

define_hint! {
    variant DECOMPRESS_TWIST_BLS12_381 { input: [u8;96] }
    hint_id 0x1C
}

define_hint! {
    variant IS_ON_CURVE_TWIST_BLS12_381 { p: [u64;24] }
    hint_id 0x1D
}

define_hint! {
    variant IS_ON_SUBGROUP_TWIST_BLS12_381 { p: [u64;24] }
    hint_id 0x1E
}

define_hint! {
    variant ADD_TWIST_BLS12_381 { p1: [u64;24], p2: [u64;24] }
    hint_id 0x1F
}

define_hint! {
    variant SCALAR_MUL_TWIST_BLS12_381 { p: [u64;24], k: [u64;6] }
    hint_id 0x20
}

define_hint! {
    variant MILLER_LOOP_BLS12_381 { p: [u64;12], q: [u64;24] }
    hint_id 0x21
}

define_hint! {
    variant FINAL_EXP_BLS12_381 { f: [u64;72] }
    hint_id 0x22
}
