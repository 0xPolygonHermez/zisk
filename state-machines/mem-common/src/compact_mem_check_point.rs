//! What a `CompactMem` instance is planned to prove: one segment of each memory area.

use crate::MemModuleSegmentCheckPoint;

/// The three memory areas the fused `CompactMem` air carries side by side, in the order its blocks
/// appear on a row (see `state-machines/mem/pil/compact_mem.pil`).
///
/// Used wherever something has to be done once per block and the code should not be able to forget
/// one of the three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemArea {
    /// The mutable RAM area, proved by the `Mem` air outside `CompactMem`.
    Mem,
    /// The free-input area, proved by the `InputData` air outside `CompactMem`.
    InputData,
    /// The read-only program data, proved by the `RomData` air outside `CompactMem`.
    RomData,
}

impl MemArea {
    pub const ALL: [MemArea; 3] = [MemArea::Mem, MemArea::InputData, MemArea::RomData];

    /// Short name, matching the one the standalone module reports in the witness logs.
    pub fn name(&self) -> &'static str {
        match self {
            MemArea::Mem => "ram",
            MemArea::InputData => "input",
            MemArea::RomData => "rom",
        }
    }
}

/// The plan meta of a `CompactMem` instance: the segment each block is to prove.
///
/// One `MemModuleSegmentCheckPoint` per area, exactly the ones the standalone airs would have been
/// given -- the blocks are sized like the airs they take the segment from, so the planners produce
/// them unchanged and only the assembly of the plans differs.
#[derive(Debug, Default, Clone)]
pub struct CompactMemSegmentCheckPoint {
    pub mem: MemModuleSegmentCheckPoint,
    pub input_data: MemModuleSegmentCheckPoint,
    pub rom_data: MemModuleSegmentCheckPoint,
}

impl CompactMemSegmentCheckPoint {
    pub fn new(
        mem: MemModuleSegmentCheckPoint,
        input_data: MemModuleSegmentCheckPoint,
        rom_data: MemModuleSegmentCheckPoint,
    ) -> Self {
        Self { mem, input_data, rom_data }
    }

    pub fn area(&self, area: MemArea) -> &MemModuleSegmentCheckPoint {
        match area {
            MemArea::Mem => &self.mem,
            MemArea::InputData => &self.input_data,
            MemArea::RomData => &self.rom_data,
        }
    }
}
