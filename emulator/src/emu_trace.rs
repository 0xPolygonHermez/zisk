//! Emulator trace

/// Trace data at the beginning of the program execution: pc, sp, c and step.
#[derive(Default, Debug, Clone)]
pub struct EmuTraceStart {
    /// Value of the program counter (ROM address)
    pub pc: u64,
    /// Value of the sp register
    pub sp: u64,
    /// Value of the c register
    pub c: u64,
    /// Value of the step
    pub step: u64,
    /// Value of the registers
    pub regs: [u64; 32],
}

/// Trace data of a complete program execution (start, steps, and end) or of a segment of a program
/// execution (also includes last_state).
#[derive(Default, Debug, Clone)]
pub struct EmuTrace {
    /// State at the begining of the execution
    pub start_state: EmuTraceStart,

    /// State at the end of the execution
    pub last_state: EmuTraceStart,

    /// Number of steps executed
    pub steps: u64,

    /// Memory reads
    pub mem_reads: Vec<u64>,

    /// Index of the last executed step in memory reads.
    pub last_mem_reads_index: usize,

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
    pub end: bool,
}
