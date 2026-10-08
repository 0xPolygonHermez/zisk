mod poseidon;
mod poseidon_mem_inputs;

pub use poseidon::*;

zisk_common::zisk_precompile! {
    name = Poseidon,
    op_type = Poseidon,
    trace = PoseidonTrace,
    num_available = {
        ::zisk_pil::PoseidonTrace::<::zisk_pil::PoseidonTraceRow<F>>::NUM_ROWS / CLOCKS - 1
    },
    cost = ::zisk_pil::POSEIDON_INSTANCE_COST,
    ops = [
        (OperationPoseidonData, PoseidonInput),
    ],
}

#[cfg(test)]
mod poseidon_tests {
    use zisk_common::io::ZiskStdin;
    use zisk_test_artifacts::{ELF_POSEIDON1, ELF_POSEIDON2};

    /// Number of `syscall_poseidon2` invocations the guest will perform.
    const NUM_POSEIDON2S: u64 = 10;

    /// Number of `syscall_poseidon1` invocations the guest will perform.
    const NUM_POSEIDON1S: u64 = 10;

    #[test]
    fn poseidon2_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_POSEIDON2S);

        ELF_POSEIDON2.run_emulation(stdin, None).expect("poseidon2 guest emulation failed");
    }

    #[test]
    fn poseidon1_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_POSEIDON1S);

        ELF_POSEIDON1.run_emulation(stdin, None).expect("poseidon1 guest emulation failed");
    }
}
