#[allow(dead_code)]
fn format_u64_hex(value: u64) -> String {
    let hex_str = format!("{value:016x}");
    hex_str
        .as_bytes()
        .chunks(4)
        .map(|chunk| std::str::from_utf8(chunk).unwrap())
        .collect::<Vec<_>>()
        .join("_")
}

#[derive(Debug, Clone)]
pub struct MemAlignInput {
    pub addr: u32,
    pub is_write: bool,
    pub width: u8,
    pub step: u64,
    pub value: u64,
    pub mem_values: [u64; 2],
}

#[derive(Debug)]
pub struct MemInput {
    pub addr: u32,      // address in word native format means byte_address / MEM_BYTES
    pub is_write: bool, // it's a write operation
    pub step: u64,      // mem_step = f(main_step, main_step_offset)
    pub value: u64,     // value to read or write
}
