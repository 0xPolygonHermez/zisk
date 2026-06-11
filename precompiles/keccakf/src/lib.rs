mod keccakf;
mod keccakf_constants;
mod keccakf_expr_generator;
mod keccakf_mem_inputs;
mod keccakf_table;

pub use keccakf::*;
use keccakf_constants::*;
pub use keccakf_expr_generator::*;
use keccakf_table::*;

zisk_common::zisk_precompile! {
    name = Keccakf,
    op_type = Keccak,
    trace = KeccakfTrace,
    num_available = {
        ::zisk_pil::KeccakfTrace::<()>::NUM_ROWS / CLOCKS
            - (::zisk_pil::KeccakfTrace::<()>::NUM_ROWS % CLOCKS != 0) as usize
    },
    ops = [
        (OperationKeccakData, KeccakfInput),
    ],
}

#[cfg(test)]
mod keccakf_tests {
    use test_artifacts::ELF_KECCAK;
    use zisk_common::io::ZiskStdin;

    /// Number of `syscall_keccak_f` invocations the guest will perform.
    const NUM_KECCAKFS: u64 = 10;

    #[test]
    fn keccakf_tests() {
        let stdin = ZiskStdin::new();
        stdin.write(&NUM_KECCAKFS);

        ELF_KECCAK.run_emulation(stdin, None).expect("keccak guest emulation failed");
    }
}

// =====================================================================
// Unit-test framework marker.
//
// NOTE: Keccakf packs multiple inputs per circuit; a constant
// `rows_per_input` can't express it cleanly. Hooks see absolute
// `row_idx` in `input_idx` with `clock` = 0; the closure can recompute
// mappings from `precomp_keccakf` constants if needed.
// =====================================================================

use zisk_common::unit_test_sm;
use zisk_pil::{KeccakfTrace, KeccakfTraceRow, KeccakfTraceRowPacked, KECCAKF_AIR_IDS};

// The `trace:` line additionally emits the raw trace-authoring override
// impl, letting a unit test bypass `compute_witness` and write the Keccakf
// trace directly (see `TraceOverrideBag`).
unit_test_sm! {
    KeccakfSm => {
        name: "Keccakf",
        air: KECCAKF_AIR_IDS[0],
        input: KeccakfInput,
        manager: KeccakfSM<F>,
        row: KeccakfTraceRow<F>,
        row_packed: KeccakfTraceRowPacked<F>,
        trace: KeccakfTrace,
        chunk_size: |sm| sm.num_available_keccakfs,
    }
}
