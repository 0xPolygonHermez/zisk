mod sha256f;
mod sha256f_bus_device;
mod sha256f_constants;
mod sha256f_gen_mem_inputs;
mod sha256f_input;
mod sha256f_instance;
mod sha256f_manager;
mod sha256f_planner;

pub use sha256f::*;
pub use sha256f_bus_device::*;
pub use sha256f_constants::*;
pub use sha256f_gen_mem_inputs::*;
pub use sha256f_input::*;
pub use sha256f_instance::*;
pub use sha256f_manager::*;
pub use sha256f_planner::*;

// =====================================================================
// Unit-test framework marker.
// =====================================================================

use zisk_common::unit_test_sm;
use zisk_pil::{Sha256fTraceRow, Sha256fTraceRowPacked, SHA_256_F_AIR_IDS};

unit_test_sm! {
    Sha256fSm => {
        name: "Sha256f",
        air: SHA_256_F_AIR_IDS[0],
        input: Sha256fInput,
        manager: Sha256fSM<F>,
        row: Sha256fTraceRow<F>,
        row_packed: Sha256fTraceRowPacked<F>,
        rows_per_input: CLOCKS,
        chunk_size: |sm| sm.num_available_sha256fs,
    }
}
