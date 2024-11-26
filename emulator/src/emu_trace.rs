/// Trace data at the beginning of the program execution: pc, sp, c and step
#[derive(Default, Debug, Clone)]
pub struct EmuTraceStart {
    pub pc: u64,
    pub sp: u64,
    pub c: u64,
    pub step: u64,
}

/// Trace data at every step.  Only the values of registers a and b are required.
/// The current value of pc evolves starting at the start pc value, as we execute the ROM.
/// The value of c and flag can be obtained by executing the ROM instruction corresponding to the
/// current value of pc and taking a and b as the input.
#[derive(Default, Debug, Clone)]
pub struct EmuTraceStep {
    pub a: u64,
    pub b: u64,
}

/// Trace data at the end of the program execution, including only the end flag
/// If the end flag is true, the program executed completely.  This does not mean that the
/// program ended successfully; it could have found an error condition due to, for example, invalid
/// input data, and then jump directly to the end of the ROM.  In this error situation, the output
/// data should reveal the success or fail of the completed execution.  This table shows the
/// possible combinations:
///
/// - end = false  --> program did not complete, e.g. the emulator run out of steps (you can
///   configure more steps)
/// - end = true --> program completed
///     - output data correct --> program completed successfully
///     - output data incorrect --> program completed with an error
#[derive(Default, Debug, Clone)]
pub struct EmuTraceEnd {
    pub end: bool,
}

/// Trace data of a complete program execution (start, steps, and end) or of a segment of a program
/// execution (also includes last_state)
#[derive(Default, Debug, Clone)]
pub struct EmuTrace {
    pub start_state: EmuTraceStart,
    pub last_state: EmuTraceStart,
    pub steps: Vec<EmuTraceStep>,
    pub end: EmuTraceEnd,
}
