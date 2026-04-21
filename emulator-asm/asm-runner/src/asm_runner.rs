use std::process::{Command, Stdio};
use thiserror::Error;

use crate::{AsmService, AsmServices};

#[derive(Debug, Error)]
pub enum AsmRunError {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
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

#[derive(Debug, Clone)]
pub enum AsmRunnerTraceLevel {
    None,
    Trace,
    ExtendedTrace,
}

#[derive(Debug, Clone)]
pub struct AsmRunnerOptions {
    pub log_output: bool,
    pub metrics: bool,
    pub verbose: bool,
    pub trace_level: AsmRunnerTraceLevel,
    pub keccak_trace: bool,
    pub local_rank: i32,
    pub unlock_mapped_memory: bool,
    pub asm_out_file: bool,
    pub share_input_shmem: bool,
    pub open_input_shmem: bool,
    pub stdio: bool,
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
            share_input_shmem: false,
            open_input_shmem: false,
            stdio: true,
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

    pub fn with_local_rank(mut self, rank: i32) -> Self {
        self.local_rank = rank;
        self
    }

    pub fn with_unlock_mapped_memory(mut self, value: bool) -> Self {
        self.unlock_mapped_memory = value;
        self
    }

    pub fn with_asm_out_file(mut self, value: bool) -> Self {
        self.asm_out_file = value;
        self
    }

    pub fn with_share_input_shmem(mut self, value: bool) -> Self {
        self.share_input_shmem = value;
        self
    }

    pub fn with_open_input_shmem(mut self, value: bool) -> Self {
        self.open_input_shmem = value;
        self
    }

    pub fn with_stdio(mut self, value: bool) -> Self {
        self.stdio = value;
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

        if self.stdio {
            command.arg("--stdio");
            command.arg("--open_all_shm");
        }

        if self.unlock_mapped_memory {
            command.arg("-u");
        }

        if self.asm_out_file {
            command.arg("--redirect-output-to-file");
        }

        command.arg("--shm_prefix").arg(shm_prefix);
        command.arg("--sem_prefix").arg(sem_prefix);

        if !self.log_output {
            command.arg("-o");
        }

        if self.metrics {
            command.arg("-m");
        }

        // --share_input_shm / --open_input_shm are TCP-mode flags for shared input shmem.
        // In stdio mode --open_all_shm already covers input shmem; passing both conflicts.
        if !self.stdio {
            if self.share_input_shmem {
                command.arg("--share_input_shm");
            }
            if self.open_input_shmem {
                command.arg("--open_input_shm");
            }
        }

        if self.verbose {
            command.arg("-v");
        }

        if !self.stdio {
            command.stdout(if self.verbose { Stdio::inherit() } else { Stdio::null() });
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

        if !self.stdio {
            command
                .arg("-p")
                .arg(AsmServices::default_port(asm_service, self.local_rank).to_string());
        }
    }
}
