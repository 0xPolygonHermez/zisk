mod add256;
mod add256_bus_device;
mod add256_constants;
mod add256_gen_mem_inputs;
mod add256_input;
mod add256_instance;
mod add256_manager;
mod add256_planner;

pub use add256::*;
pub use add256_bus_device::*;
pub use add256_constants::*;
pub use add256_gen_mem_inputs::*;
pub use add256_input::*;
pub use add256_instance::*;
pub use add256_manager::*;
pub use add256_planner::*;

// =====================================================================
// Unit-test framework marker.
// =====================================================================

use zisk_common::unit_test_sm;
use zisk_pil::{Add256Trace, Add256TraceRow, Add256TraceRowPacked, ADD_256_AIR_IDS};

unit_test_sm! {
    Add256Sm => {
        name: "Add256",
        air: ADD_256_AIR_IDS[0],
        input: Add256Input,
        manager: Add256SM<F>,
        row: Add256TraceRow<F>,
        row_packed: Add256TraceRowPacked<F>,
        chunk_size: |_| Add256Trace::<usize>::NUM_ROWS,
    }
}
