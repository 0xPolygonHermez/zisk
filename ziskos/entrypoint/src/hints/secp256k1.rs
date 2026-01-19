use crate::hints::macros::define_hint;

// === secp256k1_fn_reduce_c (x) ===

define_hint! {
    variant SECP256K1_FN_REDUCE { x: [u64;4] }
    hint_id 0x02000
}

// === secp256k1_fn_add_c (x, y) ===

define_hint! {
    variant SECP256K1_FN_ADD { x: [u64;4], y: [u64;4] }
    hint_id 0x02001
}

// === secp256k1_fn_neg_c (x) ===

define_hint! {
    variant SECP256K1_FN_NEG { x: [u64; 4] }
    hint_id 0x02002
}

// === secp256k1_fn_sub_c (x, y) ===

define_hint! {
    variant SECP256K1_FN_SUB { x: [u64; 4], y: [u64; 4] }
    hint_id 0x02003
}

// === secp256k1_fn_mul_c (x, y) ===

define_hint! {
    variant SECP256K1_FN_MUL { x: [u64; 4], y: [u64; 4] }
    hint_id 0x02004
}

// === secp256k1_fn_inv_c (x) ===

define_hint! {
    variant SECP256K1_FN_INV { x: [u64; 4] }
    hint_id 0x02005
}

// === secp256k1_to_affine_c (p) ===

define_hint! {
    variant SECP256K1_TO_AFFINE { p: [u64; 12] }
    hint_id 0x02020
}

// === secp256k1_decompress_c (x_bytes, y_is_odd) ===
define_hint! {
    variant SECP256K1_DECOMPRESS { x_bytes: [u8; 32], y_is_odd: [u64; 1] }
    hint_id 0x02021
}

// === secp256k1_double_scalar_mul_with_g_c (k1, k2, p) ===

define_hint! {
    variant SECP256K1_DOUBLE_SCALAR_MUL_WITH_G { k1: [u64; 4], k2: [u64; 4], p: [u64; 8] }
    hint_id 0x02022
}

// === secp256k1_ecdsa_verify_c (pk, z, r, s) ===

define_hint! {
    variant SECP256K1_ECDSA_VERIFY { pk: [u64; 8], z: [u64; 4], r: [u64; 4], s: [u64; 4] }
    hint_id 0x02023
}
