// SPDX-License-Identifier: AGPL-3.0
pragma solidity ^0.8.20;

/// @title Zisk Verifier Interface
/// @author SilentSig
/// @notice This contract is the interface for the Zisk Verifier.
/// @dev `verifySnarkProof` is the only ZisK-binding entry point. The deployed
/// `ZiskVerifier` also carries `verifyProof`, the inherited raw PLONK verifier -- public
/// because its assembly reads arguments at fixed calldata offsets -- which proves only
/// that some proof satisfies some digest, with no ZisK statement attached. Note the names
/// invert the usual convention: the raw one is the shorter.
interface IZiskVerifier {
    /// @notice Verifies a proof with given public values and vkey.
    /// @dev `programVK` MUST be a constant of the calling contract, never a value taken
    /// from the submitted proof, or verification is self-keyed. For an aggregated
    /// proof it is the recurser's verkey, and pinning it is what extends that
    /// recurser's leaf allow-list over the whole fold tree. The recursion root is not a
    /// parameter: it belongs to the ZisK setup, and the verifier holds its own.
    /// @param programVK The identity committed by the proof: the ROM root of the
    /// RISC-V program for a plain proof, the recursion domain for an aggregated one.
    /// @param publicValues The public values encoded as bytes.
    /// @param proofBytes The proof of the program execution the Zisk zkVM encoded as bytes.
    function verifySnarkProof(
        bytes32 programVK,
        bytes calldata publicValues,
        bytes calldata proofBytes
    ) external view;
}
