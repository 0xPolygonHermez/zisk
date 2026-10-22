// SPDX-License-Identifier: AGPL-3.0
pragma solidity ^0.8.20;

import {IZiskVerifier} from "./IZiskVerifier.sol";
import {PilfflonkVerifier} from "./PilfflonkVerifier.sol";

/// @title Zisk Verifier
/// @author SilentSig
/// @notice This contracts implements a solidity verifier for Zisk.
contract ZiskVerifier is PilfflonkVerifier, IZiskVerifier {
    error InvalidProof();

    function VERSION() external pure returns (string memory) {
        return "v1.4.0-alpha";
    }

    /// @notice Root constant as bytes32 (pre-packed to match the original uint64[4] layout)
    /// @dev The root for a *leaf* proof. An aggregated proof is wrapped under its
    /// recurser's own verkey instead, so a caller verifying folds passes that one.
    /// Either way the value must be a constant of the calling contract -- see
    /// IZiskVerifier.
    function getRootCVadcopFinal() external pure returns (bytes32) {
        return bytes32(
            abi.encodePacked(
                uint64(16379923861911953426),
                uint64(16352910408364517494),
                uint64(10294560753330969230),
                uint64(16923079741480914316)));
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

        bytes32[69] memory proofDecoded = abi.decode(proofBytes, (bytes32[69]));

        bool success = this.verifyProof(proofDecoded, [publicValuesDigest]);

        if (!success) {
            revert InvalidProof();
        }
    }
}
