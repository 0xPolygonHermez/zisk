mod add256;
mod add256_constants;
mod add256_mem_inputs;

pub use add256::*;
pub use add256_constants::*;

zisk_common::zisk_precompile! {
    name = Add256,
    op_type = BigInt,
    trace = Add256Trace,
    num_available_field = num_availables,
    ops = [
        (OperationAdd256Data, Add256Input),
    ],
}
