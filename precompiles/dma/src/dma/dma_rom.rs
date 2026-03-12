pub enum DmaRom {}

impl DmaRom {
    #[allow(dead_code)]
    pub fn get_row(
        dst_offset: u32,
        src_offset: u32,
        count: usize,
        neq: bool,
        use_src: bool,
    ) -> usize {
        assert!(dst_offset < 8, "dst_offset too big {dst_offset}");
        assert!(src_offset < 8, "src_offset too big {src_offset}");
        assert!(count < u32::MAX as usize, "count too big {count}");
        assert!(!neq || use_src);
        let count = if count >= 256 { (count & 0xFF) + 256 } else { count & 0xFF };
        (dst_offset as usize * 8 + src_offset as usize) * 512
            + count
            + if neq { 1 << 15 } else { 0 }
            + if use_src { 0 } else { 1 << 16 }
    }
}
