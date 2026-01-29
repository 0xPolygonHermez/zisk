use precompiles_helpers::DmaInfo;

pub enum DmaRom {}

impl DmaRom {
    #[allow(dead_code)]
    pub fn get_row_from_encoded(encoded: u64) -> usize {
        let src_offset = DmaInfo::get_src_offset(encoded);
        let dst_offset = DmaInfo::get_dst_offset(encoded);
        let count = DmaInfo::get_count(encoded);
        Self::get_row(dst_offset as u32, src_offset as u32, count)
    }
    pub fn get_row(dst_offset: u32, src_offset: u32, count: usize) -> usize {
        assert!(dst_offset < 8, "dst_offset too big {dst_offset}");
        assert!(src_offset < 8, "src_offset too big {src_offset}");
        assert!(count < u32::MAX as usize, "count too big {count}");
        let count = if count >= 256 { (count & 0xFF) + 256 } else { count & 0xFF };
        (dst_offset as usize * 8 + src_offset as usize) * 512 + count
    }
}
