//! Polymorphic execute API. Implemented by both the full proofman-backed
//! clients (`ZiskProver<Emu>` / `ZiskProver<Asm>`) and the standalone
//! clients (`EmuExecClient` / `AsmExecClient`).
//!
//! Object-safe — usable as `Box<dyn ExecuteClient>` for callers that
//! need to dispatch dynamically (e.g. the CLI selecting between four
//! client types at runtime). Concrete types still expose their full
//! fluent APIs (`prover.setup(&program).with_hints().run()?`, etc.) for
//! direct callers that want them.

use anyhow::Result;
use zisk_common::io::{StreamSource, ZiskStdin};

use crate::{ExecuteOutput, GuestProgram};

/// Common surface for any client that can prepare a guest program and
/// run it. Backends that don't support a given feature (e.g. precompile
/// hints on the Rust emulator) silently ignore it.
pub trait ExecuteClient {
    /// Prepare the client to execute the given program. `with_hints`
    /// enables the precompile hints stream where supported (ASM); the
    /// EMU backend ignores it.
    fn setup(&self, program: &GuestProgram, with_hints: bool) -> Result<()>;

    /// Run a single execution of the already-setup program with the
    /// given stdin and optional hints stream. The program reference
    /// must match the one passed to `setup`. `hints` is ignored by
    /// backends that don't support precompile hints.
    fn execute(
        &self,
        program: &GuestProgram,
        stdin: ZiskStdin,
        hints: Option<StreamSource>,
    ) -> Result<ExecuteOutput>;
}
