//! Shared logic of the `jump_dest` precompile: the JUMPDEST walk, the chunk
//! compression, the fixed table of the bitmap AIR and the expansion of one
//! operation into trace rows.
//!
//! It lives here, in the leaf helpers crate, because both sides need it: the
//! emulator (`zisk-core`) to execute the opcode, and the state machine
//! (`precomp-evm`) to build the witness. Neither can depend on the other.

mod bitmap;
mod rows;
mod table;

pub use bitmap::*;
pub use rows::*;
pub use table::*;
