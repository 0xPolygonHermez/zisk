mod add256;
mod add256_constants;
mod add256_mem_inputs;

pub use add256::*;
pub use add256_constants::*;

zisk_common::zisk_precompile! {
    name = Add256,
    op_type = BigInt,
    trace = Add256Trace,
    num_available = ::zisk_pil::Add256Trace::<()>::NUM_ROWS,
    ops = [
        (OperationAdd256Data, Add256Input),
    ],
}

#[cfg(test)]
mod add256_tests {
    use test_artifacts::ELF_ADD256;
    use zisk_common::io::ZiskStdin;

    #[test]
    fn add256_tests() {
        ELF_ADD256.run_emulation(ZiskStdin::new(), None).expect("add256 guest emulation failed");
    }
}
