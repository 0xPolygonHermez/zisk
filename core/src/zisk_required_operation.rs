//! Data required to prove the different Zisk operations

use core::fmt;

/// Required data to make an operation.  
///
/// Stores the minimum information to reproduce an operation execution:
/// * The opcode and the a and b registers values (regardless of their sources)
/// * The step is also stored to keep track of the program execution point
///
/// This data is generated during the first emulation execution.
/// This data is required by the main state machine executor to generate the witness computation.
#[derive(Clone)]
pub struct ZiskRequiredOperation {
    pub step: u64,
    pub opcode: u8,
    pub a: u64,
    pub b: u64,
}

/// Stores the minimum information to generate the memory state machine witness computation.
#[derive(Clone)]
pub enum ZiskRequiredMemory {
    Basic { step: u64, value: u64, address: u32, is_write: bool, width: u8, step_offset: u8 },
    Extended { values: [u64; 2], address: u32 },
}

impl fmt::Debug for ZiskRequiredMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZiskRequiredMemory::Basic { step, value, address, is_write, width, step_offset: _ } => {
                let label = if *is_write { "WR" } else { "RD" };
                write!(
                    f,
                    "{0} addr:{1:#08X}({1}) offset:{5} width:{2} value:{3:#016X}({3}) step:{4}",
                    label,
                    address,
                    width,
                    value,
                    step,
                    address & 0x07
                )
            }
            ZiskRequiredMemory::Extended { values, address } => {
                write!(
                    f,
                    "addr:{1:#08X}({0}) value[1]:{1} value[2]:{2}",
                    address, values[0], values[1],
                )
            }
        }
    }
}

impl ZiskRequiredMemory {
    pub fn get_address(&self) -> u32 {
        match self {
            ZiskRequiredMemory::Basic {
                step: _,
                value: _,
                address,
                is_write: _,
                width: _,
                step_offset: _,
            } => *address,
            ZiskRequiredMemory::Extended { values: _, address } => *address,
        }
    }
}
