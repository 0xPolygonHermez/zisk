mod keccakf;
mod keccakf_bus_device;
mod keccakf_constants;
mod keccakf_expr_generator;
mod keccakf_gen_mem_inputs;
mod keccakf_input;
mod keccakf_instance;
mod keccakf_manager;
mod keccakf_planner;
mod keccakf_table;

pub use keccakf::*;
pub use keccakf_bus_device::*;
use keccakf_constants::*;
pub use keccakf_expr_generator::*;
pub use keccakf_gen_mem_inputs::*;
pub use keccakf_input::*;
pub use keccakf_instance::*;
pub use keccakf_manager::*;
pub use keccakf_planner::*;
use keccakf_table::*;

// =====================================================================
// Unit-test framework marker.
//
// NOTE: Keccakf packs multiple inputs per circuit; a constant
// `rows_per_input` can't express it cleanly. Hooks see absolute
// `row_idx` in `input_idx` with `clock` = 0; the closure can recompute
// mappings from `precomp_keccakf` constants if needed.
// =====================================================================

use zisk_common::unit_test_sm;
use zisk_pil::{KeccakfTraceRow, KeccakfTraceRowPacked, KECCAKF_AIR_IDS};

unit_test_sm! {
    KeccakfSm => {
        name: "Keccakf",
        air: KECCAKF_AIR_IDS[0],
        input: KeccakfInput,
        manager: KeccakfSM<F>,
        row: KeccakfTraceRow<F>,
        row_packed: KeccakfTraceRowPacked<F>,
        chunk_size: |sm| sm.num_available_keccakfs,
    }
}
