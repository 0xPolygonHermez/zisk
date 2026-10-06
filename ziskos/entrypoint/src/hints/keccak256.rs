use crate::hints::macros::define_hint_ptr;
use zisk_definitions::{HINT_KECCAK256, KECCAK_RESULTS};

define_hint_ptr! {
    keccak256 => {
        hint_id: HINT_KECCAK256,
        param: input,
        is_result: false,
        enabled: KECCAK_RESULTS,
    }
}
