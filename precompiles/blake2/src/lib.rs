mod blake2;
mod blake2_constants;
mod blake2_mem_inputs;

pub use blake2::*;
pub use blake2_constants::*;

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
mod blake2_tests {
    use test_artifacts::ELF_BLAKE2;
    use zisk_common::io::ZiskStdin;

    /// Number of `syscall_blake2b_round` invocations the guest will perform.
    const NUM_BLAKE2B_ROUNDS: u64 = 10;

    #[test]
    fn blake2_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_BLAKE2B_ROUNDS);

        ELF_BLAKE2.run_emulation(stdin, None).expect("blake2 guest emulation failed");
    }
}
