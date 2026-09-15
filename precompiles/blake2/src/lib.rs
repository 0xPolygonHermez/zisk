mod blake2b;
pub mod blake2b_constants;
mod blake2b_mem_inputs;
mod blake2s;
pub mod blake2s_constants;
mod blake2s_mem_inputs;
mod blake_table;

pub use blake2b::*;
pub use blake2s::*;
pub use blake_table::*;

zisk_common::zisk_precompile! {
    name = Blake2b,
    op_type = Blake2b,
    trace = Blake2brTrace,
    num_available = {
        let n = ::zisk_pil::Blake2brTrace::<::zisk_pil::Blake2brTraceRow<F>>::NUM_ROWS;
        n / crate::blake2b_constants::CLOCKS
    },
    ops = [
        (OperationBlake2bData, Blake2bInput),
    ],
}

zisk_common::zisk_precompile! {
    name = Blake2s,
    op_type = Blake2s,
    trace = Blake2sTrace,
    num_available = {
        let n = ::zisk_pil::Blake2sTrace::<::zisk_pil::Blake2sTraceRow<F>>::NUM_ROWS;
        n / crate::blake2s_constants::CLOCKS
    },
    ops = [
        (OperationBlake2sData, Blake2sInput),
    ],
}

#[cfg(test)]
mod blake2b_tests {
    use zisk_common::io::ZiskStdin;
    use zisk_test_artifacts::ELF_BLAKE2B;

    /// Number of `syscall_blake2b_round` invocations the guest will perform.
    const NUM_BLAKE2B_ROUNDS: u64 = 10;

    #[test]
    fn blake2b_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_BLAKE2B_ROUNDS);

        ELF_BLAKE2B.run_emulation(stdin, None).expect("blake2b guest emulation failed");
    }
}

#[cfg(test)]
mod blake2s_tests {
    use zisk_common::io::ZiskStdin;
    use zisk_test_artifacts::ELF_BLAKE2S;

    /// Number of `syscall_blake2sf` invocations the guest will perform.
    const NUM_BLAKE2SFS: u64 = 10;

    #[test]
    fn blake2s_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_BLAKE2SFS);

        ELF_BLAKE2S.run_emulation(stdin, None).expect("blake2s guest emulation failed");
    }
}
