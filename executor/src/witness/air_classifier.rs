//! AIR classification helpers.

use zisk_pil::{
    INPUT_DATA_AIR_IDS, MAIN_AIR_IDS, MEM_AIR_IDS, ROM_AIR_IDS, ROM_DATA_AIR_IDS, ZISK_AIRGROUP_ID,
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

    /// Checks if the AIR ID corresponds to a memory-related state machine.
    #[inline]
    pub fn is_memory_related(air_id: usize) -> bool {
        air_id == MEM_AIR_IDS[0] || air_id == ROM_DATA_AIR_IDS[0] || air_id == INPUT_DATA_AIR_IDS[0]
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
