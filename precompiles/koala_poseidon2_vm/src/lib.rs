//! KoalaBear Poseidon2 precompile state machine: witness, memory tuples and planner registration.

mod memory;
mod witness;

pub use witness::{KoalaPoseidon2Input, KoalaPoseidon2SM, CLOCKS};

zisk_common::zisk_precompile! {
    name = KoalaPoseidon2,
    op_type = KoalaPoseidon2,
    trace = KoalaPoseidon2Trace,
    num_available = {
        ::zisk_pil::KoalaPoseidon2Trace::<::zisk_pil::KoalaPoseidon2TraceRow<F>>::NUM_ROWS / CLOCKS
    },
    ops = [(OperationKoalaPoseidon2Data, KoalaPoseidon2Input)],
}
