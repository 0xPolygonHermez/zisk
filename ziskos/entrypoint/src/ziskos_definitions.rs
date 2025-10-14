pub mod ziskos_config {
    pub static mut SV: u64 = 0xBBBB;

    pub const QEMU_EXIT_ADDR: u64 = 0x100000;
    pub const QEMU_EXIT_CODE: u64 = 0x5555;
    pub const INPUT_ADDR: u64 = 0x9000_0000;
    pub const OUTPUT_ADDR: u64 = 0xa001_0000;
    pub const UART_ADDR: u64 = 0xa000_0200;
    pub const ARCH_ID_ZISK: u64 = 0xFFFEEEE; // TEMPORARY  // TODO register one

    pub const MAX_INPUT: usize = 0x2000;
    pub const MAX_OUTPUT: usize = 0x1_0000;
}
