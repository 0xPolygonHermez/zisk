use clap::Parser;
use std::fmt;

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
    /// Sets the maximum number of steps to execute
    #[clap(short = 'n', long, value_name = "MAX_STEPS", default_value = "100000000")]
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
    /// Log the output to console
    #[clap(short = 'c', long, value_name = "LOG_OUTPUT", default_value = "true")]
    pub log_output: bool,
    /// Trace every this number of steps
    pub trace_steps: Option<u64>,
    /// Log performance metrics
    #[clap(short = 'm', long, value_name = "LOG_METRICS", default_value = "false")]
    pub log_metrics: bool,
}

/// Default constructor for impl fmt::Display for EmuOptions structure
impl Default for EmuOptions {
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
            trace_steps: None,
            log_metrics: false,
        }
    }
}

impl fmt::Display for EmuOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ROM: {:?}\nELF: {:?}\nINPUT: {:?}\nMAX_STEPS: {}\nPRINT_STEP: {:?}\nTRACE: {:?}\nOUTPUT: {:?}\nVERBOSE: {}",
            self.rom, self.elf, self.inputs, self.max_steps, self.print_step, self.trace, self.output, self.verbose
        )
    }
}
