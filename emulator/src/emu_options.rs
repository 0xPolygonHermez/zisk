use clap::Parser;
use std::fmt;

/// ZisK emulator options structure
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
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
    pub input: Option<String>,
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
}

/// Default constructor for impl fmt::Display for EmuOptions structure
impl Default for EmuOptions {
    fn default() -> Self {
        Self {
            rom: None,
            elf: None,
            input: None,
            output: None,
            max_steps: 0xFFFFFFFFFFFFFFFF,
            print_step: None,
            trace: None,
            verbose: false,
            log_step: false,
            log_output: false,
        }
    }
}

impl fmt::Display for EmuOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ROM: {:?}\nELF: {:?}\nINPUT: {:?}\nMAX_STEPS: {}\nPRINT_STEP: {:?}\nTRACE: {:?}\nOUTPUT: {:?}\nVERBOSE: {}",
            self.rom, self.elf, self.input, self.max_steps, self.print_step, self.trace, self.output, self.verbose
        )
    }
}
