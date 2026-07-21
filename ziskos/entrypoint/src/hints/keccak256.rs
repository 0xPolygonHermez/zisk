use crate::hints::macros::define_hint_disabled;
use zisk_definitions::HINT_KECCAK256;

define_hint_disabled! {
    keccak256 => {
        hint_id: HINT_KECCAK256,
        param: input,
        is_result: false,
    }
}
