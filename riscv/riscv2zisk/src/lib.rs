/*pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}*/

mod elf2rom;
mod riscv2zisk;
mod riscv_inst;
mod riscv_interpreter;
mod riscv_registers;
mod riscv_rvd;
mod utils;
mod zisk_definitions;
mod zisk_inst;
mod zisk_inst_builder;
mod zisk_operation;
mod zisk_operations;
mod zisk_rom;
// mod zisk_sources;
mod zv2zisk;

pub use elf2rom::*;
pub use riscv2zisk::*;
pub use riscv_inst::*;
pub use riscv_interpreter::*;
pub use riscv_registers::*;
pub use riscv_rvd::*;
pub use utils::*;
pub use zisk_definitions::*;
pub use zisk_inst::*;
pub use zisk_inst_builder::*;
pub use zisk_operation::*;
pub use zisk_operations::*;
pub use zisk_rom::*;
// pub use zisk_sources::*;
pub use zv2zisk::*;
