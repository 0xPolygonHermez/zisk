use crate::hints::macros::define_hint_ptr;
use zisk_common::HINT_SHA256;

define_hint_ptr! {
    sha256 => {
        hint_id: HINT_SHA256,
        param: f,
        is_result: false,
    }
}
