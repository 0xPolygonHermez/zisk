#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct MemAlignCounters {
    pub chunk_id: u32,
    // full - 5 rows (non-aligned 2 address write)
    pub full_5: u32,
    // full - 3 rows (non-aligned 1 address write, 2 address read)
    pub full_3: u32,
    // full - 2 rows (non-aligned 1 address read)
    pub full_2: u32,
    pub read_byte: u32,
    pub write_byte: u32,
}

impl MemAlignCounters {
    pub fn to_array(&self) -> [u32; 5] {
        [self.full_5, self.full_3, self.full_2, self.read_byte, self.write_byte]
    }
}
