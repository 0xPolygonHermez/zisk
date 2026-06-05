mod keccakf;
mod keccakf_constants;
mod keccakf_expr_generator;
mod keccakf_mem_inputs;
mod keccakf_table;

pub use keccakf::*;
use keccakf_constants::*;
pub use keccakf_expr_generator::*;
use keccakf_table::*;

zisk_common::zisk_precompile! {
    name = Keccakf,
    op_type = Keccak,
    trace = KeccakfTrace,
    num_available = {
        ::zisk_pil::KeccakfTrace::<()>::NUM_ROWS / CLOCKS
            - (::zisk_pil::KeccakfTrace::<()>::NUM_ROWS % CLOCKS != 0) as usize
    },
    ops = [
        (OperationKeccakData, KeccakfInput),
    ],
}

#[cfg(test)]
mod keccakf_tests {
    use test_artifacts::ELF_KECCAK;
    use zisk_common::io::ZiskStdin;

    /// Number of `syscall_keccak_f` invocations the guest will perform.
    const NUM_KECCAKFS: u64 = 10;

    #[test]
    fn keccakf_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_KECCAKFS);

        ELF_KECCAK.run_emulation(stdin, None).expect("keccak guest emulation failed");
    }
}
