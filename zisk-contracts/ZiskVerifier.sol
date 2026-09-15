// SPDX-License-Identifier: AGPL-3.0
pragma solidity ^0.8.20;

import {IZiskVerifier} from "./IZiskVerifier.sol";
import {PlonkVerifier} from "./PlonkVerifier.sol";

/// @title Zisk Verifier
/// @author SilentSig
/// @notice This contracts implements a solidity verifier for Zisk.
contract ZiskVerifier is PlonkVerifier, IZiskVerifier {
    error InvalidProof();

    function VERSION() external pure returns (string memory) {
        return "v1.3.0-alpha";
    }

    /// @notice Root of the ZisK recursion setup this verifier was generated for.
    /// @dev A property of the ZisK version (see VERSION), not of the deploying
    /// application, so it is fixed here rather than accepted from the caller.
    /// Pre-packed to match the original uint64[4] layout.
    function _rootCVadcopFinal() internal pure returns (bytes32) {
        return bytes32(
            abi.encodePacked(
                uint64(6218392583875695404),
                uint64(9353885302538021251),
                uint64(8280779842059074605),
                uint64(10684020678174455855)));
    }

    /// @notice Root constant as bytes32 (pre-packed to match the original uint64[4] layout)
    function getRootCVadcopFinal() external pure returns (bytes32) {
        return _rootCVadcopFinal();
    }

    uint256 internal constant _RFIELD =
        21888242871839275222246405745257275088548364400416034343698204186575808495617;

    /// @notice Hashes the public values into a field element inside BN254.
    /// @dev Binds the fixed recursion root, so a digest this returns is one the
    /// verifier can actually accept.
    function hashPublicValues(
        bytes32 programVK,
        bytes calldata publicValues
    ) public pure returns (uint256) {
        return uint256(
            sha256(abi.encodePacked(programVK, publicValues, _rootCVadcopFinal()))
        ) % _RFIELD;
    }

    /// @notice Verifies a proof with given public values and vkey.
    function verifySnarkProof(
        bytes32 programVK,
        bytes calldata publicValues,
        bytes calldata proofBytes
    ) external view {
        uint256 publicValuesDigest = hashPublicValues(programVK, publicValues);

        uint256[24] memory proofDecoded = abi.decode(proofBytes, (uint256[24]));

        bool success = this.verifyProof(proofDecoded, [publicValuesDigest]);

        if (!success) {
            revert InvalidProof();
        }
    }
}
