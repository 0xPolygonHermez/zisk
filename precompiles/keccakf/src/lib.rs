mod keccakf;
mod keccakf_bus_device;
mod keccakf_constants;
mod keccakf_expr_generator;
mod keccakf_gen_mem_inputs;
mod keccakf_input;
mod keccakf_instance;
mod keccakf_manager;
mod keccakf_planner;
mod keccakf_table;

pub use keccakf::*;
pub use keccakf_bus_device::*;
use keccakf_constants::*;
pub use keccakf_expr_generator::*;
pub use keccakf_gen_mem_inputs::*;
pub use keccakf_input::*;
pub use keccakf_instance::*;
pub use keccakf_manager::*;
pub use keccakf_planner::*;
use keccakf_table::*;

#[cfg(test)]
mod keccak_tests {
    use test_artifacts::ELF_KECCAK;
    use zisk_common::io::ZiskStdin;

    /// Number of `syscall_keccak_f` invocations the guest will perform.
    const NUM_KECCAKFS: u64 = 10;

    #[test]
    fn execute_keccak() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_KECCAKFS);

        ELF_KECCAK.run_emulation(stdin, None).expect("keccak guest emulation failed");
    }
}
