use crate::hints::macros::define_hint_ptr;

const SHA256_HINT_ID: u32 = 0x0100;

define_hint_ptr! {
    sha256 => {
        hint_id: SHA256_HINT_ID,
        param: f,
        is_result: false,
    }
}
