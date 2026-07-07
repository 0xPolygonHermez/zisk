//! Shared L2 block-info type for the `l2` example: the guest commits its
//! Solidity ABI encoding, the host reads it back. All fields are static ABI
//! types (32 bytes = 8 slots each), so 8 fields fill all 64 publics slots
//! (`n-publics-agg = 64`). The `SLOT_*` constants below are the field→slot map
//! shared with `aggregate.circom`.

use alloy_sol_types::sol;

sol! {
    /// L2 block-range settlement info, ABI-encoded into the proof publics.
    /// Mirrors the shape an L2 settlement contract would verify on-chain.
    #[derive(Debug)]
    struct BlocksInfoStruct {
        uint256 startBlock;
        uint256 endBlock;
        bytes32 globalExitRoot;
        bytes32 accountRoot;
        bytes32 depositRoot;
        bytes32 priorityExitRoot;
        uint256 oldGlobalExitRoot;
        uint256 oldAccountRoot;
    }
}

/// Each ABI field is one 32-byte word = 8 `u32` publics slots.
pub const SLOTS_PER_FIELD: usize = 8;

// First publics slot of each field; kept in sync with aggregate.circom.
pub const SLOT_START_BLOCK: usize = 0;
pub const SLOT_END_BLOCK: usize = 8;
pub const SLOT_GLOBAL_EXIT_ROOT: usize = 16;
pub const SLOT_ACCOUNT_ROOT: usize = 24;
pub const SLOT_DEPOSIT_ROOT: usize = 32;
pub const SLOT_PRIORITY_EXIT_ROOT: usize = 40;
pub const SLOT_OLD_GLOBAL_EXIT_ROOT: usize = 48;
pub const SLOT_OLD_ACCOUNT_ROOT: usize = 56;

/// Populated publics slots = the aggregation's `n-publics-agg` (all 64).
pub const N_PUBLICS_AGG: usize = 64;
