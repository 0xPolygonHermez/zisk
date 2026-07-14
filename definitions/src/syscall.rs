// Syscall 0x800 - 0x84F (80 syscalls)
//
// Important: syscall ids must be contiguous and in the same order as in
// riscv2zisk_context.rs (the CSR_PRECOMPILED table, indexed by `csr - START`).
//
// The precompile syscall ids (`SYSCALL_<name>_ID`) are generated from the
// precompile manifests into `syscall_ids.gen.rs` by `cargo-zisk-dev gen-ops`.
// The non-precompile syscalls (DMA, profile) are hand-written below. Together
// they must cover the window with no gaps or collisions.

include!("syscall_ids.gen.rs");

pub const SYSCALL_DMA_MEMCPY_ID: u16 = 0x813;
pub const SYSCALL_DMA_MEMCMP_ID: u16 = 0x814;
pub const SYSCALL_DMA_INPUTCPY_ID: u16 = 0x815;
pub const SYSCALL_DMA_MEMSET_ID: u16 = 0x816;
pub const SYSCALL_PROFILE_ID: u16 = 0x81A;
