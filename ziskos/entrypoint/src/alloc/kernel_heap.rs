//! Kernel heap symbols defined in the linker script

extern "C" {
    pub static _heap_bottom: u8;
    pub static _heap_size: u8;
    pub static _heap_top: u8;
}
