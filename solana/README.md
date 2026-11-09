# ZisK Pilfflonk verification on Solana

A verifier for the BLAKE3 leaf profile from ZisK v1.4.0-alpha
(`64a1f1664184e76053686745d50c6b9aaa51b034`). It does **not** accept standard
snarkjs FFLONK or PLONK proofs. The verification key and recursion root are fixed;
the caller binds its expected guest key and all 512 output bytes.
The [profile specification](docs/profile.md) records the pinned inputs,
statement encoding and cryptographic acceptance requirements.

Compiled SBF verification and authenticated consumer CPI pass in LiteSVM with the
ordinary 1,400,000-CU transaction limit and default syscall pricing:

| Path | CUs |
|---|---:|
| Valid proof | 1,385,858 |
| Invalid final opening, rejected at pairing | 1,385,885 |
| Consumer verification plus receipt write | 1,388,559 |
| Consumer rejecting invalid proof | 1,388,017 |

**The 1,000,000-CU target is not met.** The consuming example has only 11,441 CUs
of headroom. These measurements cover one real release proof and the pinned
local runtime, not a public-cluster deployment or general application workloads.
More independent proofs and an external cryptographic audit are still required
before production use. Agent reviews do not replace that audit.

## Reproduce

Tested tools: Rust 1.95.0, Node 26.12.1, Foundry 1.5.1, Python 3, and
`cargo-build-sbf` 4.6.0. SBF builds explicitly use platform-tools v1.57 and SBPF v3.
The host runtime uses LiteSVM 0.16.0 / Agave 4.2.2 with its default feature set.
The local `rust-toolchain.toml` pins the host compiler and lint/format components.

From this directory:

```sh
rustup component add clippy rustfmt
cargo install cargo-build-sbf --version 4.6.0 --locked --root .cache/build-tools
bash scripts/check.sh

cargo install cargo-fuzz --version 0.13.1 --locked --root .cache/fuzz-tools
rustup toolchain install nightly-2026-02-12 --profile minimal
bash scripts/fuzz.sh 60
```

If the builder installer reports a shadowing Solana CLI, put `$HOME/.cargo/bin`
first in PATH for that installation command. The build script invokes the
installed task-local binary directly. It also rejects LLVM stack-frame errors
that may otherwise leave `cargo-build-sbf` with a successful exit code.

`PROFILE_CUS=1 bash scripts/build-sbf.sh` enables stage logs for measurement.
Rebuild without that variable for release measurements. No enlarged compute
budget is used by the runtime tests.

`scripts/check.sh` regenerates fixtures, checks the generator, formatting and Clippy, runs
the independent references and host/client tests, builds SBF, and runs the runtime
suite. Its benchmark check rejects missing measurements, packet-limit violations,
transactions reaching 1.4M CUs, and CU increases over 5%. Reports go in `.cache/`.
The CI workflow runs these same commands and publishes runtime and fuzz logs.

The fuzz script runs each target for the supplied number of seconds. It rebuilds
public starting seeds and retains discovered inputs in ignored `fuzz/corpus/`.
The [validation report](docs/validation.md) records campaign results, measured
stage costs, review scope and remaining acceptance gates.

## EVM comparison on the same proof

Run `python3 tools/benchmark_evm.py` to measure real transaction receipts against
the unchanged Solidity verifier in a temporary, loopback-only Anvil. It terminates
that node when finished. Results are saved in [benchmarks/evm.json](benchmarks/evm.json).
The recorded run uses Anvil 1.5.1, Prague, solc 0.8.30 and 200 optimizer runs.

| Measurement | EVM | Solana |
|---|---:|---:|
| Valid verification transaction | 435,183 gas | 1,385,858 CUs |
| Late-invalid verification transaction | 435,200 gas | 1,385,885 CUs |
| Raw proof | 6,752 bytes | 6,752 bytes |
| Transactions to upload and verify a fresh proof | 1 | 11 |
| Refundable proof-buffer rent reserve | None | 0.048198 SOL |

Gas and CUs have different pricing and execution models; their ratio is not a
speed or fee comparison. EVM's valid total includes 132,356 intrinsic/calldata
gas and 302,827 execution gas. Its calldata is 7,460 bytes. The direct core
verifier, with a precomputed public digest, uses 266,942 execution gas; use the
full wrapper for the statement-binding comparison. The old Foundry test's gas
includes loading fixture storage and is not the direct transaction cost.

Solana's 11 transactions are create/initialize, eight appends, seal and verify.
They use 12 signatures: at 5,000 lamports each, the base fees total 0.000060 SOL
before priority fees. Closing adds one transaction and 0.000005 SOL base fee,
and returns the rent reserve. These figures exclude deployment and application
state setup. For EVM, 435,183 gas at an illustrative effective price of 10 gwei
costs 0.00435183 ETH; L2 fee components require a separate chain-specific estimate.
Fee rules: [Solana](https://solana.com/docs/core/fees),
[Ethereum](https://ethereum.org/developers/docs/gas/).

This compares one real proof on pinned local runtimes. It does not measure
public-network confirmation latency, current token prices or an EVM consumer
equivalent to the Solana receipt example.
Solidity bytecode includes source-path metadata, so its hash can differ between
checkouts even when the executable bytes and measured gas are identical.

## Components and guarantees

- `verifier/`: strict scalar/point parsing, SHA-256 statement binding, exact
  Keccak transcript, checked inverses, polynomial arithmetic and native BN254
  operations. JS intermediate traces, arkworks arithmetic and Solidity verdicts
  provide independent comparisons.
- `program/`: sequential bounded upload, immutable seal, fresh verification and
  authority-controlled rent return. The proof remains in an account; verification
  completes in one transaction.
- `consumer/`: example pinning the verifier program ID and guest key, propagating
  CPI failure and updating a signed receipt atomically. Its configuration is for
  tests and must be replaced with the application's trusted deployment and guest.
- `client/`: resumable uploader using fully signed transaction size checks.
  [Wire format and usage](docs/wire.md) describe the authority and consumption
  requirements. No live RPC submission or deployment was performed here.
- `tools/generate_profile.py`: deterministic translation of six stages from one
  SHA-256-pinned Solidity verifier; unsupported source/syntax fails closed.
  [Scalar arithmetic notes](docs/scalar.md) explain the optimized backend's bounds.

The runtime suite checks every proof-word mutation, late pairing failure,
malformed accounts/instructions, wrong authority, post-seal writes, rent return,
verifier substitution and atomic CPI failure. Application output semantics,
freshness and authorization remain the consuming application's responsibility.
Production configuration must also account for trusted setup and verifier
upgrade authority.

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
