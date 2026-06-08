//! AIR classification helpers.

use zisk_pil::{
    ADD_256_AIR_IDS, ARITH_AIR_IDS, ARITH_EQ_384_AIR_IDS, ARITH_EQ_AIR_IDS, BINARY_ADD_AIR_IDS,
    BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS, BLAKE_2_BR_AIR_IDS, DMA_64_ALIGNED_AIR_IDS,
    DMA_64_ALIGNED_INPUT_CPY_AIR_IDS, DMA_64_ALIGNED_MEM_AIR_IDS, DMA_64_ALIGNED_MEM_CPY_AIR_IDS,
    DMA_64_ALIGNED_MEM_SET_AIR_IDS, DMA_AIR_IDS, DMA_INPUT_CPY_AIR_IDS, DMA_MEM_CPY_AIR_IDS,
    DMA_PRE_POST_AIR_IDS, DMA_PRE_POST_INPUT_CPY_AIR_IDS, DMA_PRE_POST_MEM_CPY_AIR_IDS,
    DMA_UNALIGNED_AIR_IDS, INPUT_DATA_AIR_IDS, KECCAKF_AIR_IDS, MAIN_AIR_IDS, MEM_AIR_IDS,
    MEM_ALIGN_AIR_IDS, MEM_ALIGN_BYTE_AIR_IDS, MEM_ALIGN_READ_BYTE_AIR_IDS,
    MEM_ALIGN_WRITE_BYTE_AIR_IDS, POSEIDON_AIR_IDS, ROM_AIR_IDS, ROM_DATA_AIR_IDS,
    SHA_256_F_AIR_IDS, VIRTUAL_TABLE_ZISK_0_AIR_IDS, VIRTUAL_TABLE_ZISK_1_AIR_IDS,
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

    /// Checks if the AIR ID corresponds to a memory-related state machine.
    #[inline]
    pub fn is_memory_related(air_id: usize) -> bool {
        air_id == MEM_AIR_IDS[0] || air_id == ROM_DATA_AIR_IDS[0] || air_id == INPUT_DATA_AIR_IDS[0]
    }

    /// Display name for a known `(airgroup_id, air_id)` pair. Returns
    /// `"Unknown"` for unrecognised pairs. Used by the standalone plan
    /// summary; no proofman/setup lookup required.
    pub fn name(airgroup_id: usize, air_id: usize) -> &'static str {
        if airgroup_id != ZISK_AIRGROUP_ID {
            return "Unknown";
        }
        if MAIN_AIR_IDS.contains(&air_id) {
            "Main"
        } else if ROM_AIR_IDS.contains(&air_id) {
            "Rom"
        } else if MEM_AIR_IDS.contains(&air_id) {
            "Mem"
        } else if ROM_DATA_AIR_IDS.contains(&air_id) {
            "RomData"
        } else if INPUT_DATA_AIR_IDS.contains(&air_id) {
            "InputData"
        } else if MEM_ALIGN_AIR_IDS.contains(&air_id) {
            "MemAlign"
        } else if MEM_ALIGN_BYTE_AIR_IDS.contains(&air_id) {
            "MemAlignByte"
        } else if MEM_ALIGN_READ_BYTE_AIR_IDS.contains(&air_id) {
            "MemAlignReadByte"
        } else if MEM_ALIGN_WRITE_BYTE_AIR_IDS.contains(&air_id) {
            "MemAlignWriteByte"
        } else if ARITH_AIR_IDS.contains(&air_id) {
            "Arith"
        } else if BINARY_AIR_IDS.contains(&air_id) {
            "Binary"
        } else if BINARY_ADD_AIR_IDS.contains(&air_id) {
            "BinaryAdd"
        } else if BINARY_EXTENSION_AIR_IDS.contains(&air_id) {
            "BinaryExtension"
        } else if ADD_256_AIR_IDS.contains(&air_id) {
            "Add256"
        } else if ARITH_EQ_AIR_IDS.contains(&air_id) {
            "ArithEq"
        } else if ARITH_EQ_384_AIR_IDS.contains(&air_id) {
            "ArithEq384"
        } else if KECCAKF_AIR_IDS.contains(&air_id) {
            "Keccakf"
        } else if SHA_256_F_AIR_IDS.contains(&air_id) {
            "Sha256f"
        } else if POSEIDON_AIR_IDS.contains(&air_id) {
            "Poseidon"
        } else if BLAKE_2_BR_AIR_IDS.contains(&air_id) {
            "Blake2"
        } else if VIRTUAL_TABLE_ZISK_0_AIR_IDS.contains(&air_id) {
            "VirtualTable0"
        } else if VIRTUAL_TABLE_ZISK_1_AIR_IDS.contains(&air_id) {
            "VirtualTable1"
        } else if DMA_AIR_IDS.contains(&air_id) {
            "Dma"
        } else if DMA_MEM_CPY_AIR_IDS.contains(&air_id) {
            "DmaMemCpy"
        } else if DMA_INPUT_CPY_AIR_IDS.contains(&air_id) {
            "DmaInputCpy"
        } else if DMA_64_ALIGNED_AIR_IDS.contains(&air_id) {
            "Dma64Aligned"
        } else if DMA_64_ALIGNED_INPUT_CPY_AIR_IDS.contains(&air_id) {
            "Dma64AlignedInputCpy"
        } else if DMA_64_ALIGNED_MEM_SET_AIR_IDS.contains(&air_id) {
            "Dma64AlignedMemSet"
        } else if DMA_64_ALIGNED_MEM_AIR_IDS.contains(&air_id) {
            "Dma64AlignedMem"
        } else if DMA_64_ALIGNED_MEM_CPY_AIR_IDS.contains(&air_id) {
            "Dma64AlignedMemCpy"
        } else if DMA_UNALIGNED_AIR_IDS.contains(&air_id) {
            "DmaUnaligned"
        } else if DMA_PRE_POST_AIR_IDS.contains(&air_id) {
            "DmaPrePost"
        } else if DMA_PRE_POST_MEM_CPY_AIR_IDS.contains(&air_id) {
            "DmaPrePostMemCpy"
        } else if DMA_PRE_POST_INPUT_CPY_AIR_IDS.contains(&air_id) {
            "DmaPrePostInputCpy"
        } else {
            "Unknown"
        }
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
