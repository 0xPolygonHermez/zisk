// SPDX-License-Identifier: AGPL-3.0
pragma solidity ^0.8.20;

/// @title Zisk Verifier Interface
/// @author SilentSig
/// @notice This contract is the interface for the Zisk Verifier.
interface IZiskVerifier {
    /// @notice Verifies a proof with given public values and vkey.
    /// @dev Both keys MUST be constants of the calling contract, never values taken
    /// from the submitted proof, or verification is self-keyed. For an aggregated
    /// proof both are the recurser's verkey, and pinning `programVK` is what
    /// extends its leaf allow-list over the whole fold tree.
    /// @param programVK The identity committed by the proof: the ROM root of the
    /// RISC-V program for a plain proof, the recursion domain for an aggregated one.
    /// @param publicValues The public values encoded as bytes.
    /// @param proofBytes The proof of the program execution the Zisk zkVM encoded as bytes.
    function verifySnarkProof(
        bytes32 programVK,
        bytes32 rootCVadcopFinal,
        bytes calldata publicValues,
        bytes calldata proofBytes
    ) external view;
}
