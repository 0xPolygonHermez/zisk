mod arith_eq;
mod arith_eq_bus_device;
mod arith_eq_constants;
mod arith_eq_input;
mod arith_eq_instance;
mod arith_eq_lt_table;
mod arith_eq_manager;
mod arith_eq_planner;
mod equations;
mod executors;
pub mod generator;
mod mem_inputs;
pub mod test_data;

pub use arith_eq::*;
pub use arith_eq_bus_device::*;
pub use arith_eq_constants::*;
pub use arith_eq_input::*;
pub use arith_eq_instance::*;
pub use arith_eq_lt_table::*;
pub use arith_eq_manager::*;
pub use arith_eq_planner::*;

// =====================================================================
// Unit-test framework marker.
// =====================================================================

use zisk_common::unit_test_sm;
use zisk_pil::{ArithEqTrace, ArithEqTraceRow, ArithEqTraceRowPacked, ARITH_EQ_AIR_IDS};

unit_test_sm! {
    ArithEqSm => {
        name: "ArithEq",
        air: ARITH_EQ_AIR_IDS[0],
        input: ArithEqInput,
        manager: ArithEqSM<F>,
        row: ArithEqTraceRow<F>,
        row_packed: ArithEqTraceRowPacked<F>,
        rows_per_input: ARITH_EQ_ROWS_BY_OP,
        chunk_size: |_| ArithEqTrace::<usize>::NUM_ROWS / ARITH_EQ_ROWS_BY_OP,
    }
}
