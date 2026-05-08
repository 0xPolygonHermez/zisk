mod arith_eq_384;
mod arith_eq_384_bus_device;
mod arith_eq_384_constants;
mod arith_eq_384_input;
mod arith_eq_384_instance;
mod arith_eq_384_manager;
mod arith_eq_384_planner;
mod equations;
mod executors;
mod mem_inputs;
pub mod test_data;

pub use arith_eq_384::*;
pub use arith_eq_384_bus_device::*;
pub use arith_eq_384_constants::*;
pub use arith_eq_384_input::*;
pub use arith_eq_384_instance::*;
pub use arith_eq_384_manager::*;
pub use arith_eq_384_planner::*;

// =====================================================================
// Unit-test framework marker.
// =====================================================================

use zisk_common::unit_test_sm;
use zisk_pil::{
    ArithEq384Trace, ArithEq384TraceRow, ArithEq384TraceRowPacked, ARITH_EQ_384_AIR_IDS,
};

unit_test_sm! {
    ArithEq384Sm => {
        name: "ArithEq384",
        air: ARITH_EQ_384_AIR_IDS[0],
        input: ArithEq384Input,
        manager: ArithEq384SM<F>,
        row: ArithEq384TraceRow<F>,
        row_packed: ArithEq384TraceRowPacked<F>,
        rows_per_input: ARITH_EQ_384_ROWS_BY_OP,
        chunk_size: |_| ArithEq384Trace::<usize>::NUM_ROWS / ARITH_EQ_384_ROWS_BY_OP - 1,
    }
}
