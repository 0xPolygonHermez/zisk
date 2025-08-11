use static_assertions::const_assert;
use zisk_core::zisk_ops::ZiskOp;

const OP_SIGNEXTENDB: u8 = ZiskOp::SignExtendB.code();
const OP_SIGNEXTENDH: u8 = ZiskOp::SignExtendH.code();
const OP_SIGNEXTENDW: u8 = ZiskOp::SignExtendW.code();
const OP_ADD: u8 = ZiskOp::Add.code();
const OP_ADDW: u8 = ZiskOp::AddW.code();
const OP_SUB: u8 = ZiskOp::Sub.code();
const OP_SUBW: u8 = ZiskOp::SubW.code();
const OP_SLL: u8 = ZiskOp::Sll.code();
const OP_SLLW: u8 = ZiskOp::SllW.code();
const OP_SRA: u8 = ZiskOp::Sra.code();
const OP_SRL: u8 = ZiskOp::Srl.code();
const OP_SRAW: u8 = ZiskOp::SraW.code();
const OP_SRLW: u8 = ZiskOp::SrlW.code();
const OP_EQ: u8 = ZiskOp::Eq.code();
const OP_EQW: u8 = ZiskOp::EqW.code();
const OP_LTU: u8 = ZiskOp::Ltu.code();
const OP_LT: u8 = ZiskOp::Lt.code();
const OP_LTUW: u8 = ZiskOp::LtuW.code();
const OP_LTW: u8 = ZiskOp::LtW.code();
const OP_LEU: u8 = ZiskOp::Leu.code();
const OP_LE: u8 = ZiskOp::Le.code();
const OP_LEUW: u8 = ZiskOp::LeuW.code();
const OP_LEW: u8 = ZiskOp::LeW.code();
const OP_AND: u8 = ZiskOp::And.code();
const OP_OR: u8 = ZiskOp::Or.code();
const OP_XOR: u8 = ZiskOp::Xor.code();
const OP_MULU: u8 = ZiskOp::Mulu.code();
const OP_MULUH: u8 = ZiskOp::Muluh.code();
const OP_MULSUH: u8 = ZiskOp::Mulsuh.code();
const OP_MUL: u8 = ZiskOp::Mul.code();
const OP_MULH: u8 = ZiskOp::Mulh.code();
const OP_MULW: u8 = ZiskOp::MulW.code();
const OP_DIVU: u8 = ZiskOp::Divu.code();
const OP_REMU: u8 = ZiskOp::Remu.code();
const OP_DIV: u8 = ZiskOp::Div.code();
const OP_REM: u8 = ZiskOp::Rem.code();
const OP_DIVUW: u8 = ZiskOp::DivuW.code();
const OP_REMUW: u8 = ZiskOp::RemuW.code();
const OP_DIVW: u8 = ZiskOp::DivW.code();
const OP_REMW: u8 = ZiskOp::RemW.code();

const LOW_VALUES_OPCODES: [u8; 40] = [
    OP_SIGNEXTENDB,
    OP_SIGNEXTENDH,
    OP_SIGNEXTENDW,
    OP_ADD,
    OP_ADDW,
    OP_SUB,
    OP_SUBW,
    OP_SLL,
    OP_SLLW,
    OP_SRA,
    OP_SRL,
    OP_SRAW,
    OP_SRLW,
    OP_EQ,
    OP_EQW,
    OP_LTU,
    OP_LT,
    OP_LTUW,
    OP_LTW,
    OP_LEU,
    OP_LE,
    OP_LEUW,
    OP_LEW,
    OP_AND,
    OP_OR,
    OP_XOR,
    OP_MULU,
    OP_MULUH,
    OP_MULSUH,
    OP_MUL,
    OP_MULH,
    OP_MULW,
    OP_DIVU,
    OP_REMU,
    OP_DIV,
    OP_REM,
    OP_DIVUW,
    OP_REMUW,
    OP_DIVW,
    OP_REMW,
];

const MAX_A_LOW_VALUE: u64 = 386;
const MAX_B_LOW_VALUE: u64 = 386;
const LOW_VALUE_SIZE: usize = (MAX_A_LOW_VALUE * MAX_B_LOW_VALUE) as usize;
const MINUS_ONE: u64 = -1i64 as u64;
const MAX_U64: u64 = 0xFFFF_FFFF_FFFF_FFFF;
const EQ_OP_B_ZERO_A_LIMIT: u64 = 0xFFFFF;
const LTU_OP_B_LT_ONE_FROM: u64 = -128i64 as u64;

// LT

const LT_FROM_ADDR: u64 = 0xA010_0000;
const LT_TO_ADDR: u64 = 0xA012_0000;
const LT_DELTA: u64 = 8;
const LT_LOW_DISTANCE_1: u64 = 16; // 0 - 15
const LT_HIGH_DISTANCE_8: u64 = 240; // 16,24,32,40,.....
const LT_LOW_HIGH_DISTANCES: u64 = LT_LOW_DISTANCE_1 + LT_HIGH_DISTANCE_8;
const LT_MAX_DISTANCE: u64 = LT_LOW_DISTANCE_1 + (LT_HIGH_DISTANCE_8 - 1) * 8;
const LT_FROM_TO_SIZE: usize = ((LT_TO_ADDR - LT_FROM_ADDR) / LT_DELTA) as usize;
const LT_ALL_FROM_TO_SIZE: usize = LT_FROM_TO_SIZE * LT_LOW_HIGH_DISTANCES as usize;

