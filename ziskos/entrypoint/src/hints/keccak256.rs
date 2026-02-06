use crate::hints::macros::define_hint_ptr;
use zisk_common::HINT_KECCAK256;

define_hint_ptr! {
    keccak256 => {
        hint_id: HINT_KECCAK256,
        param: input,
        is_result: false,
    }
}
