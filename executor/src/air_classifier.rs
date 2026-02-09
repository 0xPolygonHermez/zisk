//! AIR classification helpers.
//!
//! This module provides helpers for classifying AIR types based on their IDs,
//! centralizing the scattered `*_AIR_IDS.contains()` checks throughout the executor.

use zisk_pil::{
    INPUT_DATA_AIR_IDS, MAIN_AIR_IDS, MEM_AIR_IDS, ROM_AIR_IDS, ROM_DATA_AIR_IDS, ZISK_AIRGROUP_ID,
};

/// Helper for classifying AIR instances by their ID.
///
/// Centralizes the logic for determining AIR types, replacing scattered
/// `*_AIR_IDS.contains()` checks throughout the codebase.
pub struct AirClassifier;

impl AirClassifier {
    /// Checks if the AIR ID corresponds to a main state machine.
    #[inline]
    pub fn is_main(air_id: usize) -> bool {
        MAIN_AIR_IDS.contains(&air_id)
    }

    /// Checks if the AIR ID corresponds to the ROM state machine.
    #[inline]
    pub fn is_rom(air_id: usize) -> bool {
        air_id == ROM_AIR_IDS[0]
    }

    /// Checks if the plan targets the ROM instance that requires special handling.
    ///
    /// ROM instances need to be added to the proof context with first partition assignment.
    #[inline]
    pub fn is_rom_instance(airgroup_id: usize, air_id: usize) -> bool {
        airgroup_id == ZISK_AIRGROUP_ID && Self::is_rom(air_id)
    }

    /// Checks if the AIR ID corresponds to a memory-related state machine.
    ///
    /// Memory-related AIRs include MEM, ROM_DATA, and INPUT_DATA.
    #[inline]
    pub fn is_memory_related(air_id: usize) -> bool {
        air_id == MEM_AIR_IDS[0] || air_id == ROM_DATA_AIR_IDS[0] || air_id == INPUT_DATA_AIR_IDS[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_main() {
        for &air_id in MAIN_AIR_IDS {
            assert!(AirClassifier::is_main(air_id));
        }
    }

    #[test]
    fn test_is_rom() {
        for &air_id in ROM_AIR_IDS {
            assert!(AirClassifier::is_rom(air_id));
        }
    }

    #[test]
    fn test_is_memory_related() {
        assert!(AirClassifier::is_memory_related(MEM_AIR_IDS[0]));
        assert!(AirClassifier::is_memory_related(ROM_DATA_AIR_IDS[0]));
        assert!(AirClassifier::is_memory_related(INPUT_DATA_AIR_IDS[0]));
    }
}
