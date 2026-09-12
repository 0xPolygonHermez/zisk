//! Hint for the KoalaBear Poseidon2 precompile: the 64-byte input state, no result.

use crate::hints::macros::define_hint;
use zisk_definitions::{HINT_KOALA_POSEIDON2, KOALA_POSEIDON2_RESULTS};

define_hint! {
    koala_poseidon2 => {
        hint_id: HINT_KOALA_POSEIDON2,
        params: (state: 64),
        is_result: false,
        enabled: KOALA_POSEIDON2_RESULTS,
    }
}
