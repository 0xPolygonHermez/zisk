use std::collections::HashMap;

use proofman_common::PackedInfo;

use crate::PACKED_INFO;

/// Materialize [`PACKED_INFO`] into the `(airgroup_id, air_id) -> PackedInfo` map proofman
/// expects. Every air keeps the full packing; Main is no longer emitted in the compact
/// indexed form — see the module docs of `main_row` for why lanes rule it out. Only
/// meaningful for packed traces (the sole caller gates on that).
pub fn get_packed_info() -> HashMap<(usize, usize), PackedInfo> {
    PACKED_INFO
        .iter()
        .map(|p| {
            let c = &p.2;
            ((p.0, p.1), PackedInfo::new(c.is_packed, c.num_packed_words, c.unpack_info.to_vec()))
        })
        .collect()
}
