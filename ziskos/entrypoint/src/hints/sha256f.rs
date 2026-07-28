use crate::hints::macros::define_hint_ptr;
use zisk_definitions::{HINT_SHA256, SHA256_RESULTS};

define_hint_ptr! {
    sha256 => {
        hint_id: HINT_SHA256,
        param: f,
        is_result: false,
        enabled: SHA256_RESULTS,
    }
}
