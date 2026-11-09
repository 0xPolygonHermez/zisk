# ZisK Pilfflonk reference checks

Pinned BLAKE3 leaf Pilfflonk inputs from ZisK v1.4.0-alpha establish the encoding,
statement binding and expected reference values for a Solana verifier.
[Profile and acceptance requirements](docs/profile.md) identify the trusted key,
recursion root, source hashes and unsupported profiles.

## Reproduce the references

Use Rust 1.95.0, Node 26.12.1, Foundry 1.5.1, Python 3, curl, shasum and tar.
From this directory:

```sh
bash tools/prepare-fixtures.sh
git diff --exit-code -- fixtures
```

The script checks the Proofman crate checksum, installs locked JS dependencies,
extracts the pinned release fixture, records JS intermediate values, and runs
the unchanged Solidity verifier. The extractor accepts one SHA-256-pinned
container and checks byte-identical Serde prefix reserialization.

`fixtures/blake3.json` contains the 6,752-byte proof, guest key and complete output
statement. The other JSON files contain the reference key, public input, proof
object and trace. GitHub collapses these generated files by default; review
their pinned source and extraction/reference checks before accepting changes.

Only one independently generated real proof is available. Mutations and reference
agreement do not replace additional positive proofs or an external cryptographic
audit.