const LT_ZERO_TO_B: u64 = 0x10000;

// ADD
const MAX_ADD_MINUS_ONE: u64 = 24628;
const MAX_ADD_MINUS_A: u64 = 1024;
const MAX_ADD_MINUS_B: u64 = 8;
const ADD_ONE_FROM_ADDR: u64 = 0xA010_0000; // address
const ADD_ONE_TO_ADDR: u64 = 0xA020_0000;
const ADD_EIGHT_FROM_ADDR: u64 = 0xA010_0000; // address
const ADD_EIGHT_TO_ADDR: u64 = 0xA020_0000;
const ADD_EIGHT_FROM_CODE: u64 = 0x8000_0000; // address
const ADD_EIGHT_TO_CODE: u64 = 0x8080_0000;
const ADD_EIGHT_STEP: u64 = 8;

const ADD_ZERO_FROM_ADDR: u64 = 0xA010_0000; // address
const ADD_ZERO_TO_ADDR: u64 = 0xA020_0000;
const ADD_ZERO_STEP: u64 = 8;

const ADD_MINUS_ONE_SIZE: usize = MAX_ADD_MINUS_ONE as usize;
const ADD_MINUS_A_B_SIZE: usize = (MAX_ADD_MINUS_A * MAX_ADD_MINUS_B) as usize;
const ADD_MINUS_A_B_FROM_B: u64 = MINUS_ONE - MAX_ADD_MINUS_B;

const ADD_ONE_ADDR_SIZE: usize = (ADD_ONE_TO_ADDR - ADD_ONE_FROM_ADDR) as usize;
const ADD_EIGHT_ADDR_SIZE: usize =
    ((ADD_EIGHT_TO_ADDR - ADD_EIGHT_FROM_ADDR) / ADD_EIGHT_STEP) as usize;
const ADD_EIGHT_CODE_SIZE: usize =
    ((ADD_EIGHT_TO_CODE - ADD_EIGHT_FROM_CODE) / ADD_EIGHT_STEP) as usize;
const ADD_ZERO_ADDR_SIZE: usize =
    ((ADD_ZERO_TO_ADDR - ADD_ZERO_FROM_ADDR) / ADD_ZERO_STEP) as usize;

const ADD_MINUS_ONE_OFFSET: usize = LOW_VALUE_SIZE;
const ADD_MINUS_A_B_OFFSET: usize = ADD_MINUS_ONE_OFFSET + ADD_MINUS_ONE_SIZE;
const ADD_ONE_ADDR_OFFSET: usize = ADD_MINUS_A_B_OFFSET + ADD_MINUS_A_B_SIZE;
const ADD_EIGHT_ADDR_OFFSET: usize = ADD_ONE_ADDR_OFFSET + ADD_ONE_ADDR_SIZE;
const ADD_EIGHT_CODE_OFFSET: usize = ADD_EIGHT_ADDR_OFFSET + ADD_EIGHT_ADDR_SIZE;
const ADD_ZERO_ADDR_OFFSET: usize = ADD_EIGHT_CODE_OFFSET + ADD_EIGHT_CODE_SIZE;

// AND
const AND_CODE_ADDR_FROM: u64 = 0x8000_0000;
const AND_CODE_ADDR_TO: u64 = 0x8090_0000; // address
const AND_CODE_ADDR_STEP: u64 = 4;
const AND_CODE_ADDR_MASK: u64 = 0xFFFF_FFFF_FFFF_FFFC;

const AND_RESET_LAST_THREE_BITS_B: u64 = 0xFFFF_FFFF_FFFF_FFF8;
const AND_RESET_LAST_THREE_BITS_A_TO: u64 = 1024;
const AND_GET_LAST_THREE_BITS_B: u64 = 0x7;
const AND_GET_LAST_THREE_BITS_FROM: u64 = 0xA010_0000;
const AND_GET_LAST_THREE_BITS_TO: u64 = 0xA020_0000;
const AND_GET_LAST_THREE_BITS_STEP: u64 = 8;

const AND_CODE_ADDR_OFFSET: usize = LOW_VALUE_SIZE;
const AND_CODE_ADDR_SIZE: usize =
    ((AND_CODE_ADDR_TO - AND_CODE_ADDR_FROM) / AND_CODE_ADDR_STEP) as usize;

const AND_RESET_LAST_THREE_BITS_OFFSET: usize = AND_CODE_ADDR_OFFSET + AND_CODE_ADDR_SIZE;
const AND_RESET_LAST_THREE_BITS_SIZE: usize = AND_RESET_LAST_THREE_BITS_A_TO as usize;

const AND_GET_LAST_THREE_BITS_OFFSET: usize =
    AND_RESET_LAST_THREE_BITS_OFFSET + AND_RESET_LAST_THREE_BITS_SIZE;
const AND_GET_LAST_THREE_BITS_SIZE: usize = ((AND_GET_LAST_THREE_BITS_TO
    - AND_GET_LAST_THREE_BITS_FROM)
    / AND_GET_LAST_THREE_BITS_STEP) as usize;

const OR_TO_A: u64 = 0x1000;
const OR_TO_B: u64 = 16;

const SLR_MASK_FROM: u64 = 0xFFFF_FFFF_FFFF_F000;
const SLR_TO_B: u64 = 64;

