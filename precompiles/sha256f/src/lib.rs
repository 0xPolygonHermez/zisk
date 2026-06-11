mod sha256f;
mod sha256f_constants;
mod sha256f_mem_inputs;

pub use sha256f::*;
pub use sha256f_constants::*;

zisk_common::zisk_precompile! {
    name = Sha256f,
    op_type = Sha256,
    trace = Sha256fTrace,
    num_available = {
        ::zisk_pil::Sha256fTrace::<::zisk_pil::Sha256fTraceRow<F>>::NUM_ROWS / CLOCKS - 1
    },
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
    fn sha256f_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_SHA256FS);

        ELF_SHA256.run_emulation(stdin, None).expect("sha256f guest emulation failed");
    }
}

// =====================================================================
// Unit-test framework marker.
// =====================================================================

use zisk_common::unit_test_sm;
use zisk_pil::{Sha256fTrace, Sha256fTraceRow, Sha256fTraceRowPacked, SHA_256_F_AIR_IDS};

unit_test_sm! {
    Sha256fSm => {
        name: "Sha256f",
        air: SHA_256_F_AIR_IDS[0],
        input: Sha256fInput,
        manager: Sha256fSM<F>,
        row: Sha256fTraceRow<F>,
        row_packed: Sha256fTraceRowPacked<F>,
        trace: Sha256fTrace,
        rows_per_input: CLOCKS,
        chunk_size: |sm| sm.num_available_sha256fs,
    }
}
