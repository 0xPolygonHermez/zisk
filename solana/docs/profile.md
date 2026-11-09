# Supported Pilfflonk profile

This verifier accepts the BLAKE3 leaf Pilfflonk profile from ZisK v1.4.0-alpha,
commit `64a1f1664184e76053686745d50c6b9aaa51b034`. The SNARK verification key and
final recursion root are fixed. Standard snarkjs FFLONK, PLONK, other hash
families and aggregation roots require separate implementations and fixtures.

## Reference inputs

All paths below are relative to the repository root.

| Input | SHA-256 |
|---|---|
| `zisk-contracts/blake3/ZiskVerifier.sol` | `7ef30d3cd7c5601d3416adb63f1d86b2fc5f8844de606d92723d6001c4b70cd4` |
| `zisk-contracts/blake3/PilfflonkVerifier.sol` | `a531f5bed7c5e8c85f0ab96e54706d718c42c003cc3c1c900f0eb118d9a70155` |
| `zisk-contracts/test/blake3_pilfflonk.bin` | `002e1c53bd52a05f733835d10d12b7506e215e33e9be08e4d5c210245596ad8b` |
| Published `proofman-pilfflonk` 1.4.0-alpha crate | `f7c0a2aeb1ddd06d7b8aa4c786eea4cb4f82bdf7e02c1ca75259aa2570244cf4` |

The key digest is
`0xe9eb2c78fc2cff7c38c459927e3afd00189a06af869a597c8109240ab0aedd3a`.
The trusted leaf root, as four u64 values, is:

```
[17146226293136250724, 16833344589786796689,
 10321050178608571017, 2095991909911874349]
```

`tools/extract-fixture` accepts only the pinned release container. It projects
the release's Serde schema, checks byte-identical prefix reserialization, and
checks the key, root and public layout. This bounded fixture extractor is not a
general proof decoder. The local macOS exporter lacks the BN128 backend, and
the Linux exporter requires instructions unavailable under local emulation.

From `solana/`, run `bash tools/prepare-fixtures.sh` to reproduce the JSON,
check the unchanged Proofman JavaScript verifier and run the unchanged Solidity
verifier. `tools/reference.mjs` records the statement digest, challenges,
quotient values and group accumulations for differential tests. The JSON is
marked generated for GitHub's diff display; its extraction and reference checks
remain part of the review surface.

## Statement and encoding

The caller supplies its expected 32-byte guest program key and all 512 output
bytes. The single scalar public input is:

```
BE_integer(SHA256(programVK || publicValues || rootCVadcopFinal)) mod r
```

Here `r` is the BN254 scalar modulus. The guest key and root each contain four
big-endian u64 limbs. The outputs contain 64 little-endian u64 Goldilocks field
elements; their high bits are part of the statement. Guest and output limbs must
be canonical Goldilocks elements. A key or root supplied by the proof cannot
replace the caller's expected key or the fixed recursion root.

The proof payload is exactly 6,752 bytes: nine affine G1 points, each encoded as
two big-endian 32-byte coordinates, followed by 193 big-endian scalar words.
The 520,754-byte release container also carries metadata and is not this payload.

## Acceptance requirements

- Reject incorrect lengths and noncanonical proof scalars or point coordinates.
  SHA-256 and Keccak digests are reduced modulo `r`; proof scalars are not.
- Validate untrusted G1 points on curve and reject infinity. Preserve the
  reference's coordinate lower bound of 2^192 for the first eight points.
- Match the reference transcript order, encodings and challenge reduction.
  Verify supplied inverses and reject the reference's exceptional denominators.
- Validate the fixed G2 key's canonical encoding, subgroup and nonidentity in
  tests. The proof does not supply G2 points.
- Match the reference pairing equation and EIP-197 coefficient order:
  `e(F - E - J + y W', G2) * e(-W', X2) = 1`. Propagate curve/syscall failures and
  require the pairing result to be true.
- Bind every verification to the caller's expected guest key and complete output
  statement. A consuming application must authenticate the verifier program and
  apply its own output authorization and freshness rules before acting.

These requirements inherit the ZisK proof system and trusted setup assumptions.
Only one independently generated real proof is present. Mutations and reference
agreement do not replace broader positive proof coverage or an external
cryptographic audit. Deployment also requires a reviewed setup provenance,
upgrade policy and target cluster feature set.
