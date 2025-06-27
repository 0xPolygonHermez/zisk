//! Zisk emulator options

use clap::Parser;
use std::fmt;
use zisk_core::DEFAULT_MAX_STEPS_STR;

pub const ZISK_VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("VERGEN_GIT_SHA"),
    " ",
    env!("VERGEN_BUILD_TIMESTAMP"),
    ")"
);

/// ZisK emulator options structure
#[derive(Parser, Debug, Clone)]
#[command(version = ZISK_VERSION_MESSAGE, about, long_about = None)]
#[command(propagate_version = true)]
pub struct EmuOptions {
    /// Sets the Zisk ROM data file path
    #[clap(short, long, value_name = "ROM_FILE")]
    pub rom: Option<String>,
    /// Sets the ELF data file path, to be converted to ZisK ROM data
    #[clap(short, long, value_name = "ELF_FILE")]
    pub elf: Option<String>,
    /// Sets the input data file path
    #[clap(short, long, value_name = "INPUT_FILE")]
    pub inputs: Option<String>,
    /// Sets the output data file path
    #[clap(short, long, value_name = "OUTPUT_FILE")]
    pub output: Option<String>,
    /// Sets the maximum number of steps to execute.  Default value is 1000000000.  Configured with
    /// `-n`.
    #[clap(short = 'n', long, value_name = "MAX_STEPS", default_value = DEFAULT_MAX_STEPS_STR)]
    pub max_steps: u64,
    /// Sets the print step period in number of steps
    #[clap(short, long, value_name = "PRINT_STEP", default_value = "0")]
    pub print_step: Option<u64>,
    /// Sets the trace output file
    #[clap(short, long, value_name = "TRACE_FILE")]
    pub trace: Option<String>,
    /// Sets the verbose mode
    #[clap(short, long, value_name = "VERBOSE", default_value = "false")]
    pub verbose: bool,
    /// Sets the log step mode
    #[clap(short, long, value_name = "LOG_STEP", default_value = "false")]
    pub log_step: bool,
    /// Log the output to console. This option is set by default to true as a requirement to pass
    /// the riscof GHA tests.  Enabled with `-c`.
    #[clap(short = 'c', long, value_name = "LOG_OUTPUT", default_value = "true")]
    pub log_output: bool,
    /// Trace every this number of steps.
    pub chunk_size: Option<u64>,
    /// Log performance metrics.  Enabled with `-m`.
    #[clap(short = 'm', long, value_name = "LOG_METRICS", default_value = "false")]
    pub log_metrics: bool,
    /// Tracer v.  Enabled with `-a`.
    #[clap(short = 'a', long, value_name = "TRACERV", default_value = "false")]
    pub tracerv: bool,
    /// Generates statistics about opcodes and memory usage.  Enabled with `-x`.
    #[clap(short = 'x', long, value_name = "STATS", default_value = "false")]
    pub stats: bool,
    /// Generates minimal traces.  Enabled with `-g`.
    #[clap(short = 'g', long, value_name = "MINIMAL_TRACES", default_value = "false")]
    pub generate_minimal_traces: bool,
}

impl Default for EmuOptions {
    /// Default constructor for impl fmt::Display for EmuOptions structure
    fn default() -> Self {
        Self {
            rom: None,
            elf: None,
            inputs: None,
            output: None,
            max_steps: 0xFFFFFFFFFFFFFFFF,
            print_step: None,
            trace: None,
            verbose: false,
            log_step: false,
            log_output: false,
            chunk_size: None,
            log_metrics: false,
            tracerv: false,
            stats: false,
            generate_minimal_traces: false,
        }
    }
}

impl fmt::Display for EmuOptions {
    /// Formats a string with the configuration information
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "ROM: {:?}", self.rom)?;
        writeln!(f, "ELF: {:?}", self.elf)?;
        writeln!(f, "INPUT: {:?}", self.inputs)?;
        writeln!(f, "MAX_STEPS: {}", self.max_steps)?;
        writeln!(f, "PRINT_STEP: {:?}", self.print_step)?;
        writeln!(f, "TRACE: {:?}", self.trace)?;
        writeln!(f, "OUTPUT: {:?}", self.output)?;
        writeln!(f, "LOG_OUTPUT: {:?}", self.log_output)?;
        writeln!(f, "VERBOSE: {}", self.verbose)?;
        writeln!(f, "CHUNK_SIZE: {:?}", self.chunk_size)?;
        writeln!(f, "METRICS: {:?}", self.log_metrics)?;
        writeln!(f, "STATS: {:?}", self.stats)?;
        writeln!(f, "TRACERV: {:?}", self.tracerv)?;
        writeln!(f, "LOG_STEP: {:?}", self.log_step)?;
        writeln!(f, "MINIMAL_TRACES: {:?}", self.generate_minimal_traces)
    }
}

impl EmuOptions {
    /// Returns true if the configuration allows to emulate in fast mode, maximizing the performance
    pub fn is_fast(&self) -> bool {
        self.chunk_size.is_none()
            && (self.print_step.is_none() || (self.print_step.unwrap() == 0))
            && self.trace.is_none()
            && !self.log_step
            && !self.verbose
            && !self.tracerv
            && !self.stats
            && !self.generate_minimal_traces
    }
}