const SUB_W_ADDR_FROM: u64 = 0xA010_0000;
const SUB_W_ADDR_TO: u64 = 0xA020_0000;
const SUB_W_ADDR_STEP: u64 = 4;

const SUB_TO_A: u64 = 4192;
const SUB_TO_B: u64 = 8;

// table autogenerated with FrequentOpsTable::print_table_offsets()
// this table is used to calculate the offset (row) of each operation
const OP_TABLE_OFFSETS: [usize; 192] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 149124, 0, 4557574, 5755146, 8296258, 8479508, 8628504, 8777500,
    11417888, 11629954, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 11778952,
    11927948, 0, 12076944, 12225940, 12374936, 12786076, 12935072, 0, 13084068, 13233064, 13648300,
    13797296, 13946292, 14095288, 14244284, 14393280, 14542276, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 14691272, 14840268, 0, 14989264, 15138260, 15287256, 15436252, 0,
    15585248, 15734244, 15883240, 16032236, 16181232, 16330228, 16479224, 16628220,
];

#[derive(Debug, Clone)]
pub struct FrequentOpsTable {
    table_by_op: [usize; 256],
    table_ops: Vec<Vec<[u64; 2]>>,
}

const FREQUENT_OP_EMPTY: usize = 256;

impl Default for FrequentOpsTable {
    fn default() -> Self {
        Self::new()
    }
}
impl FrequentOpsTable {
    pub fn new() -> Self {
        Self { table_by_op: [FREQUENT_OP_EMPTY; 256], table_ops: Vec::new() }
    }
    fn add_ops(&mut self, op: u8, ops: &mut Vec<[u64; 2]>, move_contents: bool) {
        let mut index = self.table_by_op[op as usize];
        if index == FREQUENT_OP_EMPTY {
            index = self.table_ops.len();
            self.table_ops.push(Vec::new());
            self.table_by_op[op as usize] = index;
        }
        if move_contents {
            self.table_ops[index].append(ops);
        } else {
            self.table_ops[index].extend(ops.iter().cloned());
        }
    }

    fn build_low_values_operations(&mut self) {
        let mut ops: Vec<[u64; 2]> = Vec::new();
        for i in 0..MAX_A_LOW_VALUE {
            for j in 0..MAX_B_LOW_VALUE {
                ops.push([i, j]);
            }
        }

        for op in LOW_VALUES_OPCODES.iter() {
            self.add_ops(*op, &mut ops, false);
        }
    }

    fn build_eq_zero(&mut self) {
        let mut ops: Vec<[u64; 2]> = Vec::new();
        for i in 0..=EQ_OP_B_ZERO_A_LIMIT {
            ops.push([i, 0]);
        }
        self.add_ops(OP_EQ, &mut ops, true);
    }
    #[inline(always)]
    fn get_eq_offset(a: u64, b: u64) -> Option<usize> {
        if b == 0 && a <= EQ_OP_B_ZERO_A_LIMIT {
            Some(LOW_VALUE_SIZE + a as usize)
        } else if b < MAX_B_LOW_VALUE && a < MAX_A_LOW_VALUE {
            Some(Self::get_low_values_offset(a, b))
        } else {
            None
        }
    }

    fn build_ltu_one(&mut self) {
        let mut ops: Vec<[u64; 2]> = Vec::new();
        for i in LTU_OP_B_LT_ONE_FROM..=MAX_U64 {
            ops.push([i, 1]);
        }
        self.add_ops(OP_LTU, &mut ops, true);
    }

    #[inline(always)]
    fn get_ltu_offset(a: u64, b: u64) -> Option<usize> {
        if b == 1 {
            if a >= LTU_OP_B_LT_ONE_FROM {
                Some(LOW_VALUE_SIZE + (a - LTU_OP_B_LT_ONE_FROM) as usize)
            } else if a < MAX_A_LOW_VALUE {
                Some(Self::get_low_values_offset(a, 1))
            } else {
                None
            }
        } else if b < MAX_B_LOW_VALUE && a < MAX_A_LOW_VALUE {
            Some(Self::get_low_values_offset(a, b))
        } else {
            None
        }
    }

    fn build_lt(&mut self) {
        let mut ops: Vec<[u64; 2]> = Vec::new();
        let mut i = LT_FROM_ADDR;
        while i < LT_TO_ADDR {
            for j in 0..LT_LOW_DISTANCE_1 {
                ops.push([i - j, i]);
            }
            for j in 0..LT_HIGH_DISTANCE_8 {
                ops.push([i - j * 8 + 16, i]);
            }
            i += LT_DELTA;
        }
        for i in MAX_B_LOW_VALUE..LT_ZERO_TO_B {
            ops.push([0, i]);
        }
        self.add_ops(OP_LT, &mut ops, true);
    }
    #[inline(always)]
    fn is_frequent_lt(a: u64, b: u64) -> bool {
        if a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE {
            true
        } else if a & 0xFFFF_FFFF_FFFE_0007 == 0xA010_0000 {
            // 256 / 8 = 32 (5 bits)
            let dist = b - a;
            if dist < LT_LOW_DISTANCE_1 {
                true
            } else if dist <= LT_MAX_DISTANCE && dist & 0x7 == 0 {
                // 16 - dist >> 3 - 2 = 14 - dist >> 3
                true
            } else {
                false
            }
        } else {
            a == 0 && b < 8192
        }
    }

