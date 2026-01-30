use crate::hints::macros::define_hint_ptr;

const MODEXP_HINT_ID: u32 = 0x0500;

define_hint_ptr! {
    modexp_bytes => {
        hint_id: MODEXP_HINT_ID,
        params: (base, exp, modulus),
        is_result: false,
    }
}
