//! # Zero-Knowledge Proving
//!
//! * Proving a program execution using zero-knowledge has the following phases:
//!   * Witness Computation
//!     * Executes the program
//!     * Plans the number and size of the secondary state machines instances required to contain
//!       all the operations
//!     * Generates field element traces of the execution for all the required state machine
//!       instances
//!   * Proof Generation
//!     * Generates individual proofs for every state machine instance
//!     * Aggregates individual proofs into aggregated proofs, recursively
//!     * Generates final proof
//!   * Proof Verification
//!     * Authenticates the final proof
//!
//! # Proof Delegation
//!
//! * The Zisk main state machine processes the input data using the Zisk program and generates the
//!   output data.
//! * This process performs some simple operations that can be proven by the main state machine
//!   itself, and also some more complex operations that must be delegated to other specialized
//!   state machines that can prove them more efficiently.
//! * These secondary state machines can be composed of several inner state machines that are more
//!   specialized in proving some specific operations, again for efficiency reasons.
//! * The proof delegation between the different state machines requires that the client state
//!   machines provide the required data to the server state machines to proof their operations.
//! * The required data depends on the type of proof delegation.
//! * Some secondary machines that are made of several, more specialized state machines, include a
//!   state machine proxy to dispatch incoming required data and distribute it among the rest.
//! * The executor is the component in charge of calling the different state machines, in the right
//!   order, collecting the required data from ones and providing it to the others, parallelizing
//!   when possible.
//! * In order to parallelize this process as much and as soon as possible, a first execution of the
//!   program is done, collecting the minimum trace required to split the execution in smaller parts
//!   that can be re-executed again in parallel, and this time generating more information that will
//!   feed the secondary state machines.
//!
//! # State machines map
//!
//! * Main
//!   * Binary Proxy
//!     * Binary Basic --> Binary Basic Table
//!     * Binary Extension --> Binary Extension Table
//!   * Rom
//!   * Arith
//!   * Memory Proxy
//!     * Memory Aligned
//!     * Memory Unaligned
//!     * Memory Input
//!
//! The zisk_core crate contains basic structures and functionality used by several other modules:
//! opcodes, instructions and transpilation
pub mod elf2rom;
pub mod elf_extraction;
pub mod fcall;
pub mod helpers;
pub mod inst_context;
pub mod mem;
pub mod riscv2zisk;
pub mod riscv2zisk_context;
mod utils;
pub mod zisk_definitions;
pub mod zisk_inst;
pub mod zisk_inst_builder;
pub mod zisk_ops;
pub mod zisk_registers;
pub mod zisk_required_operation;
pub mod zisk_rom;
pub mod zisk_rom_2_asm;

pub use elf2rom::*;
pub use fcall::*;
pub use helpers::*;
pub use inst_context::*;
pub use mem::*;
pub use riscv2zisk::*;
pub use riscv2zisk_context::*;
pub use utils::*;
pub use zisk_definitions::*;
pub use zisk_inst::*;
pub use zisk_inst_builder::*;
pub use zisk_registers::*;
pub use zisk_required_operation::*;
pub use zisk_rom::*;
pub use zisk_rom_2_asm::*;
