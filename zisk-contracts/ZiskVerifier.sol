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
        return "v0.17.0";
    }

    /// @notice Root constant as bytes32 (pre-packed to match the original uint64[4] layout)
    function getRootCVadcopFinal() external pure returns (bytes32) {
        return bytes32(
            abi.encodePacked(
                uint64(11766295392914223603),
                uint64(9891544556924317672),
                uint64(11057384261039105715),
                uint64(18086400189378942219)
            )
        );
    }

    uint256 internal constant _RFIELD =
        21888242871839275222246405745257275088548364400416034343698204186575808495617;

    /// @notice Hashes the public values into a field element inside BN254.
    function hashPublicValues(
        bytes32 programVK,
        bytes32 rootCVadcopFinal,
        bytes calldata publicValues
    ) public pure returns (uint256) {
        return uint256(
            sha256(abi.encodePacked(programVK, publicValues, rootCVadcopFinal))
        ) % _RFIELD;
    }

    /// @notice Verifies a proof with given public values and vkey.
    function verifySnarkProof(
        bytes32 programVK,
        bytes32 rootCVadcopFinal,
        bytes calldata publicValues,
        bytes calldata proofBytes
    ) external view {
        uint256 publicValuesDigest = hashPublicValues(programVK, rootCVadcopFinal, publicValues);

        uint256[24] memory proofDecoded = abi.decode(proofBytes, (uint256[24]));

        bool success = this.verifyProof(proofDecoded, [publicValuesDigest]);

        if (!success) {
            revert InvalidProof();
        }
    }
}