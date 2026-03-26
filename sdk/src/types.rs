/// Executor backend for running programs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Executor {
    /// Emulator: always available.
    #[default]
    Emulator,
    /// Assembly: must be explicitly enabled on the client builder.
    Assembly,
}

/// Events emitted during proof generation.
///
/// `WatchEvent::All` is a subscription filter meaning "receive all events".
/// It is never emitted as a concrete event in callbacks.
#[derive(Debug, Clone)]
pub enum WatchEvent {
    /// Subscribe to all events (filter only; never emitted to callbacks).
    All,
    /// Job accepted and execution started.
    Started,
    /// Proof generation progress (0–100).
    Progress(u8),
    /// Proof completed successfully.
    Completed,
    /// Proof generation failed.
    Failed(String),
}

/// Shared capability interface implemented by both `ProverClient` and `RemoteProverClient`.
pub trait ClientConfig {
    /// Whether this client was configured to support the Assembly executor.
    fn assembly_enabled(&self) -> bool;

    /// The default executor used when not overridden per-prove call.
    fn default_executor(&self) -> Executor;
}