    #[inline(always)]
    fn get_lt_offset(a: u64, b: u64) -> Option<usize> {
        const_assert!(LT_DELTA == 8);
        const_assert!(LT_FROM_ADDR == 0xA010_0000);
        const_assert!(LT_TO_ADDR == 0xA012_0000);

        if a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE {
            Some(Self::get_low_values_offset(a, b))
        // TODO_ASSERT
        } else if a & 0xFFFF_FFFF_FFFE_0007 == 0xA010_0000 {
            // 256 / 8 = 32 (5 bits)
            let addr_offset = ((a - LT_FROM_ADDR) >> 3) * LT_LOW_HIGH_DISTANCES;
            let dist = b - a;
            if dist < LT_LOW_DISTANCE_1 {
                Some(LOW_VALUE_SIZE)
            } else if dist <= LT_MAX_DISTANCE && dist & 0x7 == 0 {
                // 16 - dist >> 3 - 2 = 14 - dist >> 3
                Some(LOW_VALUE_SIZE + ((addr_offset + LT_LOW_DISTANCE_1 - 2 + dist) >> 3) as usize)
            } else {
                None
            }
        } else if a == 0 && b < 8192 {
            // in this point B >= MAX_B_LOW_VALUE
            Some(LOW_VALUE_SIZE + LT_ALL_FROM_TO_SIZE + (b - MAX_B_LOW_VALUE) as usize)
        } else {
            None
        }
    }

    fn build_add(&mut self) {
        let mut ops: Vec<[u64; 2]> = Vec::new();
        for i in 0..MAX_ADD_MINUS_ONE {
            ops.push([i, MINUS_ONE]);
        }
        assert_eq!(ADD_MINUS_ONE_SIZE, ops.len());

        assert_eq!(ADD_MINUS_A_B_OFFSET, LOW_VALUE_SIZE + ops.len());
        for i in 0..MAX_ADD_MINUS_A {
            for j in 1..=MAX_ADD_MINUS_B {
                ops.push([i, MAX_U64 - j]);
            }
        }
        assert_eq!((ADD_MINUS_A_B_OFFSET + ADD_MINUS_A_B_SIZE), LOW_VALUE_SIZE + ops.len());
        // address + 1

        assert_eq!(ADD_ONE_ADDR_OFFSET, LOW_VALUE_SIZE + ops.len());
        for i in ADD_ONE_FROM_ADDR..ADD_ONE_TO_ADDR {
            ops.push([i, 1]);
        }
        assert_eq!((ADD_ONE_ADDR_OFFSET + ADD_ONE_ADDR_SIZE), LOW_VALUE_SIZE + ops.len());

        assert_eq!(ADD_EIGHT_ADDR_OFFSET, LOW_VALUE_SIZE + ops.len());
        for i in (ADD_EIGHT_FROM_ADDR..ADD_EIGHT_TO_ADDR).step_by(ADD_EIGHT_STEP as usize) {
            ops.push([i, 8]);
        }
        assert_eq!((ADD_EIGHT_ADDR_OFFSET + ADD_EIGHT_ADDR_SIZE), LOW_VALUE_SIZE + ops.len());

        assert_eq!(ADD_EIGHT_CODE_OFFSET, LOW_VALUE_SIZE + ops.len());
        for i in (ADD_EIGHT_FROM_CODE..ADD_EIGHT_TO_CODE).step_by(ADD_EIGHT_STEP as usize) {
            ops.push([i, 8]);
        }
        assert_eq!((ADD_EIGHT_CODE_OFFSET + ADD_EIGHT_CODE_SIZE), LOW_VALUE_SIZE + ops.len());

        assert_eq!(ADD_ZERO_ADDR_OFFSET, LOW_VALUE_SIZE + ops.len());
        for i in (ADD_ZERO_FROM_ADDR..ADD_ZERO_TO_ADDR).step_by(ADD_ZERO_STEP as usize) {
            ops.push([i, 0]);
        }
        assert_eq!((ADD_ZERO_ADDR_OFFSET + ADD_ZERO_ADDR_SIZE), LOW_VALUE_SIZE + ops.len());

        self.add_ops(OP_ADD, &mut ops, true);
    }

