# Validation record — 2026-11-09

This record covers the fixed ZisK v1.4.0-alpha BLAKE3 leaf Pilfflonk profile.
It is not a production sign-off. Standard snarkjs FFLONK,
other recursion roots and other verification keys are outside this implementation.

## Reproducible checks

`bash scripts/check.sh` passed locally with the versions in the README:

- The unchanged Proofman JS verifier accepts the release proof and rejects
  changed statement/inverse inputs. The fixture and generated profile reproduce.
- Three Solidity tests against the unchanged reference pass, including all 211 proof-word
  mutations and changed high output bits.
- Nine Python generator/benchmark/build-script tests, nine Rust verifier tests and two client
  tests pass. The Rust suite compares JS intermediate values and 100,000 seeded
  scalar arithmetic/conversion pairs against arkworks.
- Six compiled-SBF runtime tests pass. They cover all 211 proof-word mutations,
  late pairing failure, malformed accounts/instructions, unauthorized writes,
  seal immutability, close/recreate, duplicate accounts, rent return, authenticated
  consumer CPI, substitute verifier rejection and atomic failure.

The runtime tests use signed, serialized transactions and the normal 1.4M-CU
ceiling. The release ELF was restored after profiling, and the runtime suite and
benchmark comparison passed again. The GitHub workflow uses the same scripts;
the results described here come from local runs.

Build-script tests cover Bash 3.2 and LLVM diagnostic rejection without optional
search tools. Clippy runs with warnings denied. The scalar loops retain narrow
Clippy allowances to preserve their measured SBF instruction sequence, and the
workspace pins Rust 1.95.0 independently of the parent repository's toolchain.

The complete suite passed on macOS and in a clean Ubuntu 24.04.5 x86_64 container
under Docker emulation on Apple silicon. The container used freshly installed
pinned tools and no cached project dependencies. CU measurements matched, and
the verifier ELF was byte-identical. The consumer ELF differed between hosts,
with the same size and measured CUs; its runtime tests passed on both hosts.
[`linux.json`](../benchmarks/linux.json) records the Linux source revision and
artifact hashes. Source revisions in historical reports identify the code used
for those runs, before PR history was consolidated. Actionlint 1.7.12 passed for
the verifier workflow with ShellCheck disabled.

## Compute and resource measurements

The uninstrumented release measurements and artifact hashes are in
[`integration.json`](../benchmarks/integration.json). Verification consumes
1,385,858 CUs; a late-invalid opening consumes 1,385,885 CUs and returns the
verifier's invalid-proof error. The small consumer consumes 1,388,559 CUs,
including CPI and its receipt update: 2,701 CUs more than standalone verification.
The consumer has only 11,441 CUs of headroom. These are observed paths, not a
proven upper bound for every possible proof or application.

[`profile.json`](../benchmarks/profile.json) records adjacent instrumented
checkpoint deltas. They include logging overhead and are for attribution;
their total of 1,388,367 CUs is not the release total.

| Instrumented stage | CUs |
|---|---:|
| Entry and first checkpoint | 750 |
| Decode, range checks and point validation | 167,869 |
| Statement hash and transcript | 14,256 |
| ZH and ZI | 26,136 |
| Q polynomial | 757,573 |
| Z polynomial | 9,753 |
| Checked inversions | 74,902 |
| R polynomials | 184,908 |
| Group accumulation and final assembly | 102,691 |
| Pairing and return | 49,529 |

The Q stage is the largest remaining cost. The 1M-CU target is unmet; optimization
is not complete. The original arkworks scalar backend was over budget at about
3.43M CUs with profiling. The replacement's arithmetic bounds and equivalence
checks are described in [scalar.md](scalar.md).

The proof has 6,752 bytes; its account has 6,797 bytes and requires 48,198,000
lamports at this runtime's rent settings. Eight append transactions upload it.
The largest serialized upload transaction is 1,150 bytes, below 1,232 bytes.
Create plus initialize consumes 41,742 CUs, each append 884 CUs, seal 852 CUs
and close 938 CUs. Rent is returned on close; transaction fees are separate.

Release program sizes are 256,720 bytes for the verifier and 25,040 bytes for the
consumer. Execution passes with default runtime stack/heap settings and no
enlarged heap request. The two principal field vectors hold 193 and 98 values
of 36 bytes each. No exact stack or allocator high-water measurement was made.
The build script rejects LLVM stack-frame errors even if the builder exits zero.

The pinned builder emits `Invalid syscall code 25` during its advisory check.
Inspection found that its check scans the entire ELF as instructions: the
call-like bytes are at offset 9,040 in `.rodata`, outside `.text` (offset 12,272).
The runtime's actual ELF loader and execution pass. This warning is not hidden
or treated as evidence that a real unavailable syscall works.

## Fuzzing

The `verify` target compares arithmetic with arkworks and exercises the real
decoder/verifier, beginning with the valid release proof. A 30-minute local
campaign completed 3,944,810 executions in 1,801 seconds without a crash or
arithmetic mismatch. It ended at libFuzzer counters `cov=2478`, `ft=3873`;
these are instrumentation counters, not percent coverage or a soundness claim.

`buffer_state` compares the actual host entrypoint with a state model over at
most 64 operations. It varies authorities, signer/writable privileges, owners,
account sizes, offsets, chunk sizes, sealing, verification, closing,
reinitialization and duplicate-account aliases. Its public seed reaches a
complete upload and seal. This host harness does not model runtime rollback,
account deletion or concurrent transactions; compiled-SBF integration tests
cover close/recreate and atomic failure separately.

The pinned `bash scripts/fuzz.sh 60` smoke command also passed for both targets
locally, with no fuzz lockfile drift. CI starts from the reconstructed public
seeds; the macOS cleanup smoke reused the campaign corpus. The clean Linux run
started from public seeds and completed 161,067 proof/arithmetic executions and
359,026 account-state executions without failure, with no fuzz lockfile drift.

The 10-minute upload-state campaign completed 4,012,854 executions in 601 seconds without a
crash or model disagreement, ending at `cov=549`, `ft=3575`.

Seeds, durations, final counters and binary/log hashes are recorded in
[`fuzz.json`](../benchmarks/fuzz.json). `tools/seed_fuzz.py` reconstructs the public
starting inputs. The discovered corpus and complete logs are retained locally
under ignored `fuzz/corpus/` and `.cache/campaigns/`. Replaying the same random
seed is not guaranteed to reproduce an identical mutation schedule across
machines, compilers, elapsed-time limits or a grown corpus. There were no crash
inputs to retain from the completed campaigns.

## Review and remaining gates

Source reviews covered the reference equations, transcript and statement binding,
generated arithmetic, scalar bounds, curve encodings, account lifecycle, client
and consumer. These agent reviews did not constitute an independent professional
cryptographic audit. The arithmetic bounds are documented in [scalar.md](scalar.md).

The following gates remain open:

- Two additional independently generated real proofs with distinct outputs
  and confirmed key identity. Only one release fixture was available; mutations
  do not replace this corpus. Native exporter attempts failed on this host, as
  documented in the README.
- The 1M-CU performance target and broader proof/application cost measurements.
  The current consumer leaves too little headroom for substantial application work.
- External cryptographic review, including the generated profile and custom
  field backend, before production use.
- A selected deployment's live feature set, RPC submission, trusted setup
  provenance review, upgrade authority policy and application output/freshness
  rules. The example's program ID and guest key are test configuration.

No deployment or public-cluster transaction was performed during validation.
