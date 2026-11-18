use std::process::{Command, Stdio};
use thiserror::Error;

use crate::{
    AsmService, RegionFailure, TaskAt, PAR_ABORT_ERROR, PAR_ABORT_TASK_ENDED,
    PAR_ABORT_TASK_FAULTED,
};

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

    /// A service's process has exited, or was stopped. Every later request to it fails with
    /// this error rather than with whatever the broken pipe would report, until its program's
    /// next activation starts it again.
    #[error(
        "ASM service {service} can serve no more requests ({how}); \
         it is started again before its program's next job"
    )]
    ServiceDied {
        /// The service, as its binary is named: `mo`, `mt` or `rh`.
        service: String,
        /// Why: how its process exited, or why it was stopped.
        how: String,
    },

    /// The guest ended or faulted inside a fork-join task, which a region run apart does not
    /// follow: a task must not end the program.
    #[error(
        "the guest {failure} inside a fork-join task{}: a task must not end the program (with \
         ZISK_ASM_PAR_WORKERS=1 regions run one task after another, as in a sequential execution)",
        task_at_suffix(.at)
    )]
    ForkJoinTask {
        /// What the guest did inside the task.
        failure: TaskFailure,
        /// The task, when the service said which.
        at: Option<TaskAt>,
    },

    /// A fork-join region run apart failed in the assembly service, other than by the guest
    /// ending or faulting inside a task: a task's process could not be set up or publish its
    /// chunks, left its task in another state than the region's, or reached a limit of the
    /// host's; or the service could not set the region up.
    #[error(
        "a fork-join region failed in the assembly service: {} (with ZISK_ASM_PAR_WORKERS=1 \
         regions run one task after another, as in a sequential execution)",
        region_failure_text(.failure)
    )]
    ForkJoinRegionFailed {
        /// What failed, as the service reported it, if it did.
        failure: Option<RegionFailure>,
    },

    /// Errors related to unexpected conditions, such as unwrapping an `Arc` that has been dropped.
    #[error("Arc unwrap failed")]
    ArcUnwrap,
}

/// What the guest did inside a fork-join task: see [`AsmRunError::ForkJoinTask`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskFailure {
    /// It ended the program: it exited or panicked.
    Ended,
    /// It faulted.
    Faulted,
}

impl std::fmt::Display for TaskFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Ended => "ended (it exited or panicked)",
            Self::Faulted => "faulted",
        })
    }
}

fn task_at_suffix(at: &Option<TaskAt>) -> String {
    at.map(|at| format!(" ({at})")).unwrap_or_default()
}

fn region_failure_text(failure: &Option<RegionFailure>) -> String {
    match failure {
        Some(failure) => failure.to_string(),
        None => "the service did not say how; its log does".to_string(),
    }
}