    #[inline(always)]
    fn get_add_offset(a: u64, b: u64) -> Option<usize> {
        const_assert!(ADD_ZERO_STEP == 8);
        if b < MAX_B_LOW_VALUE {
            if a < MAX_A_LOW_VALUE {
                Some(Self::get_low_values_offset(a, b))
            } else {
                match b {
                    0 => {
                        if (ADD_ZERO_FROM_ADDR..ADD_ZERO_TO_ADDR).contains(&a) && a & 0x7 == 0 {
                            Some(ADD_ZERO_ADDR_OFFSET + ((a - ADD_ZERO_FROM_ADDR) >> 3) as usize)
                        } else {
                            None
                        }
                    }
                    1 => {
                        if (ADD_ONE_FROM_ADDR..ADD_ONE_TO_ADDR).contains(&a) {
                            Some(ADD_ONE_ADDR_OFFSET + (a - ADD_ONE_FROM_ADDR) as usize)
                        } else {
                            None
                        }
                    }
                    8 => {
                        if a & 0x7 == 0 {
                            if (ADD_EIGHT_FROM_ADDR..ADD_EIGHT_TO_ADDR).contains(&a) {
                                Some(
                                    ADD_EIGHT_ADDR_OFFSET
                                        + ((a - ADD_EIGHT_FROM_ADDR) >> 3) as usize,
                                )
                            } else if (ADD_EIGHT_FROM_CODE..ADD_EIGHT_TO_CODE).contains(&a) {
                                Some(
                                    ADD_EIGHT_CODE_OFFSET
                                        + ((a - ADD_EIGHT_FROM_CODE) >> 3) as usize,
                                )
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }
        } else if b == MINUS_ONE {
            if a < MAX_ADD_MINUS_ONE {
                Some(ADD_MINUS_ONE_OFFSET + a as usize)
            } else {
                None
            }
        } else if b >= ADD_MINUS_A_B_FROM_B && a < MAX_ADD_MINUS_A {
            Some(ADD_MINUS_A_B_OFFSET + (a * MAX_ADD_MINUS_B + (MAX_U64 - 1 - b)) as usize)
        } else {
            None
        }
    }

    fn build_and(&mut self) {
        let mut ops: Vec<[u64; 2]> = Vec::new();
        assert_eq!(AND_CODE_ADDR_OFFSET, LOW_VALUE_SIZE + ops.len());
        for i in (AND_CODE_ADDR_FROM..AND_CODE_ADDR_TO).step_by(AND_CODE_ADDR_STEP as usize) {
            ops.push([AND_CODE_ADDR_MASK, i]);
        }
        assert_eq!(AND_CODE_ADDR_OFFSET + AND_CODE_ADDR_SIZE, LOW_VALUE_SIZE + ops.len());

        assert_eq!(AND_RESET_LAST_THREE_BITS_OFFSET, LOW_VALUE_SIZE + ops.len());
        for i in 0..AND_RESET_LAST_THREE_BITS_A_TO {
            ops.push([i, AND_RESET_LAST_THREE_BITS_B]);
        }
        assert_eq!(
            (AND_RESET_LAST_THREE_BITS_OFFSET + AND_RESET_LAST_THREE_BITS_SIZE),
            LOW_VALUE_SIZE + ops.len()
        );

        assert_eq!(AND_GET_LAST_THREE_BITS_OFFSET, LOW_VALUE_SIZE + ops.len());
        for i in (AND_GET_LAST_THREE_BITS_FROM..AND_GET_LAST_THREE_BITS_TO)
            .step_by(AND_GET_LAST_THREE_BITS_STEP as usize)
        {
            ops.push([i, AND_GET_LAST_THREE_BITS_B]);
        }
        assert_eq!(
            (AND_GET_LAST_THREE_BITS_OFFSET + AND_GET_LAST_THREE_BITS_SIZE),
            LOW_VALUE_SIZE + ops.len()
        );
        self.add_ops(OP_AND, &mut ops, true);
    }

    #[inline(always)]
    fn get_and_offset(a: u64, b: u64) -> Option<usize> {
        if a == AND_CODE_ADDR_MASK
            && (b & 0x03) == 0
            && (AND_CODE_ADDR_FROM..AND_CODE_ADDR_TO).contains(&b)
        {
            Some(AND_CODE_ADDR_OFFSET + ((b - AND_CODE_ADDR_FROM) >> 2) as usize)
        } else if b == AND_RESET_LAST_THREE_BITS_B && a < AND_RESET_LAST_THREE_BITS_A_TO {
            Some(AND_RESET_LAST_THREE_BITS_OFFSET + a as usize)
        } else if b == AND_GET_LAST_THREE_BITS_B
            && a >= AND_GET_LAST_THREE_BITS_FROM
            && a <= AND_GET_LAST_THREE_BITS_TO
            && a & 0x7 == 0
        {
            Some(
                AND_GET_LAST_THREE_BITS_OFFSET + ((a - AND_GET_LAST_THREE_BITS_FROM) >> 3) as usize,
            )
        } else if a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE {
            Some(Self::get_low_values_offset(a, b))
        } else {
            None
        }
    }

    fn build_or(&mut self) {
        let mut ops: Vec<[u64; 2]> = Vec::new();
        for i in MAX_A_LOW_VALUE..OR_TO_A {
            for j in 0..=OR_TO_B {
                ops.push([i, j]);
            }
        }
        self.add_ops(OP_OR, &mut ops, true);
    }
    #[inline(always)]
    fn is_frequent_or(a: u64, b: u64) -> bool {
        (a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE) || (a < OR_TO_A && b <= OR_TO_B)
    }
    #[inline(always)]
    fn get_or_offset(a: u64, b: u64) -> Option<usize> {
        if a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE {
            Some(Self::get_low_values_offset(a, b))
        } else if a < OR_TO_A && b <= OR_TO_B {
            Some(LOW_VALUE_SIZE + ((a - MAX_A_LOW_VALUE) * (OR_TO_B + 1) + b) as usize)
        } else {
            None
        }
    }

    fn build_srl(&mut self) {
        let mut ops: Vec<[u64; 2]> = Vec::new();
        for i in SLR_MASK_FROM..=MAX_U64 {
            for j in 0..=SLR_TO_B {
                ops.push([i, j]);
            }
        }
        self.add_ops(OP_SRL, &mut ops, true);
    }
    #[inline(always)]
    fn is_frequent_srl(a: u64, b: u64) -> bool {
        (a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE) || (a >= SLR_MASK_FROM && b <= SLR_TO_B)
    }
    #[inline(always)]
    fn get_srl_offset(a: u64, b: u64) -> Option<usize> {
        if a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE {
            Some(Self::get_low_values_offset(a, b))
        } else if a >= SLR_MASK_FROM && b <= SLR_TO_B {
            Some(LOW_VALUE_SIZE + ((a - SLR_MASK_FROM) * (SLR_TO_B + 1) + b) as usize)
        } else {
            None
        }
    }
    fn build_sub_w(&mut self) {
        let mut ops: Vec<[u64; 2]> = Vec::new();
        for i in (SUB_W_ADDR_FROM..SUB_W_ADDR_TO).step_by(SUB_W_ADDR_STEP as usize) {
            ops.push([0, i]);
        }
        self.add_ops(OP_SUBW, &mut ops, true);
    }
    #[inline(always)]
    fn is_frequent_sub_w(a: u64, b: u64) -> bool {
        (a == 0 && ((b & 0xFFFF_FFFF_FFFE_0003 == 0xA010_0000) || b < MAX_B_LOW_VALUE))
            || (a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE)
    }
    #[inline(always)]
    fn get_sub_w_offset(a: u64, b: u64) -> Option<usize> {
        if a == 0 {
            if b & 0xFFFF_FFFF_FFFE_0003 == 0xA010_0000 {
                Some(LOW_VALUE_SIZE + ((b - SUB_W_ADDR_FROM) >> 2) as usize)
            } else if b < MAX_B_LOW_VALUE {
                Some(Self::get_low_values_offset(0, b))
            } else {
                None
            }
        } else if a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE {
            Some(Self::get_low_values_offset(a, b))
        } else {
            None
        }
    }
    fn build_xor(&mut self) {
        let mut ops: Vec<[u64; 2]> = Vec::new();
        ops.push([0, MAX_U64]);
        ops.push([1, MAX_U64]);
        self.add_ops(OP_XOR, &mut ops, true);
    }
    #[inline(always)]
    fn is_frequent_xor(a: u64, b: u64) -> bool {
        (b == MAX_U64 && a < 2) || (a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE)
    }
    #[inline(always)]
    fn get_xor_offset(a: u64, b: u64) -> Option<usize> {
        if b == MAX_U64 {
            if a < 2 {
                Some(LOW_VALUE_SIZE + a as usize)
            } else {
                None
            }
        } else if a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE {
            Some(Self::get_low_values_offset(a, b))
        } else {
            None
        }
    }
    fn build_sub(&mut self) {
        let mut ops: Vec<[u64; 2]> = Vec::new();
        for i in MAX_A_LOW_VALUE..SUB_TO_A {
            for j in 0..=SUB_TO_B {
                ops.push([i, j]);
            }
        }
        self.add_ops(OP_SUB, &mut ops, true);
    }
    #[inline(always)]
    fn is_frequent_sub(a: u64, b: u64) -> bool {
        (a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE) || (a < SUB_TO_A && b <= SUB_TO_B)
    }
    #[inline(always)]
    fn get_sub_offset(a: u64, b: u64) -> Option<usize> {
        if a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE {
            Some(Self::get_low_values_offset(a, b))
        } else if a < SUB_TO_A && b <= SUB_TO_B {
            Some(LOW_VALUE_SIZE + ((a - MAX_A_LOW_VALUE) * (SUB_TO_B + 1) + b) as usize)
        } else {
            None
        }
    }
    pub fn build_table(&mut self) {
        self.build_low_values_operations();
        self.build_eq_zero();
        self.build_ltu_one();
        self.build_lt();
        self.build_add();
        self.build_and();
        self.build_or();
        self.build_srl();
        self.build_sub_w();
        self.build_xor();
        self.build_sub();
    }
    #[inline(always)]
    fn get_low_values_offset(a: u64, b: u64) -> usize {
        (a * MAX_B_LOW_VALUE + b) as usize
    }

    #[inline(always)]
    pub fn is_frequent_op(op: u8, a: u64, b: u64) -> bool {
        // Use lookup table for faster branching instead of match on enum
        match op {
            // Low value operations - check bounds first (most common case)
            OP_SIGNEXTENDB | OP_SIGNEXTENDH | OP_SIGNEXTENDW | OP_ADDW | OP_SLL | OP_SLLW
            | OP_SRA | OP_SRAW | OP_SRLW | OP_EQW | OP_LTUW | OP_LTW | OP_LEU | OP_LE | OP_LEUW
            | OP_LEW | OP_MULU | OP_MULUH | OP_MULSUH | OP_MUL | OP_MULH | OP_MULW | OP_DIVU
            | OP_REMU | OP_DIV | OP_REM | OP_DIVUW | OP_REMUW | OP_DIVW | OP_REMW => {
                a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE
            }
            // Special cases - inline the logic to avoid function calls
            OP_EQ => {
                (b == 0 && a <= EQ_OP_B_ZERO_A_LIMIT)
                    || (b < MAX_B_LOW_VALUE && a < MAX_A_LOW_VALUE)
            }
            OP_LTU => {
                (b == 1 && !(MAX_A_LOW_VALUE..LTU_OP_B_LT_ONE_FROM).contains(&a))
                    || (b < MAX_B_LOW_VALUE && a < MAX_A_LOW_VALUE)
            }
            OP_ADD => {
                // Inline is_frequent_add logic
                if b < MAX_B_LOW_VALUE {
                    if a < MAX_A_LOW_VALUE {
                        true
                    } else {
                        match b {
                            0 => {
                                (ADD_ZERO_FROM_ADDR..ADD_ZERO_TO_ADDR).contains(&a) && a & 0x7 == 0
                            }
                            1 => (ADD_ONE_FROM_ADDR..ADD_ONE_TO_ADDR).contains(&a),
                            8 => {
                                a & 0x7 == 0
                                    && ((ADD_EIGHT_FROM_ADDR..ADD_EIGHT_TO_ADDR).contains(&a)
                                        || (ADD_EIGHT_FROM_CODE..ADD_EIGHT_TO_CODE).contains(&a))
                            }
                            _ => false,
                        }
                    }
                } else if b == MINUS_ONE {
                    a < MAX_ADD_MINUS_ONE
                } else {
                    b >= ADD_MINUS_A_B_FROM_B && a < MAX_ADD_MINUS_A
                }
            }
            OP_AND => {
                // Inline is_frequent_and logic
                (a == AND_CODE_ADDR_MASK
                    && (b & 0x03) == 0
                    && (AND_CODE_ADDR_FROM..AND_CODE_ADDR_TO).contains(&b))
                    || (b == AND_RESET_LAST_THREE_BITS_B && a < AND_RESET_LAST_THREE_BITS_A_TO)
                    || (b == AND_GET_LAST_THREE_BITS_B
                        && a >= AND_GET_LAST_THREE_BITS_FROM
                        && a <= AND_GET_LAST_THREE_BITS_TO
                        && a & 0x7 == 0)
                    || (a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE)
            }
            // Other special cases - call functions for less common operations
            OP_LT => Self::is_frequent_lt(a, b),
            OP_SUBW => Self::is_frequent_sub_w(a, b),
            OP_SUB => Self::is_frequent_sub(a, b),
            OP_OR => Self::is_frequent_or(a, b),
            OP_SRL => Self::is_frequent_srl(a, b),
            OP_XOR => Self::is_frequent_xor(a, b),
            _ => false,
        }
    }

    #[inline(always)]
    pub fn get_row(op: u8, a: u64, b: u64) -> Option<usize> {
        // ecall/system call functions are not candidates to be usual
        let relative_offset = match op {
            OP_SIGNEXTENDB | OP_SIGNEXTENDH | OP_SIGNEXTENDW | OP_ADDW | OP_SLL | OP_SLLW
            | OP_SRA | OP_SRAW | OP_SRLW | OP_EQW | OP_LTUW | OP_LTW | OP_LEU | OP_LE | OP_LEUW
            | OP_LEW | OP_MULU | OP_MULUH | OP_MULSUH | OP_MUL | OP_MULH | OP_MULW | OP_DIVU
            | OP_REMU | OP_DIV | OP_REM | OP_DIVUW | OP_REMUW | OP_DIVW | OP_REMW => {
                if a < MAX_A_LOW_VALUE && b < MAX_B_LOW_VALUE {
                    Some(Self::get_low_values_offset(a, b))
                } else {
                    None
                }
            }
            OP_EQ => Self::get_eq_offset(a, b),
            OP_LTU => Self::get_ltu_offset(a, b),
            OP_LT => Self::get_lt_offset(a, b),
            OP_SUBW => Self::get_sub_w_offset(a, b),
            OP_SUB => Self::get_sub_offset(a, b),
            OP_OR => Self::get_or_offset(a, b),
            OP_SRL => Self::get_srl_offset(a, b),
            OP_XOR => Self::get_xor_offset(a, b),
            OP_AND => Self::get_and_offset(a, b),
            OP_ADD => Self::get_add_offset(a, b),
            _ => None,
        };
        relative_offset.map(|offset| OP_TABLE_OFFSETS[op as usize] + offset)
    }
    pub fn count(&self) -> usize {
        self.table_ops.iter().map(|ops| ops.len()).sum()
    }
    pub fn print_table_offsets(&self) {
        let offsets = self.generate_table_offsets();
        println!("const OP_TABLE_OFFSETS: [usize; {}] = {:?};", offsets.len(), &offsets);
    }

    pub fn generate_table_offsets(&self) -> Vec<usize> {
        let op_indexes = self.get_op_indexes();
        let mut offsets: [usize; 256] = [0; 256];
        let mut size: usize = 0;
        for (op, index) in op_indexes.iter() {
            offsets[*op as usize] = size;
            size += self.table_ops[*index].len();
        }
        let mut last_non_zero: usize = 255;
        while (offsets[last_non_zero] == 0) && (last_non_zero > 0) {
            last_non_zero -= 1;
        }

        assert_eq!(size, 1 << 24);
        offsets[..last_non_zero + 1].to_vec()
    }
    pub fn test_table_offsets(&self) {
        let offsets = self.generate_table_offsets();
        assert_eq!(offsets, OP_TABLE_OFFSETS);
    }
    pub fn generate_full_table(&self) -> Vec<(u8, u64, u64, u64, bool)> {
        let op_indexes = self.get_op_indexes();
        let mut table: Vec<(u8, u64, u64, u64, bool)> = Vec::new();
        for (op, index) in op_indexes.iter() {
            table.extend(
                self.table_ops[*index]
                    .iter()
                    .map(|ab| {
                        let (c, flag) = ZiskOp::try_from_code(*op).unwrap().call_ab(ab[0], ab[1]);
                        (*op, ab[0], ab[1], c, flag)
                    })
                    .clone(),
            );
        }
        table
    }
    pub fn generate_table(&self) -> Vec<(u8, u64, u64)> {
        let op_indexes = self.get_op_indexes();
        let mut table: Vec<(u8, u64, u64)> = Vec::new();
        for (op, index) in op_indexes.iter() {
            table.extend(self.table_ops[*index].iter().map(|ab| (*op, ab[0], ab[1])).clone());
        }
        table
    }
    pub fn get_op_indexes(&self) -> Vec<(u8, usize)> {
        self.table_by_op
            .iter()
            .enumerate()
            .filter(|(_, index)| *index != &FREQUENT_OP_EMPTY)
            .map(|(op, index)| (op as u8, *index))
            .collect()
    }
    pub fn get_list(&self) -> Vec<(u8, usize)> {
        self.table_by_op
            .iter()
            .enumerate()
            .filter(|(_, index)| *index != &FREQUENT_OP_EMPTY)
            .map(|(op, index)| (op as u8, self.table_ops[*index].len()))
            .collect()
    }
    pub fn get_top(&self, num: usize) -> Vec<(u8, usize)> {
        let mut list = self.get_list();
        list.sort_by(|a, b| b.1.cmp(&a.1));
        list.truncate(num);
        list
    }
    pub fn get_top10(&self) -> Vec<(u8, usize)> {
        self.get_top(10)
    }
}

#[test]
fn test_frequent_ops() {
    let mut fops = FrequentOpsTable::new();
    fops.build_table();
    let table = fops.generate_full_table();

    let tests = [
        (ZiskOp::Add, 100, 100, true),
        (ZiskOp::Add, 100, -1i64 as u64, true),
        (ZiskOp::Add, 100000, 100000, false),
        (ZiskOp::Add, 100, -200000i64 as u64, false),
        (ZiskOp::Add, 100, -2i64 as u64, true),
        (ZiskOp::And, 0xFFFF_FFFF_FFFF_FFFC, 0x8000_1000, true),
        (ZiskOp::And, 0xFFFF_FFFF_FFFF_FFFC, 0xA010_1000, false),
    ];
    check_tests(&table, &tests);
}

#[test]
fn test_low_values() {
    let mut fops = FrequentOpsTable::new();
    fops.build_table();
    let table = fops.generate_full_table();
    let mut tests: Vec<(ZiskOp, u64, u64, bool)> = Vec::new();

    for op_index in 0..256 {
        if let Ok(op) = ZiskOp::try_from_code(op_index as u8) {
            let _flag = LOW_VALUES_OPCODES.contains(&op);
            tests.push((op, 0, MAX_B_LOW_VALUE - 1, true));
            tests.push((op, MAX_A_LOW_VALUE, 0, true));
            tests.push((op, MAX_A_LOW_VALUE - 1, 0, true));
            tests.push((op, MAX_A_LOW_VALUE, 0, true));
            tests.push((op, MAX_A_LOW_VALUE * 10, MAX_B_LOW_VALUE * 10, false));
            tests.push((op, MAX_A_LOW_VALUE, 200, false));
            tests.push((op, MAX_A_LOW_VALUE * 10, MAX_B_LOW_VALUE * 10, false));
            tests.push((op, MAX_A_LOW_VALUE, 200, false));
            tests.push((op, MAX_A_LOW_VALUE * 10, MAX_B_LOW_VALUE * 10, false));
            tests.push((op, MAX_A_LOW_VALUE, 200, false));
            tests.push((op, MAX_A_LOW_VALUE * 10, MAX_B_LOW_VALUE * 10, false));
            tests.push((op, MAX_A_LOW_VALUE, 200, false));
            tests.push((op, 200, MAX_B_LOW_VALUE, false));
            tests.push((op, MAX_A_LOW_VALUE, MAX_B_LOW_VALUE, false));
        }
    }
    check_tests(&table, &tests);
}

#[cfg(test)]
fn check_tests(table: &Vec<(u8, u64, u64, u64, bool)>, tests: &[(ZiskOp, u64, u64, bool)]) {
    for (itest, test) in tests.iter().enumerate() {
        println!(
            "> #{} {1} 0x{2:X}({2}) 0x{3:X}({3}) {4}",
            itest,
            test.0.name(),
            test.1,
            test.2,
            test.3
        );
        if let Some(index) = FrequentOpsTable::get_row(test.0.code(), test.1, test.2) {
            // let index = offsets.iter().find(|(op, _)| *op == test.0).unwrap().1 + index;
            println!(
                "= {} 0x{1:X}({1}) 0x{2:X}({2}) = 0x{3:X}({3}) F:{4} [{5}]",
                ZiskOp::try_from_code(table[index].0).unwrap().name(),
                table[index].1,
                table[index].2,
                table[index].3,
                table[index].4 as u8,
                if test.3 == true
                    && table[index].0 == test.0.code()
                    && table[index].1 == test.1
                    && table[index].2 == test.2
                {
                    "\x1B[32mOK\x1B[0m"
                } else {
                    "\x1B[31mFAIL\x1B[0m"
                }
            );
            assert_eq!(true, test.3);
            assert_eq!(table[index].0, test.0.code());
            assert_eq!(table[index].1, test.1);
            assert_eq!(table[index].2, test.2);
        } else {
            println!(
                "= Not Found [{}]",
                if test.3 == false { "\x1B[32mOK\x1B[0m" } else { "\x1B[31mFAIL\x1B[0m" }
            );
            assert_eq!(test.3, false);
        }
    }
    println!("Table Size: {}", table.len());
}
