// #![deny(missing_docs)]
mod binary;
mod binary_add;
mod binary_add_collector;
mod binary_add_instance;
mod binary_basic;
mod binary_basic_collector;
mod binary_basic_frops;
mod binary_basic_instance;
mod binary_basic_table;
mod binary_constants;
mod binary_counter;
mod binary_extension;
mod binary_extension_collector;
mod binary_extension_frops;
mod binary_extension_instance;
mod binary_extension_table;
mod binary_input;
mod binary_planner;

pub use binary::*;
use binary_add::*;
pub use binary_add_collector::*;
pub use binary_add_instance::*;
use binary_basic::*;
pub use binary_basic_collector::*;
pub use binary_basic_frops::*;
pub use binary_basic_instance::*;
use binary_basic_table::*;
pub use binary_constants::*;
pub use binary_counter::*;
use binary_extension::*;
pub use binary_extension_collector::*;
pub use binary_extension_frops::*;
pub use binary_extension_instance::*;
use binary_extension_table::*;
pub use binary_input::*;
use binary_planner::*;
// =====================================================================
// Unit-test framework markers. One marker per AIR id; all use the
// inner SM directly as `Manager` so the macro can call
// `sm.compute_witness(...)` without needing an accessor name.
// =====================================================================

use zisk_common::unit_test_sm;
use zisk_pil::{
    BinaryAddTrace, BinaryAddTraceRow, BinaryAddTraceRowPacked, BinaryExtensionTrace,
    BinaryExtensionTraceRow, BinaryExtensionTraceRowPacked, BinaryTrace, BinaryTraceRow,
    BinaryTraceRowPacked, BINARY_ADD_AIR_IDS, BINARY_AIR_IDS, BINARY_EXTENSION_AIR_IDS,
};

unit_test_sm! {
    BinarySm => {
        name: "Binary",
        air: BINARY_AIR_IDS[0],
        input: BinaryInput,
        manager: BinaryBasicSM<F>,
        row: BinaryTraceRow<F>,
        row_packed: BinaryTraceRowPacked<F>,
        chunk_size: |_| BinaryTrace::<usize>::NUM_ROWS,
    }
}

unit_test_sm! {
    BinaryAddSm => {
        name: "BinaryAdd",
        air: BINARY_ADD_AIR_IDS[0],
        input: [u64; 2],
        manager: BinaryAddSM<F>,
        row: BinaryAddTraceRow<F>,
        row_packed: BinaryAddTraceRowPacked<F>,
        chunk_size: |_| BinaryAddTrace::<usize>::NUM_ROWS,
    }
}

unit_test_sm! {
    BinaryExtensionSm => {
        name: "BinaryExtension",
        air: BINARY_EXTENSION_AIR_IDS[0],
        input: BinaryInput,
        manager: BinaryExtensionSM<F>,
        row: BinaryExtensionTraceRow<F>,
        row_packed: BinaryExtensionTraceRowPacked<F>,
        chunk_size: |_| BinaryExtensionTrace::<usize>::NUM_ROWS,
    }
}
