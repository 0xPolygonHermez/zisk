//! Data required to prove the different Zisk operations
//!
//! # Zero-Knowledge Proving
//!
//! * Proving a program execution using zero-knowledge has the following phases:
//!   * Witness Computation
//!     * Executes the program
//!     * Plans the number and size of the secondary state machines instances required to contain
//! all the operations
//!     * Generates field element traces of the execution for all the required state machine
//! instances
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
//! output data.
//! * This process performs some simple operations that can be proven by the main state machine
//! itself, and also some more complex operations that must be delegated to other specialized state
//! machines that can prove them more efficiently.
//! * These secondary state machines can be composed of several inner state machines that are more
//! specialized in proving some specific operations, again for efficiency reasons.
//! * The proof delegation between the different state machines requires that the client state
//! machines provide the required data to the server state machines to proof their operations.
//! * The required data depends on the type of proof delegation.
//! * Some secondary machines that are made of several, more specialized state machines, include a
//! state machine proxy to dispatch incoming required data and distribute it among the rest.
//! * The executor is the component in charge of calling the different state machines, in the right
//! order, collecting the required data from ones and providing it to the others, parallelizing when
//! possible.
//! * In order to parallelize this process as much and as soon as possible, a first execution of the
//! program is done, collecting the minimum trace required to split the execution in smaller parts
//! that can be re-executed again in parallel, and this time generating more information that will
//! feed the secondary state machines.
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

use std::collections::HashMap;

/// Stores the minimum information to reproduce an operation execution:
/// the opcode and the a and b registers values (regardless of their sources);
/// the step is also stored to keep track of the program execution point.
/// This data is generated during the first emulation execution.
/// This data is required by the main state machine executor to generate the witness computation.
#[derive(Clone)]
pub struct ZiskRequiredOperation {
    pub step: u64,
    pub opcode: u8,
    pub a: u64,
    pub b: u64,
}

/// Stores the minimum information to generate the memory state machine witness computation.
#[derive(Clone)]
pub struct ZiskRequiredMemory {
    pub step: u64,
    pub is_write: bool,
    pub address: u64,
    pub width: u64,
    pub value: u64,
}

/// Operations required to be proven
#[derive(Clone, Default)]
pub struct ZiskRequired {
    pub arith: Vec<ZiskRequiredOperation>,
    pub binary: Vec<ZiskRequiredOperation>,
    pub binary_extension: Vec<ZiskRequiredOperation>,
    pub memory: Vec<ZiskRequiredMemory>,
}

/// Histogram of the program counter values used during the program execution.
/// Each pc value has a u64 counter, associated to it via a hash map.
/// The counter is increased every time the corresponding instruction is executed.
#[derive(Clone, Default)]
pub struct ZiskPcHistogram {
    pub map: HashMap<u64, u64>,
    pub end_pc: u64,
    pub steps: u64,
}
