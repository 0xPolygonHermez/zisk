# Execute the recorded Nitro transfer on upstream Wasm

Continue the authorized task in `Core/Worktrees/zisk-wasm-nitro-compat`, branch
`test/nitro-wasm-compat`, from `d151975da` (original base `c8190af8e`). The user
requested a fleet to implement the remaining runtime and proving work.

## Execution contract

The compiled Nitro module and pinned runtime map the committed start state to
the committed end state using hash-authenticated preimages and exactly the
committed contiguous inbox windows. Host input is witness, never an unchecked
runtime answer. The ROM/VK identifies the module and adapter together. Existing
L1 accumulator verification remains a separate composition requirement.

For this milestone, reproduce the recorded transfer's block hash
`a67bfee73eec54ea031ebe065a14f2a34f7a783a4e9cf57d1060eb730cfb76bc`, zero send
root, batch5, position0, with the structured256-byte claim. Missing/corrupted
required witness fails. No unsupported service returns fake success.

## Parallel ownership and interfaces

All agents work on components of this same task/worktree; no branch integration
or changes to the original dirty Arbitrum repository. No overlapping writers.

- State agent: freestanding `runtime/state.c`, header, host tests and witness
  format. Bounded witness parser, global state, authenticated preimages, inbox
  consumption/accumulators and claim finalization.
- Brotli agent: `runtime/brotli.c`, freestanding libc, linker script, pinned
  reproducible builder and controls. Execute actual Nitro-compatible codec.
- Proof agent: native secp256k1 hint-failure paths and focused negative tests;
  validate available Assembly/proof route and identify concrete platform limits.
- Parent: shared runtime ABI, compiler/linker/lifecycle integration, WASI output
  policy, adapter-aware runner, recorded fixture encoding and end-to-end tests.

The native runtime is an ELF64 RISC-V ET_EXEC helper, linked into the produced
ROM by the existing RISC-V decoder and native crypto library. Code/RO reside
in `[0x97000000,0x97e00000)`; data/BSS/heap in
`[0xa0600000,0xa1000000)`. Wasm code stops below runtime ROM. Native calls use
the existing separate native stack, saving/restoring live Wasm registers.
Runtime ABI is `nitro_dispatch(op, const u64 args[8], memory_end)`; the compiler
marshals checked-width Wasm argument slots into reserved control cells. Runtime
functions validate every guest pointer. State input is immutable private stdin;
the replay hash is compiler-fixed data, never supplied by witness.

Build native C with the installed LLVM RISC-V target; native F/D instructions,
if needed by Brotli, use the already-supported float library. Bounded allocator
exhaustion fails explicitly. Match pinned codec parameters/dictionaries and
check stream completion. No unchecked host compression or hash callbacks.

The normal Wasm API/CLI retains its WASI behavior. A specific Nitro API enables
the runtime, console-only writes, initialization and successful-exit finalization
for both proc_exit(0) and normal _start return. Unsupported programs calls trap.

## Validation and completion

Start with small failing integration controls, then actual native runtime
execution, adversarial witness/pointer tests, and the recorded transfer. Match
all256 public bytes against an independently computed expected claim. Record
artifact hashes, flags, steps, memory and limitations. Bound large diagnostics
to available machine resources. Run the relevant existing suites once integrated.

Validate the exact Assembly/proof path separately; emulator success does not
substitute for proving. Use any free agent slots for independent review and
remaining proof tasks after runtime components complete. If this machine cannot
run a path, retain the implementation and reproducible commands with concrete
failure evidence. Preserve the worktree; do not push/merge without instruction.

## Implemented and measured

The state/witness, pinned Brotli, native ELF bridge, Nitro compilation API,
successful-exit finalizer and adapter-aware runner are implemented. Expected
end state is used only by the separate fixture oracle. Five inherited native
secp256k1 hint-check failures now trap; forged inverse, square-root, GLV and
parity controls failed before that fix and pass afterward.

Independent review caught two integration defects: allocator exhaustion must
trap instead of returning codec failure, and certificate validation must ignore
unused dispatch argument slots. Both were fixed with negative controls. The
first actual replay then exposed unnecessarily allocating the caller's entire
output capacity. Brotli now starts with at most 64 KiB of scratch and grows only
when output requires it; genuine arena exhaustion still traps. Byte-exact
reference controls and independent review cover both compression/decompression
growth, aliasing and success-only publication.

Recorded transfer result with the final helper: **PASS**, all 256 public bytes
match. ROM: 20,806,583 instructions; execution: 894,975,514 steps; compile time:
9.944 seconds; total process time: 20.479 seconds; sampled peak RSS:
15,880,416 KiB. Run bounded at 24 GiB RSS and 600 seconds. Logs are retained in
`target/nitro-runtime/replay-03{,-resources}.log`; reproducible commands and
artifact hashes are in `transpilers/wasm/runtime/README.md`.

Actual replay rejection controls also passed: corrupted preimage bytes trap at
step 726,350,024; missing preimages at 726,345,647; invalid sequencer count-start
at 894,970,596. Each exits unsuccessfully without writing a public-claim file.
Native state sanitizer tests independently crosscheck the recorded witness and
both claim hashes; the Python encoder self-test confirms expected end state
does not enter the witness.

Assembly/proof status: the small native-crypto control exports/reloads a ZisK
binary ELF through the common loader, preserves native-call metadata and its
full expected recovered key, and generates all four Assembly source modes with
and without hints. Raw Wasm still fails the existing ELF guard. This Darwin
ARM64 host cannot execute the generated x86-64 Assembly; Apple `as` rejects
`--noexecstack` and `--defsym`, Docker is unavailable, and the installed older
`cargo-zisk` cannot validate this source. No proof was generated or verified.

The complete branch diff from `c8190af8e`, including new implementation/tests,
received an independent review with no remaining actionable findings for this
bounded replay scope. Wider Nitro coverage and L1/binder composition remain as
listed in the runtime guide and original design.

Final checks, all passing:

- `cargo test --locked --release -p ziskemu -p zisk-riscv -p zisk-transpiler-wasm
  -- --test-threads=1`: 140 passed, 15 ignored.
- `cargo test --locked --release -p ziskemu --features float -- --test-threads=1`:
  128 passed, 15 ignored.
- `cargo test --locked --release -p zisk-riscv -p zisk-transpiler-wasm -p zisk-asm
  --features float`: 49 passed.
- Explicit runtime tests with `--features float --ignored`: 13 passed. The
  quality-11 encoder growth control takes 2,882,375,250 steps and has its own
  five-billion-step limit; other Brotli controls retain a 500-million limit.
- Explicit `export_native_crypto_proof_control --ignored`: one passed. This
  and the 13 runtime controls account for 14 tests ignored by the broad suite;
  one unrelated pre-existing ignored test was not run.
- State and Brotli native sanitizer controls, recorded witness/claim crosscheck,
  Python encoder self-test, scoped Rust formatting and `git diff --check` pass.

No whole-workspace, Assembly execution, proof generation/verification or
settlement integration claim is made.
