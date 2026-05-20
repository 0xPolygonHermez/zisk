mod sha256f;
mod sha256f_constants;
mod sha256f_mem_inputs;

pub use sha256f::*;
pub use sha256f_constants::*;

zisk_common::zisk_precompile! {
    name = Sha256f,
    op_type = Sha256,
    trace = Sha256fTrace,
    num_available_field = num_available_sha256fs,
    ops = [
        (OperationSha256Data, Sha256fInput),
    ],
}

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
