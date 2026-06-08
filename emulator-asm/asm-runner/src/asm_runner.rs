use std::process::{Command, Stdio};
use thiserror::Error;

use crate::AsmService;

/// Enum representing various errors that can occur during the execution of the assembly runner, including semaphore errors, thread pool errors, child process errors, and unexpected conditions.
#[derive(Debug, Error)]
pub enum AsmRunError {
    /// Errors related to semaphore creation and synchronization.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[error("Failed to create semaphore '{0}': {1}")]
    SemaphoreError(String, #[source] named_sem::Error),

    /// Errors related to thread pool creation for parallel execution.
    #[error("Thread pool creation failed")]
    ThreadPoolError(#[from] rayon::ThreadPoolBuildError),

    /// Errors related to waiting on a semaphore.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[error("Semaphore wait failed: {0}")]
    SemaphoreWaitError(#[from] std::io::Error),

    /// Errors related to child process execution, including non-zero exit codes.
    #[error("Child process exited with code: {0}")]
    ExitCode(u32),

    /// Errors related to joining the thread that runs the child process.
    #[error("Thread join failed")]
    JoinPanic,

    /// Errors returned by the child service process, encapsulated as `anyhow::Error` for context.
    #[error("Child service returned error: {0}")]
    ServiceError(#[source] anyhow::Error),

    /// Errors related to unexpected conditions, such as unwrapping an `Arc` that has been dropped.
    #[error("Arc unwrap failed")]
    ArcUnwrap,
}

/// Enum representing the level of tracing to be performed during assembly execution, with options for no tracing, basic tracing, and extended tracing.
#[derive(Debug, Clone)]
pub enum AsmRunnerTraceLevel {
    /// No tracing will be performed.
    None,
    /// Basic tracing will be performed, capturing essential execution information.
    Trace,
    /// Extended tracing will be performed, capturing detailed execution information for in-depth analysis.
    ExtendedTrace,
}

/// This struct represents the assembly runner options, allowing configuration of logging, metrics, verbosity, trace level, and other execution parameters. It provides a builder pattern for easy configuration and a method to apply these options to a command-line `Command` that will execute the assembly code.
#[derive(Debug, Clone)]
pub struct AsmRunnerOptions {
    /// Enables or disables logging output from the assembly runner.
    pub log_output: bool,

    /// Enables or disables metrics collection during assembly execution.
    pub metrics: bool,

    /// Enables or disables verbose output for debugging purposes.
    pub verbose: bool,

    /// Specifies the level of tracing to be performed during assembly execution.
    pub trace_level: AsmRunnerTraceLevel,

    /// Enables or disables Keccak-specific tracing, which may provide additional insights for certain workloads.
    pub keccak_trace: bool,

    /// The local rank of the process, used for distinguishing between multiple instances in a distributed setup.
    pub local_rank: i32,

    /// Enables or disables unlocking of mapped memory after use, which can be important for certain performance optimizations or resource management strategies.
    pub unlock_mapped_memory: bool,

    /// Enables or disables redirecting assembly output to a file, which can be useful for debugging or record-keeping.
    pub asm_out_file: bool,
}

impl Default for AsmRunnerOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl AsmRunnerOptions {
    /// Creates a new builder with default values.
    pub fn new() -> Self {
        Self {
            log_output: false,
            metrics: false,
            verbose: false,
            trace_level: AsmRunnerTraceLevel::None,
            keccak_trace: false,
            local_rank: 0,
            unlock_mapped_memory: false,
            asm_out_file: false,
        }
    }

    /// Enables or disables logging output.
    pub fn with_log_output(mut self, value: bool) -> Self {
        self.log_output = value;
        self
    }

    /// Enables or disables metrics collection.
    pub fn with_metrics(mut self, value: bool) -> Self {
        self.metrics = value;
        self
    }

    /// Enables or disables verbose output.
    pub fn with_verbose(mut self, value: bool) -> Self {
        self.verbose = value;
        self
    }

    /// Sets the desired trace level.
    pub fn with_trace_level(mut self, level: AsmRunnerTraceLevel) -> Self {
        self.trace_level = level;
        self
    }

    /// Enables or disables Keccak-specific tracing.
    pub fn keccak_trace(mut self, value: bool) -> Self {
        self.keccak_trace = value;
        self
    }

    /// Sets the local rank of the process.
    pub fn with_local_rank(mut self, rank: i32) -> Self {
        self.local_rank = rank;
        self
    }

    /// Enables or disables unlocking of mapped memory after use.
    pub fn with_unlock_mapped_memory(mut self, value: bool) -> Self {
        self.unlock_mapped_memory = value;
        self
    }

    /// Enables or disables redirecting assembly output to a file.
    pub fn with_asm_out_file(mut self, value: bool) -> Self {
        self.asm_out_file = value;
        self
    }

    /// Applies the configuration flags to a command-line `Command`.
    ///
    /// # Arguments
    /// * `command` - A mutable reference to the `Command` to be modified.
    pub fn apply_to_command(
        &self,
        command: &mut Command,
        asm_service: &AsmService,
        shm_prefix: &str,
        sem_prefix: &str,
    ) {
        // Execute in server mode
        command.arg("-s");

        command.arg(format!("--gen={}", asm_service.gen_index()));

        command.arg("--stdio");

        command.arg("--open_all_shm");
        command.arg("--share_input_shm");

        if self.unlock_mapped_memory {
            command.arg("-u");
        }

        if self.asm_out_file {
            command.arg("--redirect-output-to-file");
        }

        command.arg("--shm_prefix").arg(shm_prefix);
        command.arg("--sem_prefix").arg(sem_prefix);

        if self.log_output {
            command.arg("-o");
        }

        if self.metrics {
            command.arg("-m");
        }

        if self.verbose {
            command.arg("-v");
        }

        command.stderr(if self.verbose { Stdio::inherit() } else { Stdio::null() });

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
