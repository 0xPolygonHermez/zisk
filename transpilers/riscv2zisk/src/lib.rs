//! **Deprecated:** use `zisk-transpiler-common` instead (`ZiskTranspiler` and `program2rom`).
//!
//! Former home of the guest-format dispatcher, kept under its published name so existing users
//! keep compiling. The code now lives in `zisk-transpiler-common`; see `transpilers/README.md`.

pub mod riscv2zisk;

#[allow(deprecated)]
pub use riscv2zisk::*;
