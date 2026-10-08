//! Regenerates the committed generated files from `#[constants]` definitions, and only
//! when those sources change.
//!
//! Each source crate (e.g. `zisk-definitions-source`) is a build-dependency, so it is
//! compiled before this script runs — which is how we read the *evaluated* constant
//! tables without tripping the build-script phase wall (a crate's own build.rs runs
//! before its lib compiles, so this can't live in the source crate itself).
//!
//! Codegen is expressed as a list of [`Job`]s, each mapping one source constant table
//! to its per-target output dirs. See `main` for how to add another source or route a
//! target into a folder shared with hand-written files.

use std::env;
use std::path::PathBuf;

use zisk_definitions_generator::meta::{Export, GroupMeta};
use zisk_definitions_generator::{DirMode, Dirs, Out};

const REGEN_CMD: &str = "cargo build -p zisk-definitions-sync";

/// A constant table as the generator consumes it.
type Groups = &'static [(&'static GroupMeta, &'static [Export])];

/// One codegen job: a source constant table and where each target's files are written.
struct Job<'a> {
    /// Source dir watched for changes (`cargo:rerun-if-changed`, scanned recursively).
    watch: PathBuf,
    /// The constant groups to render.
    constants: Groups,
    /// Per-target output dir + reconcile mode. Use [`DirMode::Shared`] for a dir that
    /// also holds hand-written files of the same extension.
    dirs: Dirs<'a>,
}

fn main() {
    // Job 1: generated constants for the `zisk-definitions` crate. The source is the
    // `zisk-definitions-source` crate (definitions/source).
    let defs = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("..");

    use DirMode::Exclusive;

    // `zisk-definitions`' own constants → one generated root under src: Rust at the top
    // (compiled by consumers), C/PIL/asm in dedicated subdirs on the toolchains' include
    // paths. All `Exclusive`: these dirs hold only generated files. This is the single
    // canonical location for every generated form; consumers that build outside the repo
    // (emulator-asm in an installed/packaged tree) get the C headers copied to them at
    // package time — see emulator-asm/Makefile.
    let generated_folder = defs.join("src/generated");
    let [c_dir, pil_dir, asm_dir] = ["c", "pil", "asm"].map(|d| generated_folder.join(d));

    let job1 = Job {
        watch: defs.join("source/src"),
        constants: zisk_definitions_source::ZISK_CONSTANTS,
        dirs: Dirs {
            rust: Out { path: &generated_folder, mode: Exclusive },
            c: Out { path: &c_dir, mode: Exclusive },
            pil: Out { path: &pil_dir, mode: Exclusive },
            asm: Out { path: &asm_dir, mode: Exclusive },
        },
    };

    // Process each job: write the generated files, and tell Cargo to re-run this build script
    // whenever the source folder changes.
    let jobs = [job1];
    // Each write reconciles a dir against only its own job's files, so two jobs sharing
    // a target dir would delete each other's outputs: refuse that before writing any.
    let all_dirs: Vec<Dirs> = jobs.iter().map(|job| job.dirs).collect();
    if let Err(e) = zisk_definitions_generator::ensure_disjoint(&all_dirs) {
        panic!("conflicting codegen jobs: {e}");
    }
    for job in &jobs {
        // Re-run whenever a source module changes (cargo scans the dir recursively).
        println!("cargo:rerun-if-changed={}", job.watch.display());
        zisk_definitions_generator::write(job.constants, &job.dirs, REGEN_CMD)
            .expect("regenerating constants");
    }
}