impl AsmRunError {
    /// The error for a run a service ended with a nonzero `exit_code`: the codes of a run a
    /// fork-join region aborted (`PAR_ABORT_*`) by what they mean, with the `failure` the service
    /// reported, any other as is.
    pub(crate) fn from_exit_code(exit_code: u64, failure: Option<RegionFailure>) -> Self {
        let at = failure.and_then(|failure| failure.task_at());
        match exit_code {
            PAR_ABORT_ERROR => Self::ForkJoinRegionFailed { failure },
            PAR_ABORT_TASK_ENDED => Self::ForkJoinTask { failure: TaskFailure::Ended, at },
            PAR_ABORT_TASK_FAULTED => Self::ForkJoinTask { failure: TaskFailure::Faulted, at },
            code => Self::ExitCode(code as u32),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AsmService;

    #[test]
    fn options_default_to_all_off() {
        let o = AsmRunnerOptions::new();
        assert!(!o.log_output);
        assert!(!o.metrics);
        assert!(!o.verbose);
        assert!(!o.keccak_trace);
        assert!(!o.unlock_mapped_memory);
        assert!(!o.asm_out_file);
        assert_eq!(o.local_rank, 0);
        assert!(matches!(o.trace_level, AsmRunnerTraceLevel::None));
        // Default must equal `new()`.
        let d = AsmRunnerOptions::default();
        assert_eq!(d.verbose, o.verbose);
        assert_eq!(d.local_rank, o.local_rank);
    }

    #[test]
    fn builder_sets_each_field() {
        let o = AsmRunnerOptions::new()
            .with_verbose(true)
            .with_metrics(true)
            .with_log_output(true)
            .with_local_rank(3)
            .with_unlock_mapped_memory(true)
            .with_asm_out_file(true)
            .keccak_trace(true)
            .with_trace_level(AsmRunnerTraceLevel::ExtendedTrace);
        assert!(o.verbose && o.metrics && o.log_output);
        assert!(o.unlock_mapped_memory && o.asm_out_file && o.keccak_trace);
        assert_eq!(o.local_rank, 3);
        assert!(matches!(o.trace_level, AsmRunnerTraceLevel::ExtendedTrace));
    }

    /// The codes of a run a fork-join region aborted name what failed, and say what to do; any
    /// other code is reported as is.
    #[test]
    fn fork_join_abort_codes_name_their_failure() {
        use crate::FailureCause;
        let in_task = |cause| RegionFailure { region: Some(1), task: Some(3), cause };
        let at = Some(TaskAt { region: 1, task: 3 });

        assert!(matches!(
            AsmRunError::from_exit_code(PAR_ABORT_ERROR, None),
            AsmRunError::ForkJoinRegionFailed { failure: None }
        ));
        assert!(matches!(
            AsmRunError::from_exit_code(PAR_ABORT_TASK_ENDED, None),
            AsmRunError::ForkJoinTask { failure: TaskFailure::Ended, at: None }
        ));
        assert!(matches!(
            AsmRunError::from_exit_code(PAR_ABORT_TASK_ENDED, Some(in_task(FailureCause::GuestEnded))),
            AsmRunError::ForkJoinTask { failure: TaskFailure::Ended, at: a } if a == at
        ));
        assert!(matches!(
            AsmRunError::from_exit_code(PAR_ABORT_TASK_FAULTED, None),
            AsmRunError::ForkJoinTask { failure: TaskFailure::Faulted, at: None }
        ));
        for code in [1u32, 11, 0x103] {
            assert!(matches!(
                AsmRunError::from_exit_code(code.into(), None),
                AsmRunError::ExitCode(c) if c == code
            ));
        }

        let message = |code, failure| AsmRunError::from_exit_code(code, failure).to_string();
        let ended = message(PAR_ABORT_TASK_ENDED, None);
        assert!(
            ended.starts_with(
                "the guest ended (it exited or panicked) inside a fork-join task: a task must \
                 not end the program"
            ),
            "{ended}"
        );
        let ended_at = message(PAR_ABORT_TASK_ENDED, Some(in_task(FailureCause::GuestEnded)));
        assert!(
            ended_at.starts_with(
                "the guest ended (it exited or panicked) inside a fork-join task (task 3 of \
                 region 1): "
            ),
            "{ended_at}"
        );
        let faulted = message(PAR_ABORT_TASK_FAULTED, None);
        assert!(faulted.starts_with("the guest faulted inside a fork-join task"), "{faulted}");
        let region = message(PAR_ABORT_ERROR, None);
        assert!(
            region.starts_with(
                "a fork-join region failed in the assembly service: the service did not say how"
            ),
            "{region}"
        );
        let full = message(PAR_ABORT_ERROR, Some(in_task(FailureCause::ShmFull)));
        assert!(
            full.starts_with(
                "a fork-join region failed in the assembly service: task 3 of region 1: the \
                 task's process could not write its trace"
            ),
            "{full}"
        );
        for message in [ended, ended_at, faulted, region, full] {
            assert!(message.contains("ZISK_ASM_PAR_WORKERS=1 regions run one task after another"));
        }
    }

    fn applied_args(o: &AsmRunnerOptions, svc: AsmService) -> Vec<String> {
        let mut cmd = Command::new("ziskemuasm");
        o.apply_to_command(&mut cmd, &svc, "ZISK_1_0", "ZISK_1_h_0");
        cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect()
    }

    #[test]
    fn apply_to_command_emits_the_mandatory_flags() {
        let args = applied_args(&AsmRunnerOptions::new(), AsmService::MO);
        for expected in ["-s", "--gen=7", "--stdio", "--open_all_shm", "--share_input_shm"] {
            assert!(args.iter().any(|a| a == expected), "missing {expected} in {args:?}");
        }
        // prefixes are passed as flag + value pairs
        let i = args.iter().position(|a| a == "--shm_prefix").expect("--shm_prefix");
        assert_eq!(args[i + 1], "ZISK_1_0");
        let j = args.iter().position(|a| a == "--sem_prefix").expect("--sem_prefix");
        assert_eq!(args[j + 1], "ZISK_1_h_0");
        // gen index is per-service
        assert!(
            applied_args(&AsmRunnerOptions::new(), AsmService::MT).contains(&"--gen=1".to_string())
        );
        assert!(
            applied_args(&AsmRunnerOptions::new(), AsmService::RH).contains(&"--gen=2".to_string())
        );
    }

    #[test]
    fn apply_to_command_reflects_optional_flags() {
        let off = applied_args(&AsmRunnerOptions::new(), AsmService::MO);
        assert!(!off.iter().any(|a| a == "-v"
            || a == "-m"
            || a == "-o"
            || a == "-t"
            || a == "-tt"
            || a == "-k"));

        let on = applied_args(
            &AsmRunnerOptions::new()
                .with_verbose(true)
                .with_metrics(true)
                .with_log_output(true)
                .keccak_trace(true)
                .with_trace_level(AsmRunnerTraceLevel::ExtendedTrace),
            AsmService::MO,
        );
        for expected in ["-v", "-m", "-o", "-tt", "-k"] {
            assert!(on.iter().any(|a| a == expected), "missing {expected} in {on:?}");
        }
        // Trace (not ExtendedTrace) emits "-t", not "-tt".
        let t = applied_args(
            &AsmRunnerOptions::new().with_trace_level(AsmRunnerTraceLevel::Trace),
            AsmService::MO,
        );
        assert!(t.contains(&"-t".to_string()) && !t.contains(&"-tt".to_string()));
    }
}
