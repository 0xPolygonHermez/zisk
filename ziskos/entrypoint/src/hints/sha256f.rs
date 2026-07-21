use crate::hints::macros::define_hint_disabled;
use zisk_definitions::HINT_SHA256;

define_hint_disabled! {
    sha256 => {
        hint_id: HINT_SHA256,
        param: f,
        is_result: false,
    }
}
