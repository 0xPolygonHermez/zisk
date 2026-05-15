mod blake2;
mod blake2_bus_device;
mod blake2_constants;
mod blake2_gen_mem_inputs;
mod blake2_input;
mod blake2_instance;
mod blake2_manager;
mod blake2_planner;

pub use blake2::*;
pub use blake2_bus_device::*;
pub use blake2_constants::*;
pub use blake2_gen_mem_inputs::*;
pub use blake2_input::*;
pub use blake2_instance::*;
pub use blake2_manager::*;
pub use blake2_planner::*;

#[cfg(test)]
mod blake2_tests {
    use test_artifacts::ELF_BLAKE2;
    use zisk_common::io::ZiskStdin;

    /// Number of `syscall_blake2b_round` invocations the guest will perform.
    const NUM_BLAKE2B_ROUNDS: u64 = 10;

    #[test]
    fn execute_blake2() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_BLAKE2B_ROUNDS);

        ELF_BLAKE2.run_emulation(stdin, None).expect("blake2 guest emulation failed");
    }
}
