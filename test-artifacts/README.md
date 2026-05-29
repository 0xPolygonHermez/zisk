# Test Artifacts

Host-side crate that compiles every ZisK guest program under [`programs/`](./programs)
into an ELF at build time and re-exports each one as a `pub const GuestProgram`
constant. Other crates in the repository (executor, emulator, SDK tests,
end-to-end benches, …) depend on `test-artifacts` to get a stable, reproducible
set of guest binaries to run against, without having to invoke `cargo-zisk`
themselves.

## Layout

```
test-artifacts/
├── Cargo.toml          # this host crate
├── build.rs            # compiles everything under programs/ via zisk-sdk
├── src/lib.rs          # re-exports each compiled ELF as a constant
└── programs/           # nested Cargo workspace with one member per guest
    ├── Cargo.toml
    ├── blake2/
    ├── ...
```

`programs/` is its own Cargo workspace because the guests cross-compile to the
ZisK target with different profile flags (`opt-level = 3`, `lto = true`,
`panic = "abort"`, …) and a different dependency set than the host workspace.

## How it builds

`build.rs` calls `zisk_sdk::build_program_with_args(programs_dir, BuildArgs::default())`,
which drives `cargo-zisk` over the nested workspace and produces one ELF per
guest. `src/lib.rs` then exposes each ELF with the `load_program!` macro:

```rust
pub const ELF_BLAKE2: GuestProgram = load_program!("blake2");
...
```

## Using an ELF from another crate

Add `test-artifacts` as a `dev-dependency` (or regular dependency for benches)
and pull in the constant you need:

```rust
use test_artifacts::ELF_BLAKE2;
use zisk_sdk::{EmbeddedClientBuilder, ZiskStdin, VerifyConstraintsExtension};

let client  = EmbeddedClientBuilder::new().build();
let stdin   = ZiskStdin::new();
let outcome = client.verify_constraints(&ELF_BLAKE2, stdin).run()?;
```

## Adding a new guest program

1. Create a new crate under `programs/<name>/` mirroring the existing ones.
2. Register the crate as a member of [`programs/Cargo.toml`](./programs/Cargo.toml)
   so the nested workspace picks it up.
3. Expose its ELF in [`src/lib.rs`](./src/lib.rs):
   ```rust
   pub const ELF_MYPROG: GuestProgram = load_program!("myprog");
   ```
   The string argument must match the crate's `package.name`.
4. Rebuild — `build.rs` will compile the new program automatically.

## Notes

- This crate is a workspace member but is **omitted from `default-members`** in
  the outer [`Cargo.toml`](../Cargo.toml), so a bare `cargo build` / `cargo test`
  at the repository root does not trigger the (slow) cross-compilation of every
  guest. Build it explicitly with `cargo build -p test-artifacts`.
- ELF bytes are embedded into the compiled `test-artifacts` rlib via
  `include_bytes!` (inside `load_program!`), so consumers do not need access to
  the `target/` directory at runtime.

## TODOs

- [ ] Add guests for verifying basic ZisK operations like add, xor, ...
- [ ] Add guests for memory operations.
- [ ] Create diagnosis guest for hints (precompile calls)