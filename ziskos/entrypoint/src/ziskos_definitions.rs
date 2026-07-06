pub mod ziskos_config {
    pub static mut SV: u64 = 0xBBBB;

    pub const QEMU_EXIT_ADDR: u64 = 0x100000;
    pub const QEMU_EXIT_CODE: u64 = 0x5555;
    pub const INPUT_ADDR: u64 = 0x4000_0000;
    pub const RAM_ADDR: u64 = 0xA0000000;
    pub const RAM_SIZE: u64 = 0x20000000; // 512M
    pub const STACK_SIZE: u64 = 0x400000; // 4MB
    pub const SYS_ADDR: u64 = RAM_ADDR + STACK_SIZE;
    pub const SYS_SIZE: u64 = 0x10000;
    pub const OUTPUT_ADDR: u64 = SYS_ADDR + SYS_SIZE;
    pub const UART_ADDR: u64 = SYS_ADDR + 0x200;
    pub const ARCH_ID_ZISK: u64 = 0xFFFEEEE; // TEMPORARY  // TODO register one

    pub const MAX_INPUT: usize = 0x2000;
    pub const MAX_OUTPUT: usize = 0x1_0000;
}
