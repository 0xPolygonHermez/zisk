mod keccakf;
mod keccakf_chi_table;
mod keccakf_constants;
mod keccakf_mem_inputs;
mod keccakf_xor5_table;

pub use keccakf::*;
use keccakf_chi_table::*;
use keccakf_constants::*;
use keccakf_xor5_table::*;

zisk_common::zisk_precompile! {
    name = Keccakf,
    op_type = Keccak,
    trace = KeccakfTrace,
    num_available = {
        OPS_PER_SLOT * (::zisk_pil::KeccakfTrace::<()>::NUM_ROWS / CLOCKS)
    },
    ops = [
        (OperationKeccakData, KeccakfInput),
    ],
}

#[cfg(test)]
mod keccakf_tests {
    use zisk_common::io::ZiskStdin;
    use zisk_test_artifacts::{ELF_KECCAK, ELF_KECCAKF_CACHE};

    /// Number of `syscall_keccak_f` invocations the guest will perform.
    const NUM_KECCAKFS: u64 = 10;

    #[test]
    fn keccakf_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_KECCAKFS);

        ELF_KECCAK.run_emulation(stdin, None).expect("keccak guest emulation failed");
    }

    /// Drives the `fcall_set_keccakf_cache_index` / `fcall_get_keccakf_cache_index` pair from a
    /// guest: the guest asserts every hit, miss and registration lifetime itself, so a failure
    /// surfaces as a failed emulation.
    #[test]
    fn keccakf_cache_tests() {
        ELF_KECCAKF_CACHE
            .run_emulation(ZiskStdin::new(), None)
            .expect("keccakf cache guest emulation failed");
    }
}
