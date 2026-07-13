//! Regenerates `zisk-definitions`' committed generated files from its `#[constants]`
//! definitions, and only when those sources change.
//!
//! `zisk-definitions` (with `gen`) is a build-dependency, so it is compiled before
//! this script runs — which is how we read the *evaluated* `ZISK_CONSTANTS` without
//! tripping the build-script phase wall (a crate's own build.rs runs before its
//! lib compiles, so this can't live in `zisk-definitions` itself).

use std::env;
use std::path::PathBuf;

use zisk_definitions_generator::Dirs;

fn main() {
    // definitions/regen/.. == definitions
    let defs = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("..");
    let src = defs.join("src");

    // Re-run only when the constant definitions change.
    // Add a file here if the source is ever split across more modules.
    let f = "constants.rs";
    println!("cargo:rerun-if-changed={}", src.join(f).display());

    // One generated root under src: Rust files at the top (compiled by consumers),
    // C and PIL in subdirs referenced by those toolchains' include paths.
    let generated = src.join("generated");
    let c = generated.join("c");
    let pil = generated.join("pil");
    let dirs = Dirs { rust: &generated, c: &c, pil: &pil };

    zisk_definitions_generator::write(
        zisk_definitions::ZISK_CONSTANTS,
        &dirs,
        "cargo build -p zisk-definitions-regen",
    )
    .expect("regenerating zisk-definitions constants");
}
