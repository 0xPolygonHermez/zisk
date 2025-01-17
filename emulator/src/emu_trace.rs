//! Emulator trace

/// Trace data at the beginning of the program execution: pc, sp, c and step.
#[derive(Default, Debug, Clone)]
pub struct EmuTraceStart {
    /// Initial value of the program counter (ROM address)
    pub pc: u64,
    /// Initial value of the sp register
    pub sp: u64,
    /// Initial value of the c register
    pub c: u64,
    /// Initial value of the step
    pub step: u64,
    pub regs: [u64; 32],
    pub mem_reads_index: usize,
}

/// Trace data at every step.
///
/// Only the values of registers a and b are required.
/// The current value of pc evolves starting at the start pc value, as we execute the ROM.
/// The value of c and flag can be obtained by executing the ROM instruction corresponding to the
/// current value of pc and taking a and b as the input.
#[derive(Default, Debug, Clone)]
pub struct EmuTraceSteps {
    pub mem_reads: Vec<u64>,
    pub steps: u64,
}

/// Trace data at the end of the program execution, including only the `end` flag.
///
/// If the `end` flag is true, the program executed completely.
/// This does not mean that the program ended successfully; it could have found an error condition
/// due to, for example, invalid input data, and then jump directly to the end of the ROM.
/// In this error situation, the output data should reveal the success or fail of the completed
/// execution.
/// These are the possible combinations:
/// * end = false  --> program did not complete, e.g. the emulator run out of steps (you can
///   configure more steps)
/// * end = true --> program completed
///   * output data correct --> program completed successfully
///   * output data incorrect --> program completed with an error
#[derive(Default, Debug, Clone)]
pub struct EmuTraceEnd {
    /// Value of the `end` flag at the end of the execution
    pub end: bool,
}

/// Trace data of a complete program execution (start, steps, and end) or of a segment of a program
/// execution (also includes last_state).
#[derive(Default, Debug, Clone)]
pub struct EmuTrace {
    /// State at the begining of the execution
    pub start_state: EmuTraceStart,
    /// State at the end of the execution
    pub last_state: EmuTraceStart,
    pub steps: EmuTraceSteps,
    pub end: EmuTraceEnd,
}
