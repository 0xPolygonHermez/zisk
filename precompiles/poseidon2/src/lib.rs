mod poseidon2;
mod poseidon2_mem_inputs;

pub use poseidon2::*;

zisk_common::zisk_precompile! {
    name = Poseidon2,
    op_type = Poseidon2,
    trace = Poseidon2Trace,
    num_available_field = num_available_poseidon2s,
    ops = [
        (OperationPoseidon2Data, Poseidon2Input),
    ],
}

#[cfg(test)]
mod poseidon2_tests {
    use test_artifacts::ELF_POSEIDON2;
    use zisk_common::io::ZiskStdin;

    /// Number of `syscall_poseidon2` invocations the guest will perform.
    const NUM_POSEIDON2S: u64 = 10;

    #[test]
    fn poseidon2_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_POSEIDON2S);

        ELF_POSEIDON2.run_emulation(stdin, None).expect("poseidon2 guest emulation failed");
    }
}
