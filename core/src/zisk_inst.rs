//! Zisk instruction
//!
//! * A Zisk instruction performs an operation defined by its opcode (u8) over 2 input parameters a
//!   (u64) and b (u64) that gives as a result a dupla of c (u64) and flag (boolean).
//! * The a and b registers have their corresponding source, a procedure to build their value before
//!   calling the operation function.
//! * The c register has a store, a procedure to store its value after having called the operation
//!   function.
//! * Only one Zisk instruction is executed at every step of the program execution.
//! * In essence, a Zisk instruction is an execution step such that `(c,flag) = op(a,b)`.
//!
//! # Zisk register source
//!
//! The SRC_x definitions are used to specify the source of a or b registers, i.e. how to get
//! their values before calling the operation of the instruction.
//!
//! | Source   | Register(s) | Value                                                    |
//! |----------|-------------|----------------------------------------------------------|
//! | SRC_C    | a and b     | Current value of the c register                          |
//! | SRC_REG  | a and b     | Value read from current register at a constant index     |
//! | SRC_MEM  | a and b     | Value read from current memory at a constant address     |
//! | SRC_IMM  | a and b     | Constant (immediate) value                               |
//! | SRC_STEP | a           | Current execution step                                   |
//! | SRC_IND  | b           | Value read from current memory at indirect address a + b |
//!
//! # Zisk register store
//!
//! The STORE_x definitions are used to specify the storage of the c register, i.e. how to store
//! its value after calling the operation of the instruction.
//!
//! | Store      | Register | Storage                                                     |
//! |------------|----------|-------------------------------------------------------------|
//! | STORE_NONE | c        | Value is not stored anywhere                                |
//! | STORE_REG  | c        | Value is stored in register at a constant index             |
//! | STORE_MEM  | c        | Value is stored in memory at a constant address             |
//! | STORE_IND  | c        | value is stored in memory at an indirect address a + offset |

use crate::{source_to_str, store_to_str, InstContext};

/// a or b registers source is the current value of the c register
pub const SRC_C: u64 = 0;
/// a or b registers source is value read from memory at a constant address
pub const SRC_MEM: u64 = 1;
/// a or b registers source is a constant (immediate) value
pub const SRC_IMM: u64 = 2;
/// a register source is the current execution step
pub const SRC_STEP: u64 = 3;
// #[cfg(feature = "sp")]
// pub const SRC_SP: u64 = 4;
/// b register source is value read from memory at an indirect address a + b
pub const SRC_IND: u64 = 5;
/// a or b registers source is value read from register at a constant index
pub const SRC_REG: u64 = 6;

/// c register value is not stored anywhere
pub const STORE_NONE: u64 = 0;
/// c register value is stored in memory at a constant address
pub const STORE_MEM: u64 = 1;
/// c register value is stored in memory at an indirect address a + offset
pub const STORE_IND: u64 = 2;
/// c register value is stored stored in register at a constant index
pub const STORE_REG: u64 = 3;

/// Describes the type of the Zisk opcode.
///
/// This type determines how the operation result will be proven.
/// Internal operations are proven as part of the main state machine itself, given their
/// simplicity. External operations (rest of types) are proven in their corresponding secondary
/// state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd)]
#[repr(u32)]
pub enum ZiskOperationType {
    None,
    Internal,
    // ZisK Core Operations
    Arith,
    Binary,
    BinaryE,
    Keccak,
    Sha256,
    PubOut,
    ArithEq,
    ArithEq384,
    BigInt, // Note: Add new core operations here
    // ZisK Free Input Operations
    FcallParam,
    Fcall,
    FcallGet,
}

pub const NONE_OP_TYPE_ID: u32 = ZiskOperationType::None as u32;
pub const INTERNAL_OP_TYPE_ID: u32 = ZiskOperationType::Internal as u32;
pub const ARITH_OP_TYPE_ID: u32 = ZiskOperationType::Arith as u32;
pub const BINARY_OP_TYPE_ID: u32 = ZiskOperationType::Binary as u32;
pub const BINARY_E_OP_TYPE_ID: u32 = ZiskOperationType::BinaryE as u32;
pub const KECCAK_OP_TYPE_ID: u32 = ZiskOperationType::Keccak as u32;
pub const SHA256_OP_TYPE_ID: u32 = ZiskOperationType::Sha256 as u32;
pub const PUB_OUT_OP_TYPE_ID: u32 = ZiskOperationType::PubOut as u32;
pub const ARITH_EQ_OP_TYPE_ID: u32 = ZiskOperationType::ArithEq as u32;
pub const ARITH_EQ_384_OP_TYPE_ID: u32 = ZiskOperationType::ArithEq384 as u32;
pub const BIG_INT_OP_TYPE_ID: u32 = ZiskOperationType::BigInt as u32;
pub const FCALL_PARAM_OP_TYPE_ID: u32 = ZiskOperationType::FcallParam as u32;
pub const FCALL_OP_TYPE_ID: u32 = ZiskOperationType::Fcall as u32;
pub const FCALL_GET_OP_TYPE_ID: u32 = ZiskOperationType::FcallGet as u32;

