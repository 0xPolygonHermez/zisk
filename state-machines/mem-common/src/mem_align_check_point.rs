#[derive(Clone, Debug)]
pub struct MemAlignCheckPoint {
    pub skip: u32,
    pub count: u32,
    pub rows: u32,
    pub offset: u32,
}

impl MemAlignCheckPoint {
    #[allow(dead_code)]
    pub fn to_string(&self, segment_id: usize, chunk_id: usize) -> String {
        format!(
            "MEM_ALIGN #{}@{}  S:{} C:{} R:{}\n",
            segment_id, chunk_id, self.skip, self.count, self.rows,
        )
    }
}
