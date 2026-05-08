mod blake2;
mod blake2_bus_device;
mod blake2_constants;
mod blake2_gen_mem_inputs;
mod blake2_input;
mod blake2_instance;
mod blake2_manager;
mod blake2_planner;

pub use blake2::*;
pub use blake2_bus_device::*;
pub use blake2_constants::*;
pub use blake2_gen_mem_inputs::*;
pub use blake2_input::*;
pub use blake2_instance::*;
pub use blake2_manager::*;
pub use blake2_planner::*;

// =====================================================================
// Unit-test framework marker.
// =====================================================================

use zisk_common::unit_test_sm;
use zisk_pil::{Blake2brTraceRow, Blake2brTraceRowPacked, BLAKE_2_BR_AIR_IDS};

unit_test_sm! {
    Blake2Sm => {
        name: "Blake2",
        air: BLAKE_2_BR_AIR_IDS[0],
        input: Blake2Input,
        manager: Blake2SM<F>,
        row: Blake2brTraceRow<F>,
        row_packed: Blake2brTraceRowPacked<F>,
        rows_per_input: CLOCKS,
        chunk_size: |sm| sm.num_available_blake2s,
    }
}
