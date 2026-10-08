use std::collections::HashMap;

use proofman_common::PackedInfo;
use proofman_fields::Goldilocks;

use crate::{
    MainTraceRowInstrTable, MainTraceRowPackedIndexed, MAIN_AIRGROUP_ID, MAIN_AIR_ID, PACKED_INFO,
};

/// Materialize [`PACKED_INFO`] into the `(airgroup_id, air_id) -> PackedInfo` map proofman
/// expects. Main is emitted compact (indexed): fewer packed words plus the descriptor proofman
/// reconstructs it with. Every other air keeps the full packing.
pub fn get_packed_info() -> HashMap<(usize, usize), PackedInfo> {
    type Ix = MainTraceRowPackedIndexed<Goldilocks>;
    let compact_words = Ix::PACKED_WORDS as u64;
    let words_per_entry = MainTraceRowInstrTable::<Goldilocks>::PACKED_WORDS as u64;

    PACKED_INFO
        .iter()
        .map(|p| {
            let c = &p.2;
            let is_main = p.0 == MAIN_AIRGROUP_ID && p.1 == MAIN_AIR_ID;
            let info = if is_main {
                PackedInfo::new(c.is_packed, compact_words, c.unpack_info.to_vec()).with_indexed(
                    Ix::COL_SOURCE.to_vec(),
                    Ix::COL_LANE.to_vec(),
                    Ix::INDEX_BITS,
                    words_per_entry,
                    Ix::LANES as u64,
                )
            } else {
                PackedInfo::new(c.is_packed, c.num_packed_words, c.unpack_info.to_vec())
            };
            ((p.0, p.1), info)
        })
        .collect()
}
