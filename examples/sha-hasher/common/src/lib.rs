use alloy_sol_types::sol;

sol! {
    /// Output structure for the SHA hasher program.
    /// This uses Solidity ABI encoding compatible with the verification contracts.
    ///
    /// This struct is shared between guest and host to ensure consistent encoding.
    struct Output {
        bytes32 hash;
        uint32 iterations;
        uint32 magic_number;
    }
}
