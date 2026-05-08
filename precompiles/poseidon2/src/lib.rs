mod poseidon2;
mod poseidon2_bus_device;
mod poseidon2_gen_mem_inputs;
mod poseidon2_input;
mod poseidon2_instance;
mod poseidon2_manager;
mod poseidon2_planner;

pub use poseidon2::*;
pub use poseidon2_bus_device::*;
pub use poseidon2_gen_mem_inputs::*;
pub use poseidon2_input::*;
pub use poseidon2_instance::*;
pub use poseidon2_manager::*;
pub use poseidon2_planner::*;

// =====================================================================
// Unit-test framework marker.
// =====================================================================

use zisk_common::unit_test_sm;
use zisk_pil::{Poseidon2TraceRow, Poseidon2TraceRowPacked, POSEIDON_2_AIR_IDS};

unit_test_sm! {
    Poseidon2Sm => {
        name: "Poseidon2",
        air: POSEIDON_2_AIR_IDS[0],
        input: Poseidon2Input,
        manager: Poseidon2SM<F>,
        row: Poseidon2TraceRow<F>,
        row_packed: Poseidon2TraceRowPacked<F>,
        rows_per_input: CLOCKS,
        chunk_size: |sm| sm.num_available_poseidon2s,
    }
}
