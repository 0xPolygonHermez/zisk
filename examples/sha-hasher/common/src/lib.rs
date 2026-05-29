use alloy_sol_types::sol;

sol! {
    /// Output structure for the SHA hasher program.
    /// Uses Solidity ABI encoding compatible with verification contracts.
    /// Shared between guest and host to ensure consistent encoding.
    struct Output {
        bytes32 hash;
        uint32 iterations;
        uint32 magic_number;
    }
}
