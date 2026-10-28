# Final SNARK: pilfflonk

A ZisK proof is a STARK, the `VadcopFinal` proof. It is fast to produce but large, and verifying it on Ethereum would be expensive. The final SNARK wraps it in a short proof over the BN254 curve, which an EVM contract verifies with the pairing precompile.

Since 1.4.0 the final SNARK is **pilfflonk**, for every hash mode. PLONK remains available for Poseidon keys.

## Where it fits

A `--pilfflonk` proof goes through four steps:

1. **`VadcopFinal`**: the STARK proof of the execution, over Goldilocks.
2. **`recursivef`**: a STARK that verifies the `VadcopFinal` proof. For a Poseidon key its Merkle trees are over BN254; for a BLAKE3 key it stays a Goldilocks STARK hashed with BLAKE3.
3. **The final circuit**: a circom circuit that verifies the `recursivef` proof. plonk2pil turns it into a PIL2 AIR over BN254, where Poseidon, BLAKE3 and the range checks are custom gates rather than plain constraints.
4. **The pilfflonk proof** of that AIR, which `ZiskVerifier.sol` verifies on-chain.

`cargo-zisk proofman-setup setup-snark` builds the setups of steps 2 to 4 once per proving key, and `ziskup setup_snark` installs them in `$HOME/.zisk/provingKeySnark`.

## Why pilfflonk

- **Every hash mode has a final SNARK.** PLONK can only wrap a Poseidon key, whose `recursivef` is a BN254 circuit. pilfflonk verifies a BLAKE3 `recursivef` with BLAKE3 custom gates, so the BLAKE3 setup, the default, can be verified on-chain too.
- **A smaller circuit.** PLONK proves the final circuit with standard PLONK gates. pilfflonk proves it as a PIL2 AIR with custom gates, with over 20 times fewer constraints, which makes proving faster and the key smaller.
- **Faster, and much lighter on memory.** Against PLONK, the wrap is about 2.3 times faster on CPU and 1.7 times faster on GPU. The SNARK proving key is about 30 times smaller, and the prover needs about 9 times less memory on CPU and 6 times less on GPU.
- **One contract.** The on-chain verifier is a single Solidity contract.
- **No new trusted setup.** pilfflonk uses the same universal Powers of Tau files as PLONK.
- **What it costs.** The proof is almost 3 times larger than PLONK's, and verifying it on-chain costs about 26% more gas.

proofman measured these ratios on its `fibonacci-square` example with a Poseidon2 key, against PLONK on its stock final circuit. ZisK's final circuit is a different one, and its default key is BLAKE3, so ZisK's ratios differ.

## What it is based on

- **KZG commitments over BN254.** The structured reference string comes from the Hermez Powers of Tau ceremony (`powersOfTau28_hez_final_*.ptau`). It is universal: the same file serves every circuit, and no ceremony is run per circuit.
- **fflonk** ([Gabizon and Williamson, 2021](https://eprint.iacr.org/2021/1167)). It packs `k` polynomials into one, `f(X) = Σ p_j(X^k)·X^j`, so the prover commits to a few `f_i` instead of every column.
- **SHPLONK** ([Boneh, Drake, Fisch and Gabizon, 2020](https://eprint.iacr.org/2020/081)). It opens all the `f_i`, at all their points, with one proof, which the verifier checks with a single pairing check.
- **PIL2.** The constraints, the stages and the buses are those of PIL2, the same language as ZisK's STARKs, and pilfflonk shares proofman's setup passes. That is what makes custom gates possible: each gate is one more polynomial constraint of the AIR.

pilfflonk is a port to PIL2 of Polygon's [pil-fflonk](https://github.com/0xPolygonHermez/pil-fflonk). Its JS and Solidity verifiers follow snarkjs's FFLONK verifiers. The protocol, the file formats, the verifiers and the performance are documented in proofman's [`pilfflonk/docs`](https://github.com/0xPolygonHermez/pil2-proofman/tree/v1.4.0-alpha/pilfflonk/docs).

## Using it

**Install the key.** `ziskup setup_snark` installs the SNARK proving key of the installed key's hash mode. To build it from source, see [Installation](../getting_started/installation.md).

**Prove.** Ask for the final SNARK with `--pilfflonk`:

```bash
cargo-zisk prove --pilfflonk ...
cargo-zisk wrap --pilfflonk ...
cargo-zisk remote prove --pilfflonk ...
```

In the SDK, `.pilfflonk()` on the embedded or remote builder, or `.snark(SnarkProtocol::Pilfflonk)`.

**Distributed proving.** A worker started with `--plonk` and a SNARK proving key makes final SNARKs in its key's protocol, pilfflonk included: on the worker, `--plonk` turns final SNARKs on, whatever their protocol. Any worker may wrap a job, so all the workers of a cluster must share the same SNARK setup. A job that asks for a protocol the workers cannot prove is refused when it is submitted (`SNARK_UNAVAILABLE`); see the [Coordinator API](coordinator_api.md).

**Verify.** `cargo-zisk verify -p <proof>` checks a pilfflonk proof against this release's key for the proof's hash family. For a key of your own setup, pass its `pilfflonk.vkey.json` with `--pilfflonk-vk`.

**Verify on-chain.** `zisk-contracts/<family>/ZiskVerifier.sol` (on top of `PilfflonkVerifier.sol`) exposes `verifySnarkProof(programVK, rootCVadcopFinal, publicValues, proof)`. `cargo-zisk-dev export-solidity-calldata` writes its arguments from a proof file; the [zisk-contracts README](https://github.com/0xPolygonHermez/zisk/tree/v1.4.0-alpha/zisk-contracts) has a worked example.

### PLONK

PLONK needs a Poseidon key. Set it up with `cargo-zisk proofman-setup setup-snark --final-snark plonk`, prove with `--plonk`, and verify with `cargo-zisk verify --plonk-vk <final.verkey.json>`: the release publishes no PLONK key or contract. ZisK does not prove with FFLONK keys.
