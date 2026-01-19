use crate::hints::macros::define_hint;

// === redmod256 (a, m) ===

define_hint! {
    variant REDMOD256 { a: [u64; 4], m: [u64; 4] }
    hint_id 6
}

// === addmod256 (a, b, m) ===

define_hint! {
    variant ADDMOD256 { a: [u64; 4], b: [u64; 4], m: [u64; 4] }
    hint_id 7
}

// === mulmod256 (a, b, m) ===

define_hint! {
    variant MULMOD256 { a: [u64; 4], b: [u64; 4], m: [u64; 4] }
    hint_id 8
}

// === divrem256 (a, b) ===

define_hint! {
    variant DIVREM256 { a: [u64; 4], b: [u64; 4] }
    hint_id 9
}

// === wpow256 (a, exp) ===

define_hint! {
    variant WPOW256 { a: [u64; 4], exp: [u64; 4] }
    hint_id 10
}

// === omul256 (a, b) ===

define_hint! {
    variant OMUL256 { a: [u64; 4], b: [u64; 4] }
    hint_id 11
}

// === wmul256 (a, b) ===

define_hint! {
    variant WMUL256 { a: [u64; 4], b: [u64; 4] }
    hint_id 12
}
