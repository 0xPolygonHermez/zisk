use precompiles_helpers::DmaInfo;

pub enum DmaPrePostRom {}

impl DmaPrePostRom {
    // Table generated from pil
    const TABLE_OFFSETS: [usize; 64] = [
        0, 7, 14, 21, 28, 35, 42, 49, 56, 63, 70, 77, 84, 91, 98, 105, 112, 118, 124, 130, 136,
        142, 148, 154, 160, 165, 170, 175, 180, 185, 190, 195, 200, 204, 208, 212, 216, 220, 224,
        228, 232, 235, 238, 241, 244, 247, 250, 253, 256, 258, 260, 262, 264, 266, 268, 270, 272,
        273, 274, 275, 276, 277, 278, 279,
    ];
    #[allow(dead_code)]
    pub fn get_row_from_encoded(encoded: u64) -> usize {
        let src_offset = DmaInfo::get_src_offset(encoded);
        let dst_offset = DmaInfo::get_dst_offset(encoded);
        let count = DmaInfo::get_count(encoded);
        Self::get_row(dst_offset, src_offset, count)
    }
    pub fn get_row(dst_offset: usize, src_offset: usize, count: usize) -> usize {
        Self::TABLE_OFFSETS[dst_offset * 8 + src_offset] + count - 1
    }
}
