use crate::hints::macros::define_hint_ptr;

const KECCAK256_HINT_ID: u32 = 0x0700;

define_hint_ptr! {
    keccak256 => {
        hint_id: KECCAK256_HINT_ID,
        param: input,
        is_result: false,
    }
}
