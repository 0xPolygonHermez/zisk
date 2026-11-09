# ZisK Pilfflonk verification on Solana

A verifier for the BLAKE3 leaf profile from ZisK v1.4.0-alpha
(`64a1f1664184e76053686745d50c6b9aaa51b034`). It does **not** accept standard
snarkjs FFLONK or PLONK proofs. The verification key and recursion root are fixed;
the caller binds its expected guest key and all 512 output bytes.
The [profile specification](docs/profile.md) records the pinned inputs,
statement encoding and cryptographic acceptance requirements.

## Components and guarantees

- `verifier/`: strict scalar/point parsing, SHA-256 statement binding, exact
  Keccak transcript, checked inverses, polynomial arithmetic and native BN254
  operations. JS intermediate traces, arkworks arithmetic and Solidity verdicts
  provide independent comparisons.
- `tools/src/generate.rs`: deterministic translation of six stages from one
  SHA-256-pinned Solidity verifier; unsupported source/syntax fails closed.
  [Scalar arithmetic notes](docs/scalar.md) explain the optimized backend's bounds.

## Reference fixtures

`tools/prepare-fixtures.sh` verifies the published Proofman crate checksum,
installs locked JS dependencies, extracts the pinned fixture, records JS reference
values and runs the unchanged Solidity verifier. Requires curl, shasum and tar
in addition to the tools above. The checked-in JSON is reproducible output.

The official macOS exporter cannot create a BN128 transcript on arm64; its Linux
counterpart requires CPU instructions unavailable under this machine's emulation.
`tools/extract-fixture` therefore projects the release's Serde/bincode schema for
**one SHA-256-pinned test container only**. It checks the key, root, public layout
and byte-identical reserialization. Acceptance by the unchanged JS and EVM
verifiers checks the resulting encoding. It is not a general production decoder.

`fixtures/blake3.json` contains the 6,752-byte proof and exact committed output
encoding. `vkey.json`, `proof.json` and `publics.json` are JS oracle inputs;
`trace.json` contains challenges, polynomial values and group accumulations.
The original container remains in `../zisk-contracts/test/blake3_pilfflonk.bin`.
Only this one independent real proof is currently available; mutations and
cross-implementation checks do not replace the additional-proof corpus gate.
