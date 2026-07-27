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
    Poseidon,
    Blake2,
    PubOut,
    ArithEq,
    ArithEq384,
    BigInt, // Note: Add new core operations here
    Dma,    // Note: To add extra params to precompiles calls
    // ZisK Free Input Operations
    FcallParam,
    Fcall,
    FcallGet,
    Profile,
}

pub const NONE_OP_TYPE_ID: u32 = ZiskOperationType::None as u32;
pub const INTERNAL_OP_TYPE_ID: u32 = ZiskOperationType::Internal as u32;
pub const ARITH_OP_TYPE_ID: u32 = ZiskOperationType::Arith as u32;
pub const BINARY_OP_TYPE_ID: u32 = ZiskOperationType::Binary as u32;
pub const BINARY_E_OP_TYPE_ID: u32 = ZiskOperationType::BinaryE as u32;
pub const KECCAK_OP_TYPE_ID: u32 = ZiskOperationType::Keccak as u32;
pub const SHA256_OP_TYPE_ID: u32 = ZiskOperationType::Sha256 as u32;
pub const POSEIDON_OP_TYPE_ID: u32 = ZiskOperationType::Poseidon as u32;
pub const PUB_OUT_OP_TYPE_ID: u32 = ZiskOperationType::PubOut as u32;
pub const ARITH_EQ_OP_TYPE_ID: u32 = ZiskOperationType::ArithEq as u32;
pub const ARITH_EQ_384_OP_TYPE_ID: u32 = ZiskOperationType::ArithEq384 as u32;
pub const BIG_INT_OP_TYPE_ID: u32 = ZiskOperationType::BigInt as u32;
pub const FCALL_PARAM_OP_TYPE_ID: u32 = ZiskOperationType::FcallParam as u32;
pub const FCALL_OP_TYPE_ID: u32 = ZiskOperationType::Fcall as u32;
pub const DMA_OP_TYPE_ID: u32 = ZiskOperationType::Dma as u32;
pub const BLAKE2_OP_TYPE_ID: u32 = ZiskOperationType::Blake2 as u32;

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
    pub store_pc: bool,
    pub store_use_sp: bool,
    pub store: u64,
    pub store_offset: i64,
    pub set_pc: bool,
    pub is_precompiled: bool,
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
    pub riscv_inst: Option<String>,
    pub index: u64, // internal field used for tracking the instruction creation order in the ROM
    pub next_internal_inst: Option<u64>, // connection to next internal odd instruction, if any
    pub external_ref_addr: Option<u64>, // external address of the instruction, if any
    pub meta_rs1: Option<u8>, // meta information used for callstack.
    pub meta_rd: Option<u8>, // meta information used for callstack.
}

