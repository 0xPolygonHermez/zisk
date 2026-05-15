mod poseidon2;
mod poseidon2_bus_device;
mod poseidon2_gen_mem_inputs;
mod poseidon2_input;
mod poseidon2_instance;
mod poseidon2_manager;
mod poseidon2_planner;

pub use poseidon2::*;
pub use poseidon2_bus_device::*;
pub use poseidon2_gen_mem_inputs::*;
pub use poseidon2_input::*;
pub use poseidon2_instance::*;
pub use poseidon2_manager::*;
pub use poseidon2_planner::*;

#[cfg(test)]
mod poseidon2_tests {
    use test_artifacts::ELF_POSEIDON2;
    use zisk_common::io::ZiskStdin;

    /// Number of `syscall_poseidon2` invocations the guest will perform.
    const NUM_POSEIDON2S: u64 = 10;

    #[test]
    fn execute_poseidon2() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_POSEIDON2S);

        ELF_POSEIDON2.run_emulation(stdin, None).expect("poseidon2 guest emulation failed");
    }
}
