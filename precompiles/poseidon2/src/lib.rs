mod poseidon2;
mod poseidon2_mem_inputs;

pub use poseidon2::*;

zisk_common::zisk_precompile! {
    name = Poseidon2,
    op_type = Poseidon2,
    trace = Poseidon2Trace,
    num_available_field = num_available_poseidon2s,
    ops = [
        (OperationPoseidon2Data, Poseidon2Input),
    ],
}
