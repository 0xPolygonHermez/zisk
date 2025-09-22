use std::process::Command;
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
    pub world_rank: i32,
    pub local_rank: i32,
    pub base_port: Option<u16>,
    pub unlock_mapped_memory: bool,
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
            world_rank: 0,
            local_rank: 0,
            base_port: None,
            unlock_mapped_memory: false,
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

    pub fn with_world_rank(mut self, rank: i32) -> Self {
        self.world_rank = rank;
        self
    }

    pub fn with_local_rank(mut self, rank: i32) -> Self {
        self.local_rank = rank;
        self
    }

    pub fn with_base_port(mut self, port: Option<u16>) -> Self {
        self.base_port = port;
        self
    }

    pub fn with_unlock_mapped_memory(mut self, value: bool) -> Self {
        self.unlock_mapped_memory = value;
        self
    }

    /// Applies the configuration flags to a command-line `Command`.
    ///
    /// # Arguments
    /// * `command` - A mutable reference to the `Command` to be modified.
    pub fn apply_to_command(&self, command: &mut Command, asm_service: &AsmService) {
        let port = if self.base_port.is_some() {
            AsmServices::port_for(asm_service, self.base_port.unwrap(), self.local_rank)
        } else {
            AsmServices::default_port(asm_service, self.local_rank)
        };

        // Execute in server mode
        command.arg("-s");

        if self.unlock_mapped_memory {
            command.arg("-u");
        }

        command.arg("--shm_prefix").arg(AsmServices::shmem_prefix(port, self.local_rank));

        match asm_service {
            AsmService::MT => {
                command.arg("--generate_minimal_trace");
            }
            AsmService::RH => {
                command.arg("--generate_rom_histogram");
            }
            AsmService::MO => {
                command.arg("--generate_mem_op");
            }
        }

        if !self.log_output {
            command.arg("-o");
        }

        if self.metrics {
            command.arg("-m");
        }

        if self.verbose {
            command.arg("-v");
            command.stdout(std::process::Stdio::inherit()).stderr(std::process::Stdio::inherit());
        } else {
            command.stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
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

        command.arg("-p").arg(port.to_string());
    }
}
