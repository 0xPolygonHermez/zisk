/// Profiling-related types and utilities.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ProfilingMode {
    /// Tag-level inline profiling (`--sdk --profile-tags`).
    Inline,
    /// Per-opcode + top-functions summary (`--sdk --opcodes --top-functions`).
    Summary,
    /// Full profiler output written to disk (`--sdk --profiler-output`).
    Complete,
}
