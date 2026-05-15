mod sha256f;
mod sha256f_bus_device;
mod sha256f_constants;
mod sha256f_gen_mem_inputs;
mod sha256f_input;
mod sha256f_instance;
mod sha256f_manager;
mod sha256f_planner;

pub use sha256f::*;
pub use sha256f_bus_device::*;
pub use sha256f_constants::*;
pub use sha256f_gen_mem_inputs::*;
pub use sha256f_input::*;
pub use sha256f_instance::*;
pub use sha256f_manager::*;
pub use sha256f_planner::*;

#[cfg(test)]
mod sha256f_tests {
    use test_artifacts::ELF_SHA256;
    use zisk_common::io::ZiskStdin;

    /// Number of `syscall_sha256_f` invocations the guest will perform.
    const NUM_SHA256FS: u64 = 10;

    #[test]
    fn execute_sha256() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_SHA256FS);

        ELF_SHA256.run_emulation(stdin, None).expect("sha256 guest emulation failed");
    }
}
