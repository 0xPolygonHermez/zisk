// SPDX-License-Identifier: MIT OR Apache-2.0
pragma solidity ^0.8.20;
import {ZiskVerifier} from "zisk/ZiskVerifier.sol";

interface Vm {
    function readFile(string calldata) external view returns (string memory);
    function parseJsonBytes(string calldata, string calldata) external pure returns (bytes memory);
    function parseJsonBytes32(string calldata, string calldata) external pure returns (bytes32);
}

contract ReferenceTest {
    Vm constant vm = Vm(address(uint160(uint256(keccak256("hevm cheat code")))));
    ZiskVerifier verifier;
    bytes32 guest;
    bytes32 root;
    bytes outputs;
    bytes proof;

    function setUp() public {
        verifier = new ZiskVerifier();
        string memory json = vm.readFile("fixtures/blake3.json");
        guest = vm.parseJsonBytes32(json, ".programVK");
        root = vm.parseJsonBytes32(json, ".rootCVadcopFinal");
        outputs = vm.parseJsonBytes(json, ".publicValues");
        proof = vm.parseJsonBytes(json, ".proofBytes");
    }

    function testValidReleaseProof() public view {
        require(root == verifier.getRootCVadcopFinal(), "wrong leaf root");
        verifier.verifySnarkProof(guest, root, outputs, proof);
    }

    function testRejectChangedOutput() public {
        outputs[7] ^= 0x80;
        (bool ok,) = address(verifier).staticcall(abi.encodeCall(verifier.verifySnarkProof, (guest, root, outputs, proof)));
        require(!ok, "accepted altered high output bits");
    }

    function testRejectEveryChangedProofWord() public view {
        bytes memory changed = proof;
        for (uint256 i; i < changed.length; i += 32) {
            changed[i + 31] ^= 0x01;
            (bool ok,) = address(verifier).staticcall(abi.encodeCall(verifier.verifySnarkProof, (guest, root, outputs, changed)));
            require(!ok, "accepted altered proof word");
            changed[i + 31] ^= 0x01;
        }
    }
}
