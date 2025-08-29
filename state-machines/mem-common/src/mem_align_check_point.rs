use zisk_common::CollectCounter;

#[derive(Debug, Clone)]
pub struct MemAlignCheckPoint {
    pub full_2: CollectCounter,
    pub full_3: CollectCounter,
    pub full_5: CollectCounter,
    pub read_byte: CollectCounter,
    pub write_byte: CollectCounter,
    // pub rows: u32,
    // pub offset: u32,
}

impl MemAlignCheckPoint {
    pub fn count(&self) -> u32 {
        self.full_2.count()
            + self.full_3.count()
            + self.full_5.count()
            + self.read_byte.count()
            + self.write_byte.count()
    }
    #[allow(dead_code)]
    pub fn to_string(&self, segment_id: usize, chunk_id: usize) -> String {
        format!(
            "MEM_ALIGN #{}@{}  F2({},{}) F3({},{}) F5({},{}) R({},{}) W({},{}) R:{}\n",
            segment_id,
            chunk_id,
            self.full_2.skip(),
            self.full_2.count(),
            self.full_3.skip(),
            self.full_3.count(),
            self.full_5.skip(),
            self.full_5.count(),
            self.read_byte.skip(),
            self.read_byte.count(),
            self.write_byte.skip(),
            self.write_byte.count(),
            self.count(),
        )
    }
}
