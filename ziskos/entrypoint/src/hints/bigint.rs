use crate::hints::macros::define_hint_ptr;
use zisk_definitions::HINT_MODEXP;

define_hint_ptr! {
    modexp_bytes => {
        hint_id: HINT_MODEXP,
        params: (base, exp, modulus),
        is_result: false,
    }
}
