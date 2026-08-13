use std::collections::HashMap;

use proofman_fields::Goldilocks;
use proofman_common::PackedInfo;

use crate::main_indexed::{
    MainTraceRowInstrTable, MainTraceRowPackedIndexed, MAIN_AIRGROUP_ID, MAIN_AIR_ID,
};
use crate::PACKED_INFO;

/// Materialize [`PACKED_INFO`] into the `(airgroup_id, air_id) -> PackedInfo` map proofman
/// expects. Main is emitted compact (indexed) — a smaller `num_packed_words` plus the indexed
/// descriptor so proofman reconstructs it from the shared instruction table; every other air
/// keeps the full packing. Only meaningful for packed traces (the sole caller gates on that).
pub fn get_packed_info() -> HashMap<(usize, usize), PackedInfo> {
    let compact_words = MainTraceRowPackedIndexed::<Goldilocks>::PACKED_WORDS as u64;
    let words_per_entry = MainTraceRowInstrTable::<Goldilocks>::PACKED_WORDS as u64;

    PACKED_INFO
        .iter()
        .map(|p| {
            let c = &p.2;
            let is_main = p.0 == MAIN_AIRGROUP_ID && p.1 == MAIN_AIR_ID;
            let info = if is_main {
                PackedInfo::new(c.is_packed, compact_words, c.unpack_info.to_vec()).with_indexed(
                    MainTraceRowPackedIndexed::<Goldilocks>::COL_SOURCE.to_vec(),
                    MainTraceRowPackedIndexed::<Goldilocks>::INDEX_BITS,
                    words_per_entry,
                )
            } else {
                PackedInfo::new(c.is_packed, c.num_packed_words, c.unpack_info.to_vec())
            };
            ((p.0, p.1), info)
        })
        .collect()
}
