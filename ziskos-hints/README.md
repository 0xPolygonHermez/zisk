# ziskos-hints

This crate is a **wrapper around `ziskos`** that compiles the same source code with the `hints` feature enabled.

## How it works

### Symlinked Source
The `src/` directory in this crate is a **symlink** to `../ziskos/entrypoint/src/`. This means:
- Both `ziskos` and `ziskos-hints` compile from the **same source files**
- No code duplication is needed
- Changes to the source are automatically reflected in both crates

### Conditional Compilation
The source code uses conditional compilation to export different C symbols:

```rust
#[cfg_attr(not(feature = "hints"), no_mangle)]
#[cfg_attr(feature = "hints", export_name = "hints_<function_name>")]
pub extern "C" fn <function_name>(...) { ... }
```

When compiled:
- **ziskos** (no hints feature): Exports C symbol `<function_name>`
- **ziskos-hints** (hints feature enabled): Exports C symbol `hints_<function_name>`

### Why This Pattern?

This solves a Cargo limitation: **feature unification**. In a single build, if multiple crates depend on the same crate, Cargo unifies their features. This means you cannot have different feature sets for the same dependency.

By creating a separate crate (`ziskos-hints`) that always enables the `hints` feature, we can:
1. Use `ziskos` without hints in most places
2. Use `ziskos-hints` with hints where needed (e.g., in `precompiles-hints`)
3. Link both into the same binary without symbol conflicts

The different C symbol names (`<function_name>` vs `hints_<function_name>`) prevent linker duplicate symbol errors.

## Usage

In your `Cargo.toml`:
```toml
# For normal usage without hints:
ziskos = { workspace = true }

# For usage with hints enabled:
ziskos-hints = { workspace = true }
```

From Rust code, both have the same API:
```rust
use ziskos::syscall_arith256_mod;

// or
use ziskos_hints::syscall_arith256_mod;

// or rename for consistency:
use ziskos_hints as ziskos;
use ziskos::syscall_arith256_mod;
```

The function name in Rust is the same; only the exported C symbol differs.
