mod add256;
mod add256_bus_device;
mod add256_constants;
mod add256_gen_mem_inputs;
mod add256_input;
mod add256_instance;
mod add256_manager;
mod add256_planner;

pub use add256::*;
pub use add256_bus_device::*;
pub use add256_constants::*;
pub use add256_gen_mem_inputs::*;
pub use add256_input::*;
pub use add256_instance::*;
pub use add256_manager::*;
pub use add256_planner::*;

#[cfg(test)]
mod add256_tests {
    use test_artifacts::ELF_ADD256;
    use zisk_common::io::ZiskStdin;

    #[test]
    fn execute_add256_tests() {
        let stdin = ZiskStdin::new();

        ELF_ADD256.run_emulation(stdin, None).expect("add256 guest emulation failed");
    }
}
