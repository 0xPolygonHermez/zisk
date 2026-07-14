//! `cargo-zisk-dev new-precompile <name>` — scaffold a new precompile.
//!
//! Emits the *standardized* parts — a valid `zisk-precompile.toml` manifest plus a
//! crate skeleton — so an author starts from a working manifest instead of copying
//! boilerplate. The bespoke parts (witness SM, PIL, op-type const, guest wrapper,
//! `opc_*` fn) stay the author's job and are printed as next steps. It does **not**
//! enable the precompile — that changes the vk, so the author adds the registry line
//! in `zisk-precompiles.toml` when ready.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

use super::gen_ops::{validate, Manifest};

/// Scaffold a new precompile crate from the standard template.
#[derive(clap::Args)]
#[command(about = "Scaffold a new precompile (manifest + crate skeleton) from the standard template")]
pub(crate) struct NewPrecompileCmd {
    /// Precompile name in snake_case (e.g. `babyjubjub`, `big_int`).
    name: String,

    /// Target directory (defaults to `precompiles/<name>`; pass a path for out-of-tree).
    #[arg(long = "dir")]
    dir: Option<PathBuf>,
}

impl NewPrecompileCmd {
    pub(crate) fn run(&self) -> Result<()> {
        let dir =
            self.dir.clone().unwrap_or_else(|| Path::new("precompiles").join(&self.name));
        scaffold(&self.name, &dir)?;
        println!("scaffolded precompile '{}' at {}", self.name, dir.display());
        print_next_steps(&dir);
        Ok(())
    }
}

