mod poseidon2;
mod poseidon2_mem_inputs;

pub use poseidon2::*;

zisk_common::zisk_precompile! {
    name = Poseidon2,
    op_type = Poseidon2,
    trace = Poseidon2Trace,
    num_available = {
        ::zisk_pil::Poseidon2Trace::<::zisk_pil::Poseidon2TraceRow<F>>::NUM_ROWS / CLOCKS - 1
    },
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

// =====================================================================
// Unit-test framework marker.
// =====================================================================

use zisk_common::unit_test_sm;
use zisk_pil::{Poseidon2Trace, Poseidon2TraceRow, Poseidon2TraceRowPacked, POSEIDON_2_AIR_IDS};

unit_test_sm! {
    Poseidon2Sm => {
        name: "Poseidon2",
        air: POSEIDON_2_AIR_IDS[0],
        input: Poseidon2Input,
        manager: Poseidon2SM<F>,
        row: Poseidon2TraceRow<F>,
        row_packed: Poseidon2TraceRowPacked<F>,
        trace: Poseidon2Trace,
        rows_per_input: CLOCKS,
        chunk_size: |sm| sm.num_available_poseidon2s,
    }
}