/// ZisK instruction definition
///
/// ZisK instructions are defined as a binary operation with 2 results: op(a, b) -> (c, flag)
/// a, b and c are u64 registers; flag is a boolean.
/// a and b are loaded from the respective sources specified in the instruction.
/// c is stored according to the destination specified in the instruction.
/// flag meaning is operation-dependant.
#[derive(Debug, Clone)]
pub struct ZiskInst {
    pub paddr: u64,
    pub store_ra: bool,
    pub store_use_sp: bool,
    pub store: u64,
    pub store_offset: i64,
    pub set_pc: bool,
    // #[cfg(feature = "sp")]
    // pub set_sp: bool,
    pub ind_width: u64,
    // #[cfg(feature = "sp")]
    // pub inc_sp: u64,
    pub end: bool,
    pub a_src: u64,
    pub a_use_sp_imm1: u64,
    pub a_offset_imm0: u64,
    pub b_src: u64,
    pub b_use_sp_imm1: u64,
    pub b_offset_imm0: u64,
    pub jmp_offset1: i64,
    pub jmp_offset2: i64,
    pub is_external_op: bool,
    pub op: u8,
    pub func: fn(&mut InstContext) -> (),
    pub op_str: &'static str,
    pub op_type: ZiskOperationType,
    pub verbose: String,
    pub m32: bool,
    pub input_size: u64,
    pub sorted_pc_list_index: usize,
}

/// Default constructor
/// Initializes all fields to 0
impl Default for ZiskInst {
    fn default() -> Self {
        Self {
            paddr: 0,
            store_ra: false,
            store_use_sp: false,
            store: 0,
            store_offset: 0,
            set_pc: false,
            // #[cfg(feature = "sp")]
            // set_sp: false,
            ind_width: 0,
            // #[cfg(feature = "sp")]
            // inc_sp: 0,
            end: false,
            a_src: 0,
            a_use_sp_imm1: 0,
            a_offset_imm0: 0,
            b_src: 0,
            b_use_sp_imm1: 0,
            b_offset_imm0: 0,
            jmp_offset1: 0,
            jmp_offset2: 0,
            is_external_op: false,
            op: 0,
            func: |_| (),
            op_str: "",
            op_type: ZiskOperationType::None,
            verbose: String::new(),
            m32: false,
            input_size: 0,
            sorted_pc_list_index: 0,
        }
    }
}

impl ZiskInst {
    /// Creates a human-readable string containing the ZisK instruction fields that are not zero.
    /// Used only for debugging.
    pub fn to_text(&self) -> String {
        let mut s = String::new();
        if self.paddr != 0 {
            s += &format!(" paddr=0x{:x}", self.paddr);
        }
        if !self.verbose.is_empty() {
            s += &format!(" verbose={}", self.verbose);
        }
        s += &format!(" a_src={}={}", self.a_src, source_to_str(self.a_src));
        if self.a_use_sp_imm1 != 0 {
            s += &format!(" a_use_sp_imm1=0x{:x}", self.a_use_sp_imm1);
        }
        if self.a_offset_imm0 != 0 {
            s += &format!(" a_offset_imm0=0x{:x}", self.a_offset_imm0);
        }
        s += &format!(" b_src={}={}", self.b_src, source_to_str(self.b_src));
        if self.b_use_sp_imm1 != 0 {
            s += &format!(" b_use_sp_imm1=0x{:x}", self.b_use_sp_imm1);
        }
        if self.b_offset_imm0 != 0 {
            s += &format!(" b_offset_imm0=0x{:x}", self.b_offset_imm0);
        }
        if self.ind_width != 0 {
            s += &format!(" ind_width={}", self.ind_width);
        }
        {
            s += &format!(" op={}={}", self.op, self.op_str);
        }
        if self.store != 0 {
            s += &format!(" store={}={}", self.store, store_to_str(self.store));
        }
        if self.store_offset != 0 {
            s += &format!(" store_offset=0x{:x}", self.store_offset as u64);
        }
        if self.store_ra {
            s += &format!(" store_ra={}", self.store_ra);
        }
        if self.store_use_sp {
            s += &format!(" store_use_sp={}", self.store_use_sp);
        }
        if self.set_pc {
            s += &format!(" set_pc={}", self.set_pc);
        }
        if self.jmp_offset1 != 0 {
            s += &format!(" jmp_offset1={}", self.jmp_offset1);
        }
        if self.jmp_offset2 != 0 {
            s += &format!(" jmp_offset2={}", self.jmp_offset2);
        }
        // #[cfg(feature = "sp")]
        // if self.set_sp {
        //     s += &(" set_sp=".to_string() + &self.set_sp.to_string());
        // }
        // #[cfg(feature = "sp")]
        // if self.inc_sp != 0 {
        //     s += &(" inc_sp=".to_string() + &self.inc_sp.to_string());
        // }
        if self.end {
            s += &format!(" end={}", self.end);
        }
        if self.is_external_op {
            s += &format!(" is_external_op={}", self.is_external_op);
        }
        if self.m32 {
            s += &format!(" m32={}", self.m32);
        }
        s
    }

    /// Constructs a `flags`` bitmap made of combinations of fields of the Zisk instruction.  This
    /// field is used by the PIL to proof some of the operations.
    pub fn get_flags(&self) -> u64 {
        let flags: u64 = 1
            | (((self.a_src == SRC_IMM) as u64) << 1)
            | (((self.a_src == SRC_MEM) as u64) << 2)
            | (((self.a_src == SRC_STEP) as u64) << 3)
            | (((self.b_src == SRC_IMM) as u64) << 4)
            | (((self.b_src == SRC_MEM) as u64) << 5)
            | ((self.is_external_op as u64) << 6)
            | ((self.store_ra as u64) << 7)
            | (((self.store == STORE_MEM) as u64) << 8)
            | (((self.store == STORE_IND) as u64) << 9)
            | ((self.set_pc as u64) << 10)
            | ((self.m32 as u64) << 11)
            | (((self.b_src == SRC_IND) as u64) << 12)
            | (((self.a_src == SRC_REG) as u64) << 13)
            | (((self.b_src == SRC_REG) as u64) << 14)
            | (((self.store == STORE_REG) as u64) << 15);

        flags
    }
}