/// snake_case → PascalCase (`big_int` → `BigInt`, `arith_eq_384` → `ArithEq384`).
fn to_pascal(snake: &str) -> String {
    snake
        .split('_')
        .filter(|s| !s.is_empty())
        .map(|seg| {
            let mut chars = seg.chars();
            match chars.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// Write the manifest + crate skeleton for `name` into `dir`. Factored out of `run`
/// so tests can scaffold into a temp dir. Fails if `dir` already exists.
fn scaffold(name: &str, dir: &Path) -> Result<()> {
    // The name drives the crate name, Rust path, type stem, and const names, so it
    // must be a snake_case identifier.
    if name.is_empty()
        || !name.starts_with(|c: char| c.is_ascii_lowercase())
        || !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        bail!("precompile name '{name}' must be snake_case: [a-z][a-z0-9_]*");
    }
    if dir.exists() {
        bail!("{} already exists — refusing to overwrite", dir.display());
    }

    let stem = to_pascal(name); // BigInt
    let upper = name.to_uppercase(); // BIG_INT
    let crate_name = format!("zisk-precomp-{}", name.replace('_', "-"));
    let sm_crate = format!("zisk_precomp_{name}");

    let manifest = manifest_toml(name, &stem, &upper, &sm_crate);
    // Self-check: the scaffolded manifest must satisfy the gen-ops schema + validator,
    // so `new-precompile` can never emit something the pipeline would reject.
    let parsed: Manifest = toml::from_str(&manifest)
        .context("scaffolded manifest failed to parse (template bug)")?;
    validate(std::slice::from_ref(&parsed.precompile))
        .context("scaffolded manifest failed validation (template bug)")?;

    fs::create_dir_all(dir.join("src")).with_context(|| format!("creating {}", dir.display()))?;
    fs::write(dir.join("zisk-precompile.toml"), manifest)
        .with_context(|| format!("writing manifest in {}", dir.display()))?;
    fs::write(dir.join("Cargo.toml"), cargo_toml(&crate_name, name))
        .with_context(|| format!("writing Cargo.toml in {}", dir.display()))?;
    fs::write(dir.join("src/lib.rs"), lib_rs(&stem))
        .with_context(|| format!("writing src/lib.rs in {}", dir.display()))?;
    Ok(())
}

fn manifest_toml(name: &str, stem: &str, upper: &str, sm_crate: &str) -> String {
    format!(
        "# Definition of the {name} precompile — read by `cargo-zisk-dev gen-ops`.
# Scaffolded by `cargo-zisk-dev new-precompile`; fill in the TODOs, then enable it in
# `zisk-precompiles.toml` and run `cargo-zisk-dev gen-ops`.
[precompile]
name        = \"{stem}\"
op_type     = \"{stem}\"                        # ZiskOperationType variant (add to core/src/zisk_inst.rs)
op_type_id_const = \"{upper}_OP_TYPE_ID\"       # TODO: add this const to zisk_inst.rs (or wait for the band move)
sm_crate    = \"{sm_crate}\"
air_ids     = \"{upper}_AIR_IDS\"               # TODO: from generated zisk_pil traces (after compile-pil)
rank_assign = false

[[precompile.op]]
name        = \"{stem}\"
str         = \"{name}\"
opcode      = 0xEF                              # TODO: pick a free opcode (check zisk_ops_table.rs + other manifests)
cost        = \"{upper}_COST\"                  # TODO: add this const to core/src/zisk_ops_costs.rs
input_size  = 0                                 # TODO: bytes read from memory
output_size = 0                                 # TODO: bytes written back
stats       = false
syscall     = \"{upper}\"                       # SYSCALL_<syscall>_ID stem
syscall_id  = 0x81C                             # TODO: pick a free syscall id in 0x800..=0x84F
"
    )
}

fn cargo_toml(crate_name: &str, name: &str) -> String {
    format!(
        "[package]
name = \"{crate_name}\"
description = \"TODO: {name} precompile for the ZisK zkVM\"
version = {{ workspace = true }}
edition = {{ workspace = true }}
license = {{ workspace = true }}
keywords = {{ workspace = true }}
homepage = {{ workspace = true }}
repository = {{ workspace = true }}
documentation = {{ workspace = true }}
categories = {{ workspace = true }}

[dependencies]
zisk-core = {{ workspace = true }}
zisk-common = {{ workspace = true }}
zisk-pil = {{ workspace = true }}
zisk-precomp-common = {{ workspace = true }}

proofman-common = {{ workspace = true }}
proofman-fields = {{ workspace = true }}
pil2-std-lib = {{ workspace = true }}
tracing = {{ workspace = true }}

[features]
default = []
"
    )
}

fn lib_rs(stem: &str) -> String {
    format!(
        "//! {stem} precompile state machine (scaffolded by `cargo-zisk-dev new-precompile`).
//!
//! The `zisk_precompile!` macro derives the `{stem}Manager/Instance/Collector/
//! CounterInputGen` types the executor's `register_precompiles!` expects. Implement
//! the op input decode + witness generation (model it on
//! `precompiles/big_int/src/add256.rs`) and add `{stem}Trace` to the PIL, then
//! uncomment the invocation below. See docs/design/modular-precompiles.md.

// TODO: define the op input type(s) and the witness SM, then uncomment:
//
// zisk_common::zisk_precompile! {{
//     name = {stem},
//     op_type = {stem},
//     trace = {stem}Trace,
//     num_available = ::zisk_pil::{stem}Trace::<()>::NUM_ROWS,
//     ops = [
//         (Operation{stem}Data, {stem}Input),
//     ],
// }}
"
    )
}

fn print_next_steps(dir: &Path) {
    let d = dir.display();
    println!(
        "\nNext steps (the bespoke work the scaffold can't do):
  1. Fill the TODOs in {d}/zisk-precompile.toml (opcode, syscall_id, sizes, consts).
  2. Implement the witness SM in {d}/src/ (model on precompiles/big_int/src/).
  3. Add the AIR to the PIL, and the op-type variant + *_OP_TYPE_ID + *_COST consts to core.
  4. Add the guest syscall wrapper (ziskos/entrypoint/src/syscalls/) and the opc_* fn (core).
  5. Add the crate to the workspace Cargo.toml `members`, and wire deps where needed.
  6. Enable it in zisk-precompiles.toml (NOTE: this changes the vk) and run `cargo-zisk-dev gen-ops`."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_pascal_cases() {
        assert_eq!(to_pascal("big_int"), "BigInt");
        assert_eq!(to_pascal("babyjubjub"), "Babyjubjub");
        assert_eq!(to_pascal("arith_eq_384"), "ArithEq384");
    }

    #[test]
    fn scaffold_emits_a_valid_manifest_and_files() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("babyjubjub");
        scaffold("babyjubjub", &dir).expect("scaffold");

        assert!(dir.join("zisk-precompile.toml").exists());
        assert!(dir.join("Cargo.toml").exists());
        assert!(dir.join("src/lib.rs").exists());

        // Independently re-validate the emitted manifest through the gen-ops schema.
        let manifest: Manifest = toml::from_str(
            &fs::read_to_string(dir.join("zisk-precompile.toml")).expect("read manifest"),
        )
        .expect("parse scaffolded manifest");
        validate(std::slice::from_ref(&manifest.precompile)).expect("scaffolded manifest valid");
    }

    #[test]
    fn rejects_non_snake_case_and_existing_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        assert!(scaffold("BabyJubJub", &tmp.path().join("a")).is_err(), "PascalCase rejected");
        assert!(scaffold("1bad", &tmp.path().join("b")).is_err(), "leading digit rejected");

        let existing = tmp.path().join("c");
        fs::create_dir_all(&existing).unwrap();
        assert!(scaffold("valid_name", &existing).is_err(), "existing dir refused");
    }
}
