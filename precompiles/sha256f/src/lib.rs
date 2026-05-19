mod sha256f;
mod sha256f_constants;
mod sha256f_mem_inputs;

pub use sha256f::*;
pub use sha256f_constants::*;

zisk_common::zisk_precompile! {
    name = Sha256f,
    op_type = Sha256,
    trace = Sha256fTrace,
    num_available_field = num_available_sha256fs,
    ops = [
        (OperationSha256Data, Sha256fInput),
    ],
}
