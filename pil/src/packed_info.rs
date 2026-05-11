use std::collections::HashMap;

use proofman_common::PackedInfo;

use crate::PACKED_INFO;

/// Materialize the auto-generated [`PACKED_INFO`] slice into a `HashMap`
/// keyed by `(airgroup_id, air_id)` — the form proofman expects.
pub fn get_packed_info() -> HashMap<(usize, usize), PackedInfo> {
    PACKED_INFO
        .iter()
        .map(|p| {
            (
                (p.0, p.1),
                PackedInfo::new(p.2.is_packed, p.2.num_packed_words, p.2.unpack_info.to_vec()),
            )
        })
        .collect()
}
