mod keccakf;
mod keccakf_constants;
mod keccakf_expr_generator;
mod keccakf_mem_inputs;
mod keccakf_table;

pub use keccakf::*;
use keccakf_constants::*;
pub use keccakf_expr_generator::*;
use keccakf_table::*;

zisk_common::zisk_precompile! {
    name = Keccakf,
    op_type = Keccak,
    trace = KeccakfTrace,
    num_available_field = num_available_keccakfs,
    ops = [
        (OperationKeccakData, KeccakfInput),
    ],
}
