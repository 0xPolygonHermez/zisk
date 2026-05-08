mod arith;
mod arith_bus_device;
mod arith_frops;
mod arith_full;
mod arith_full_instance;
mod arith_operation;
mod arith_planner;
mod arith_range_table;
mod arith_range_table_helpers;
mod arith_table;
mod arith_table_data;
mod arith_table_helpers;

pub use arith::*;
pub use arith_bus_device::*;
pub use arith_frops::*;
use arith_full::*;
pub use arith_full_instance::*;
use arith_operation::*;
use arith_planner::*;
use arith_range_table::*;
use arith_range_table_helpers::*;
use arith_table::*;
use arith_table_data::*;
use arith_table_helpers::*;

#[cfg(test)]
mod arith_operation_test;

// =====================================================================
// Unit-test framework marker.
// =====================================================================

use zisk_common::{unit_test_sm, OperationData};
use zisk_pil::{ArithTrace, ArithTraceRow, ArithTraceRowPacked, ARITH_AIR_IDS};

unit_test_sm! {
    ArithSm => {
        name: "Arith",
        air: ARITH_AIR_IDS[0],
        input: OperationData<u64>,
        manager: ArithFullSM<F>,
        row: ArithTraceRow<F>,
        row_packed: ArithTraceRowPacked<F>,
        chunk_size: |_| ArithTrace::<usize>::NUM_ROWS,
    }
}
