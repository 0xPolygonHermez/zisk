//! Regenerates the committed generated files from `#[constants]` definitions, and only
//! when those sources change.
//!
//! A source crate (with `gen`) is a build-dependency, so it is compiled before this
//! script runs — which is how we read the *evaluated* constant tables without tripping
//! the build-script phase wall (a crate's own build.rs runs before its lib compiles, so
//! this can't live in the source crate itself).
//!
//! Codegen is expressed as a list of [`Job`]s, each mapping one source constant table
//! to its per-target output dirs. See [`jobs`] for how to add another source or route a
//! target into a folder shared with hand-written files.

use std::env;
use std::path::PathBuf;

use zisk_definitions_generator::meta::{Export, GroupMeta};
use zisk_definitions_generator::{DirMode, Dirs, Out};

const REGEN_CMD: &str = "cargo build -p zisk-definitions-sync";

/// A constant table as the generator consumes it.
type Groups = &'static [(&'static GroupMeta, &'static [Export])];

/// One codegen job: a source constant table and where each target's files are written.
struct Job {
    /// Source dir watched for changes (`cargo:rerun-if-changed`, scanned recursively).
    watch: PathBuf,
    /// The constant groups to render.
    constants: Groups,
    /// Per-target output dir + reconcile mode. Use [`DirMode::Shared`] for a dir that
    /// also holds hand-written files of the same extension.
    rust: (PathBuf, DirMode),
    c: (PathBuf, DirMode),
    pil: (PathBuf, DirMode),
    asm: (PathBuf, DirMode),
}

impl Job {
    fn dirs(&self) -> Dirs<'_> {
        Dirs {
            rust: Out { path: &self.rust.0, mode: self.rust.1 },
            c: Out { path: &self.c.0, mode: self.c.1 },
            pil: Out { path: &self.pil.0, mode: self.pil.1 },
            asm: Out { path: &self.asm.0, mode: self.asm.1 },
        }
    }
}

fn main() {
    // Job 1: generated constants for the `zisk-definitions` crate itself. The source is in
    // definitions/sync/src/constants.
    let defs = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("..");

    use DirMode::Exclusive;

    // `zisk-definitions`' own constants → one generated root under src: Rust at the top
    // (compiled by consumers), C/PIL/asm in dedicated subdirs on the toolchains' include
    // paths. All `Exclusive`: these dirs hold only generated files.
    let generated_folder = defs.join("src/generated");
    let source_folder = defs.join("src/constants");

    let job1 = Job {
        watch: source_folder,
        constants: zisk_definitions::ZISK_CONSTANTS,
        rust: (generated_folder.clone(), Exclusive),
        c: (generated_folder.join("c"), Exclusive),
        pil: (generated_folder.join("pil"), Exclusive),
        asm: (generated_folder.join("asm"), Exclusive),
    };

    // Process each job: write the generated files, and tell Cargo to re-run this build script
    // whenever the source folder changes.
    let jobs = [job1];
    for job in jobs {
        // Re-run whenever a source module changes (cargo scans the dir recursively).
        println!("cargo:rerun-if-changed={}", job.watch.display());
        zisk_definitions_generator::write(job.constants, &job.dirs(), REGEN_CMD)
            .expect("regenerating constants");
    }
}
