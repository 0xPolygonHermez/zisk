//! Kernel heap symbols defined in the linker script

extern "C" {
    pub static _kernel_heap_bottom: u8;
    pub static _kernel_heap_size: u8;
    pub static _kernel_heap_top: u8;
}
