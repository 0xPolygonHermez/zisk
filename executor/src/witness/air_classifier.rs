//! AIR classification helpers.

use zisk_pil::{
    AIR_NAMES, INPUT_DATA_AIR_IDS, MAIN_AIR_IDS, MEM_AIR_IDS, ROM_AIR_IDS, ROM_DATA_AIR_IDS,
    ZISK_AIRGROUP_ID,
};

use crate::{PRECOMPILE_AIR_IDS, PRECOMPILE_RANK_ASSIGN};

/// Helper for classifying AIR instances by their ID.
pub struct AirClassifier;

impl AirClassifier {
    /// Checks if the AIR ID corresponds to a main state machine.
    #[inline]
    pub fn is_main(air_id: usize) -> bool {
        MAIN_AIR_IDS.contains(&air_id)
    }

    /// Checks if the AIR ID corresponds to the ROM state machine.
    #[inline]
    pub fn is_rom(airgroup_id: usize, air_id: usize) -> bool {
        airgroup_id == ZISK_AIRGROUP_ID && air_id == ROM_AIR_IDS[0]
    }

    /// Checks if `air_id` belongs to a precompile registered with `rank_assign: true`.
    #[inline]
    pub fn is_rank_assigned_precompile(airgroup_id: usize, air_id: usize) -> bool {
        airgroup_id == ZISK_AIRGROUP_ID
            && PRECOMPILE_AIR_IDS
                .iter()
                .zip(PRECOMPILE_RANK_ASSIGN.iter())
                .any(|(&id, &assigned)| id == air_id && assigned)
    }

    /// `ZISK_WITNESS_ONLY` restricts the witness computation for benchmarking: `ram` computes only
    /// the Mem instances, `mem` the three memory AIRs, anything else everything.
    /// `ZISK_WITNESS_SKIP` is a comma-separated list of AIR names left out.
    pub fn witness_only_selected(air_id: usize) -> bool {
        static MODE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
        static SKIP: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
        let skip = SKIP.get_or_init(|| {
            std::env::var("ZISK_WITNESS_SKIP")
                .map(|v| v.split(',').map(|n| n.trim().to_lowercase()).collect())
                .unwrap_or_default()
        });
        if !skip.is_empty() {
            let name = AIR_NAMES
                .iter()
                .find(|(ag, a, _)| *ag == ZISK_AIRGROUP_ID && *a == air_id)
                .map(|(_, _, n)| n.to_lowercase());
            if name.is_some_and(|n| skip.iter().any(|s| *s == n)) {
                return false;
            }
        }
        match MODE.get_or_init(|| std::env::var("ZISK_WITNESS_ONLY").ok()).as_deref() {
            Some("ram") => air_id == MEM_AIR_IDS[0],
            Some("mem") => Self::is_memory_related(air_id),
            _ => true,
        }
    }

    /// The Mem instances need no collection from the replay when this block's RAM rows come from
    /// the GPU planner (`ZISK_MEM_GPU_FILL=arena` and the device fill succeeded).
    pub fn mem_collected_on_device(air_id: usize) -> bool {
        air_id == MEM_AIR_IDS[0]
            && zisk_common::MEM_RAM_ROWS_ON_DEVICE.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Checks if the AIR ID corresponds to a memory-related state machine.
    #[inline]
    pub fn is_memory_related(air_id: usize) -> bool {
        air_id == MEM_AIR_IDS[0] || air_id == ROM_DATA_AIR_IDS[0] || air_id == INPUT_DATA_AIR_IDS[0]
    }

    /// Display name for a known `(airgroup_id, air_id)` pair. Returns
    /// `"Unknown"` for unrecognised pairs. Backed by the PILOUT-generated
    /// [`AIR_NAMES`] table, so new AIRs/precompiles need no change here and
    /// no proofman/setup lookup is required.
    pub fn name(airgroup_id: usize, air_id: usize) -> &'static str {
        AIR_NAMES
            .iter()
            .find(|&&(g, a, _)| g == airgroup_id && a == air_id)
            .map_or("Unknown", |&(_, _, name)| name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_pil::KECCAKF_AIR_IDS;

    #[test]
    fn test_is_main() {
        for &air_id in MAIN_AIR_IDS {
            assert!(AirClassifier::is_main(air_id));
        }
    }

    #[test]
    fn test_is_rom() {
        for &air_id in ROM_AIR_IDS {
            assert!(AirClassifier::is_rom(ZISK_AIRGROUP_ID, air_id));
        }
    }

    #[test]
    fn test_is_memory_related() {
        assert!(AirClassifier::is_memory_related(MEM_AIR_IDS[0]));
        assert!(AirClassifier::is_memory_related(ROM_DATA_AIR_IDS[0]));
        assert!(AirClassifier::is_memory_related(INPUT_DATA_AIR_IDS[0]));
    }

    #[test]
    fn keccakf_is_rank_assigned_precompile() {
        assert!(AirClassifier::is_rank_assigned_precompile(ZISK_AIRGROUP_ID, KECCAKF_AIR_IDS[0]));
    }

    #[test]
    fn rom_is_not_rank_assigned_precompile() {
        // ROM is rank-owned via a different code path; the precompile slice
        // only covers entries from `register_precompiles!`.
        assert!(!AirClassifier::is_rank_assigned_precompile(ZISK_AIRGROUP_ID, ROM_AIR_IDS[0]));
    }

    #[test]
    fn rank_assigned_check_requires_zisk_airgroup() {
        assert!(AirClassifier::is_rank_assigned_precompile(ZISK_AIRGROUP_ID, KECCAKF_AIR_IDS[0]));
        assert!(!AirClassifier::is_rank_assigned_precompile(
            ZISK_AIRGROUP_ID + 1,
            KECCAKF_AIR_IDS[0]
        ));
    }
}
