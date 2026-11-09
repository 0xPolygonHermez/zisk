# ZisK Pilfflonk verification library

The Rust library verifies the fixed ZisK v1.4.0-alpha BLAKE3 leaf profile.
The caller supplies its expected 32-byte guest key and all 512 output bytes.
The SNARK key and recursion root are fixed. Standard snarkjs FFLONK, PLONK and
other keys or roots are outside this implementation.

## Review the verifier

- [Profile](docs/profile.md): trusted inputs, encodings and acceptance requirements.
- `verifier/src/lib.rs`: strict decoding, statement digest, transcript and verification flow.
- [Scalar arithmetic](docs/scalar.md): representation, multiplication bounds and equivalence checks.
- `tools/generate_profile.py`: six scalar stages translated from one pinned Solidity source.
- `verifier/src/curve.rs`: point checks, native curve operations and pairing encoding.
- `verifier/tests/reference.rs`: intermediate-value and adversarial proof comparisons.
- `fuzz/fuzz_targets/verify.rs`: arithmetic equivalence and proof decoding fuzz target.

## Check this implementation

Use Rust 1.95.0, Node 26.12.1, Foundry 1.5.1 and Python 3. From this directory:

```sh
bash tools/prepare-fixtures.sh
git diff --exit-code -- fixtures
python3 tools/generate_profile.py --check
python3 -m unittest discover -s tools -p 'test_*.py'
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets --features trace -- -D warnings
cargo test --locked -p zisk-solana-verifier --features trace

cargo install cargo-fuzz --version 0.13.1 --locked --root .cache/fuzz-tools
rustup toolchain install nightly-2026-02-12 --profile minimal
bash scripts/fuzz.sh 60
```

The fuzz script reconstructs the public release-proof seed and retains discovered
inputs in ignored `fuzz/corpus/`. Arithmetic tests compare with arkworks; verifier
tests compare with the unchanged JS/Solidity references. This is evidence for one
real proof and its mutations, not an external audit or broad positive proof coverage.