/// Default constructor
/// Initializes all fields to 0
impl Default for ZiskInst {
    fn default() -> Self {
        Self {
            paddr: 0,
            store_pc: false,
            store_use_sp: false,
            store: 0,
            store_offset: 0,
            set_pc: false,
            is_precompiled: false,
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
            riscv_inst: None,
            index: 0,
            next_internal_inst: None,
            external_ref_addr: None,
            meta_rs1: None,
            meta_rd: None,
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
        if self.store_pc {
            s += &format!(" store_pc={}", self.store_pc);
        }
        if self.store_use_sp {
            s += &format!(" store_use_sp={}", self.store_use_sp);
        }
        if self.set_pc {
            s += &format!(" set_pc={}", self.set_pc);
        }
        if self.is_precompiled {
            s += &format!(" op_with_step={}", self.is_precompiled);
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
        s += &format!(" index={}", self.index);
        if let Some(next_internal_inst) = self.next_internal_inst {
            s += &format!(" next_internal_inst=0x{:x}", next_internal_inst);
        }
        if let Some(meta_reg) = self.meta_rs1 {
            s += &format!(" meta_rs1=0x{:x}", meta_reg);
        }
        if let Some(meta_reg) = self.meta_rd {
            s += &format!(" meta_rd=0x{:x}", meta_reg);
        }
        s.remove(0); // remove first space
        s
    }

    /// Generates a single line of ZisK assembly code representing this instruction,
    /// following the syntax specified in `ziskasm/ziskasm.md`.
    ///
    /// The general format is:
    /// `operation(a_source, b_source) -> c_storage, j(jump1, jump2), setpc(jump), sp, end`
    /// where the `-> c_storage`, `j(...)`/`setpc(...)`, `sp` and `end` parts are optional.
    pub fn to_zisk_asm(&self) -> String {
        // a_source
        let a_source = match self.a_src {
            SRC_C => "c".to_string(),
            SRC_REG => format!("r{}", self.a_offset_imm0),
            SRC_MEM => format!("[0x{:x}]", self.a_offset_imm0),
            SRC_IMM => format!("0x{:x}", self.a_offset_imm0 | (self.a_use_sp_imm1 << 32)),
            SRC_STEP => "step".to_string(),
            _ => format!("<invalid a_src={}>", self.a_src),
        };

        // b_source
        let b_source = match self.b_src {
            SRC_C => "c".to_string(),
            SRC_REG => format!("r{}", self.b_offset_imm0),
            SRC_MEM => format!("[0x{:x}]", self.b_offset_imm0),
            SRC_IMM => format!("0x{:x}", self.b_offset_imm0 | (self.b_use_sp_imm1 << 32)),
            SRC_IND => Self::ind_operand(self.ind_width, self.b_offset_imm0 as i64),
            _ => format!("<invalid b_src={}>", self.b_src),
        };

        // operation(a_source, b_source)
        let mut s = format!("{}({}, {})", self.op_str, a_source, b_source);

        // -> c_storage (optional; omitted when the result is not stored)
        match self.store {
            STORE_NONE => {}
            STORE_REG => s += &format!(" -> r{}", self.store_offset),
            STORE_MEM => s += &format!(" -> [0x{:x}]", self.store_offset as u64),
            STORE_IND => {
                s += &format!(" -> {}", Self::ind_operand(self.ind_width, self.store_offset))
            }
            _ => s += &format!(" -> <invalid store={}>", self.store),
        }

        // , setpc(jump)  or  , j(jump1, jump2)  (mutually exclusive: see get_next_pc)
        if self.set_pc {
            s += &format!(", setpc({})", self.jmp_offset1);
        } else if self.jmp_offset1 == 4 && self.jmp_offset2 == 4 {
            // Default fall-through (next_pc = current_pc + 4): the jump field is omitted.
        } else if self.jmp_offset2 == 4 {
            // jump2 is the default next instruction, so it can be omitted.
            s += &format!(", j({})", self.jmp_offset1);
        } else {
            s += &format!(", j({}, {})", self.jmp_offset1, self.jmp_offset2);
        }

        // , sp (optional): only affects memory-addressed operands
        let uses_sp = (self.a_src == SRC_MEM && self.a_use_sp_imm1 != 0)
            || ((self.b_src == SRC_MEM || self.b_src == SRC_IND) && self.b_use_sp_imm1 != 0)
            || ((self.store == STORE_MEM || self.store == STORE_IND) && self.store_use_sp);
        if uses_sp {
            s += ", sp";
        }

        // , end (optional)
        if self.end {
            s += ", end";
        }

        s
    }

    /// Formats an indirect (`W[a + N]`) memory operand per the ziskasm spec: `W` is the
    /// access width in bytes and `N` is the signed address offset relative to register `a`.
    fn ind_operand(width: u64, offset: i64) -> String {
        if offset < 0 {
            format!("{}[a - {}]", width, -offset)
        } else {
            format!("{}[a + {}]", width, offset)
        }
    }

    /// Constructs a `flags`` bitmap made of combinations of fields of the Zisk instruction.  This
    /// field is used by the PIL to proof some of the operations.
    pub fn get_flags(&self) -> u64 {
        let flags: u64 = 1
            | (((self.a_src == SRC_IMM) as u64) << 1)
            | (((self.a_src == SRC_MEM) as u64) << 2)
            | ((self.is_precompiled as u64) << 3)
            | (((self.b_src == SRC_IMM) as u64) << 4)
            | (((self.b_src == SRC_MEM) as u64) << 5)
            | ((self.is_external_op as u64) << 6)
            | ((self.store_pc as u64) << 7)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Reconstructs the instruction from the ziskasm example (the transpiled
    /// `jalr r0, r1, 0x0`) and checks it renders to the expected ZisK asm line.
    #[test]
    fn to_zisk_asm_jalr_example() {
        let inst = ZiskInst {
            a_src: SRC_IMM,
            a_use_sp_imm1: 0xffffffff,
            a_offset_imm0: 0xfffffffe,
            b_src: SRC_REG,
            b_offset_imm0: 0x1,
            op: 14,
            op_str: "and",
            set_pc: true,
            jmp_offset2: 4,
            is_external_op: true,
            ..Default::default()
        };
        assert_eq!(inst.to_zisk_asm(), "and(0xfffffffffffffffe, r1), setpc(0)");
    }

    #[test]
    fn to_zisk_asm_covers_each_field() {
        // add(r5, 0x10) -> r6 : register + immediate sources, register store.
        let inst = ZiskInst {
            a_src: SRC_REG,
            a_offset_imm0: 5,
            b_src: SRC_IMM,
            b_offset_imm0: 0x10,
            op_str: "add",
            store: STORE_REG,
            store_offset: 6,
            jmp_offset1: 4,
            jmp_offset2: 4,
            ..Default::default()
        };
        assert_eq!(inst.to_zisk_asm(), "add(r5, 0x10) -> r6");

        // A load feeding a store to memory, with an indirect source and a taken branch.
        let inst = ZiskInst {
            a_src: SRC_REG,
            a_offset_imm0: 2,
            b_src: SRC_IND,
            b_offset_imm0: (-8_i64) as u64,
            ind_width: 8,
            op_str: "copyb",
            store: STORE_MEM,
            store_offset: 0xa0000000,
            jmp_offset1: -16,
            jmp_offset2: 4,
            end: true,
            ..Default::default()
        };
        assert_eq!(
            inst.to_zisk_asm(),
            "copyb(r2, 8[a - 8]) -> [0xa0000000], j(-16), end"
        );
    }
}
