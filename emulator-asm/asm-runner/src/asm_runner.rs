use std::process::Command;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AsmRunError {
    #[error("Failed to create semaphore '{0}': {1}")]
    SemaphoreError(String, #[source] named_sem::Error),
    #[error("Thread pool creation failed")]
    ThreadPoolError(#[from] rayon::ThreadPoolBuildError),
    #[error("Semaphore wait failed: {0}")]
    SemaphoreWaitError(#[from] std::io::Error),
    #[error("Child process exited with code: {0}")]
    ExitCode(u32),
    #[error("Thread join failed")]
    JoinPanic,
    #[error("Child service returned error: {0}")]
    ServiceError(#[source] anyhow::Error),
    #[error("Arc unwrap failed")]
    ArcUnwrap,
}

pub enum AsmRunnerTraceLevel {
    None,
    Trace,
    ExtendedTrace,
}

pub struct AsmRunnerOptions {
    pub log_output: bool,
    pub metrics: bool,
    pub verbose: bool,
    pub trace_level: AsmRunnerTraceLevel,
    pub keccak_trace: bool,
}

impl Default for AsmRunnerOptions {
    fn default() -> Self {
        Self {
            log_output: false,
            metrics: false,
            verbose: false,
            trace_level: AsmRunnerTraceLevel::None,
            keccak_trace: false,
        }
    }
}

impl AsmRunnerOptions {
    /// Applies the configuration flags to a command-line `Command`.
    ///
    /// # Arguments
    /// * `command` - A mutable reference to the `Command` to be modified.
    pub fn apply_to_command(&self, command: &mut Command) {
        if !self.log_output {
            command.arg("-o");
        }
        if self.metrics {
            command.arg("-m");
        }
        if self.verbose {
            command.arg("-v");
        }
        match self.trace_level {
            AsmRunnerTraceLevel::None => {}
            AsmRunnerTraceLevel::Trace => {
                command.arg("-t");
            }
            AsmRunnerTraceLevel::ExtendedTrace => {
                command.arg("-tt");
            }
        }
        if self.keccak_trace {
            command.arg("-k");
        }
    }
}

/// Builder for `AsmRunnerOptions` to configure assembler runner behavior.
pub struct AsmRunnerOptionsBuilder {
    log_output: bool,
    metrics: bool,
    verbose: bool,
    trace_level: AsmRunnerTraceLevel,
    keccak_trace: bool,
}

impl Default for AsmRunnerOptionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl AsmRunnerOptionsBuilder {
    /// Creates a new builder with default values.
    pub fn new() -> Self {
        Self {
            log_output: false,
            metrics: false,
            verbose: false,
            trace_level: AsmRunnerTraceLevel::None,
            keccak_trace: false,
        }
    }

    /// Enables or disables logging output.
    pub fn log_output(mut self, value: bool) -> Self {
        self.log_output = value;
        self
    }

    /// Enables logging output.
    pub fn with_log_output(self) -> Self {
        self.log_output(true)
    }

    /// Enables or disables metrics collection.
    pub fn metrics(mut self, value: bool) -> Self {
        self.metrics = value;
        self
    }

    /// Enables metrics collection.
    pub fn with_metrics(self) -> Self {
        self.metrics(true)
    }

    /// Enables or disables verbose output.
    pub fn verbose(mut self, value: bool) -> Self {
        self.verbose = value;
        self
    }

    /// Enables verbose output.
    pub fn with_verbose(self) -> Self {
        self.verbose(true)
    }

    /// Sets the desired trace level.
    pub fn trace_level(mut self, level: AsmRunnerTraceLevel) -> Self {
        self.trace_level = level;
        self
    }

    /// Enables or disables Keccak-specific tracing.
    pub fn keccak_trace(mut self, value: bool) -> Self {
        self.keccak_trace = value;
        self
    }

    /// Builds the configured `AsmRunnerOptions`.
    pub fn build(self) -> AsmRunnerOptions {
        AsmRunnerOptions {
            log_output: self.log_output,
            metrics: self.metrics,
            verbose: self.verbose,
            trace_level: self.trace_level,
            keccak_trace: self.keccak_trace,
        }
    }
}
