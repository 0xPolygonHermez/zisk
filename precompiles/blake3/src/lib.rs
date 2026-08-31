mod blake3;
mod blake3_constants;
mod blake3_mem_inputs;
mod blake3_table;

pub use blake3::*;
pub use blake3_constants::*;

zisk_common::zisk_precompile! {
    name = Blake3,
    op_type = Blake3,
    trace = Blake3fTrace,
    num_available = {
        let n = ::zisk_pil::Blake3fTrace::<::zisk_pil::Blake3fTraceRow<F>>::NUM_ROWS;
        n / CLOCKS
    },
    ops = [
        (OperationBlake3Data, Blake3Input),
    ],
}

#[cfg(test)]
mod blake3_tests {
    use zisk_common::io::ZiskStdin;
    use zisk_test_artifacts::ELF_BLAKE3;

    /// Number of `syscall_blake3f` invocations the guest will perform.
    const NUM_BLAKE3FS: u64 = 10;

    #[test]
    fn blake3_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_BLAKE3FS);

        ELF_BLAKE3.run_emulation(stdin, None).expect("blake3 guest emulation failed");
    }
}
