use crate::hints::macros::define_hint_ptr;
use zisk_common::HINT_RIPEMD160;

define_hint_ptr! {
    ripemd160 => {
        hint_id: HINT_RIPEMD160,
        param: data,
        is_result: false,
    }
}
