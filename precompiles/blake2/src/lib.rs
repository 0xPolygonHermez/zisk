mod blake2;
mod blake2_constants;
mod blake2_mem_inputs;

pub use blake2::*;
pub use blake2_constants::*;

zisk_common::zisk_precompile! {
    name = Blake2,
    op_type = Blake2,
    trace = Blake2brTrace,
    num_available_field = num_available_blake2s,
    ops = [
        (OperationBlake2Data, Blake2Input),
    ],
}
