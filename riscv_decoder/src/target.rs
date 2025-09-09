/// Extension outlines the extensions supported by this decoder
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Extension {
    /// RV32I - Base integer instruction set
    RV32I,
    /// RV64I - 64-bit extensions to base
    RV64I,
    /// RV32M - Integer multiply/divide
    RV32M,
    /// RV64M - 64-bit multiply/divide  
    RV64M,
    /// RV32A - Atomic instructions
    RV32A,
    /// RV64A - 64-bit atomic instructions
    RV64A,
    ///// Zicsr - Control and Status Register instructions
    // Zicsr,
    // /// Zifencei - Instruction-fetch fence
    // Zifencei,
    // /// Zicntr - Counter extension (performance counters)
    // Zicntr,
    // /// Zihpm - Hardware Performance Monitors extension
    // Zihpm,
    // /// RV32F - Single-precision floating point
    // RV32F,
    // /// RV64F - 64-bit single-precision floating point
    // RV64F,
    // /// RV32D - Double-precision floating point
    // RV32D,
    // /// RV64D - 64-bit double-precision floating point
    // RV64D,
    /// RVC - Compressed instruction extension
    RVC,
}
