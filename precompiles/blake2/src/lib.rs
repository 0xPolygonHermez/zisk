mod blake2;
mod blake2_constants;
mod blake2_mem_inputs;
mod blake2_table;
mod blake2s;
mod blake2s_mem_inputs;

pub use blake2::*;
pub use blake2_constants::*;
pub use blake2s::*;

zisk_common::zisk_precompile! {
    name = Blake2s,
    op_type = Blake2s,
    trace = Blake2srTrace,
    num_available = {
        let n = ::zisk_pil::Blake2srTrace::<::zisk_pil::Blake2srTraceRow<F>>::NUM_ROWS;
        n / CLOCKS - (n % CLOCKS != 0) as usize
    },
    ops = [
        (OperationBlake2Data, Blake2sInput),
    ],
}

zisk_common::zisk_precompile! {
    name = Blake2,
    op_type = Blake2,
    trace = Blake2brTrace,
    num_available = {
        let n = ::zisk_pil::Blake2brTrace::<::zisk_pil::Blake2brTraceRow<F>>::NUM_ROWS;
        n / CLOCKS - (n % CLOCKS != 0) as usize
    },
    ops = [
        (OperationBlake2Data, Blake2Input),
    ],
}

#[cfg(test)]
mod blake2s_tests {
    use zisk_common::io::ZiskStdin;
    use zisk_test_artifacts::ELF_BLAKE2S;

    /// Number of `syscall_blake2s_round` invocations the guest will perform.
    const NUM_BLAKE2S_ROUNDS: u64 = 10;

    #[test]
    fn blake2s_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_BLAKE2S_ROUNDS);

        ELF_BLAKE2S.run_emulation(stdin, None).expect("blake2s guest emulation failed");
    }
}

#[cfg(test)]
mod blake2_tests {
    use zisk_common::io::ZiskStdin;
    use zisk_test_artifacts::ELF_BLAKE2;

    /// Number of `syscall_blake2b_round` invocations the guest will perform.
    const NUM_BLAKE2B_ROUNDS: u64 = 10;

    #[test]
    fn blake2_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_BLAKE2B_ROUNDS);

        ELF_BLAKE2.run_emulation(stdin, None).expect("blake2 guest emulation failed");
    }
}
