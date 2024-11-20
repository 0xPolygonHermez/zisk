use zisk_core::zisk_ops::*;

use crate::{
    arith_constants::*, arith_table_data, ArithOperation, ArithRangeTableHelpers, ArithTableHelpers,
};

const MIN_N_64: u64 = 0x8000_0000_0000_0000;
const MIN_N_32: u64 = 0x0000_0000_8000_0000;
const MAX_P_64: u64 = 0x7FFF_FFFF_FFFF_FFFF;
const MAX_P_32: u64 = 0x0000_0000_7FFF_FFFF;
const MAX_32: u64 = 0x0000_0000_FFFF_FFFF;
const MAX_64: u64 = 0xFFFF_FFFF_FFFF_FFFF;

const ALL_VALUES: [u64; 16] = [
    0,
    1,
    2,
    3,
    MAX_P_32 - 1,
    MAX_P_32,
    MIN_N_32,
    MAX_32 - 1,
    MAX_32,
    MAX_32 + 1,
    MAX_P_64 - 1,
    MAX_P_64,
    MAX_64 - 1,
    MIN_N_64,
    MIN_N_64 + 1,
    MAX_64,
];

const ALL_OPERATIONS: [u8; 14] =
    [MUL, MULH, MULSUH, MULU, MULUH, DIVU, REMU, DIV, REM, MUL_W, DIVU_W, REMU_W, DIV_W, REM_W];

struct ArithOperationTest {
    count: u32,
    fail: u32,
    fail_by_op: [u32; 16],
    pending: u32,
    table_rows: [u16; arith_table_data::ROWS],
}

impl ArithOperationTest {
    // NOTE: use 0x0000_0000 instead of 0, to avoid auto-format in one line, 0 is too short.
    pub fn new() -> Self {
        ArithOperationTest {
            count: 0,
            fail: 0,
            fail_by_op: [0; 16],
            pending: 0,
            table_rows: [0; arith_table_data::ROWS],
        }
    }
    fn test(&mut self) {
        self.count = 0;

        for op in ALL_OPERATIONS {
            let m32 = Self::is_m32_op(op);
            for a in ALL_VALUES {
                if m32 && a > 0xFFFF_FFFF {
                    continue;
                }
                for b in ALL_VALUES {
                    if m32 && b > 0xFFFF_FFFF {
                        continue;
                    }
                    println!("===> TEST CASE op:0x{:x} with a:0x{:X} b:0x{:X} <===", op, a, b);
                    let (emu_c, emu_flag) = Self::calculate_emulator_res(op, a, b);
                    self.test_operation(op, a, b, emu_c, emu_flag);
                    self.count += 1;
                }
            }
        }
        for index in 0..arith_table_data::ROWS {
            if self.table_rows[index] == 0 {
                println!(
                    "\x1B[31mTable row {0} not tested op:0x{1:x}({1}) flags:{2}\x1B[0m",
                    index,
                    arith_table_data::ARITH_TABLE[index][0],
                    ArithTableHelpers::flags_to_string(arith_table_data::ARITH_TABLE[index][1]),
                );
                self.pending += 1;
            }
        }
        println!("TOTAL TESTS:{} ERRORS: {}", self.count, self.fail);
    }

    fn is_m32_op(op: u8) -> bool {
        match op {
            MUL | MULH | MULSUH | MULU | MULUH | DIVU | REMU | DIV | REM => false,
            MUL_W | DIVU_W | REMU_W | DIV_W | REM_W => true,
            _ => panic!("Invalid opcode"),
        }
    }
    fn calculate_emulator_res(op: u8, a: u64, b: u64) -> (u64, bool) {
        match op {
            MULU => return op_mulu(a, b),
            MULUH => return op_muluh(a, b),
            MULSUH => return op_mulsuh(a, b),
            MUL => return op_mul(a, b),
            MULH => return op_mulh(a, b),
            MUL_W => return op_mul_w(a, b),
            DIVU => return op_divu(a, b),
            REMU => return op_remu(a, b),
            DIVU_W => return op_divu_w(a, b),
            REMU_W => return op_remu_w(a, b),
            DIV => return op_div(a, b),
            REM => return op_rem(a, b),
            DIV_W => return op_div_w(a, b),
            REM_W => return op_rem_w(a, b),
            _ => {
                panic!("Invalid opcode");
            }
        }
    }

    fn test_operation(&mut self, op: u8, a: u64, b: u64, c: u64, flag: bool) {
        let mut aop = ArithOperation::new();
        aop.calculate(op, a, b);
        println!("testing op:0x{:x} a:0x{:X} b:0x{:X} c:0x{:X} flag:{}", op, a, b, c, flag);
        let chunks = aop.calculate_chunks();
        for i in 0..8 {
            let carry_in = if i > 0 { aop.carry[i - 1] } else { 0 };
            let carry_out = if i < 7 { aop.carry[i] } else { 0 };
            let res = chunks[i] + carry_in - 0x10000 * carry_out;
            if res != 0 {
                println!("{:#?}", aop);

                self.fail += 1;
                self.fail_by_op[(op - 0xb0) as usize] += 1;
                println!("\x1B[31mFAIL: 0x{4:X}({4})!= 0 chunks[{0}]=0x{1:X}({1}) carry_in: 0x{2:x},{2} carry_out: 0x{3:x},{3} failed\x1B[0m",
                i,
                chunks[i],
                carry_in,
                carry_out,
                res);
            }
        }
        println!("{:#?}", aop);

        const CHUNK_SIZE: u64 = 0x10000;
        let bus_a_low: u64 = aop.div as u64 * (aop.c[0] + aop.c[1] * CHUNK_SIZE)
            + (1 - aop.div as u64) * (aop.a[0] + aop.a[1] * CHUNK_SIZE);
        let bus_a_high: u64 = aop.div as u64 * (aop.c[2] + aop.c[3] * CHUNK_SIZE)
            + (1 - aop.div as u64) * (aop.a[2] + aop.a[3] * CHUNK_SIZE);

        let bus_b_low: u64 = aop.b[0] + CHUNK_SIZE * aop.b[1];
        let bus_b_high: u64 = aop.b[2] + CHUNK_SIZE * aop.b[3];

        let secondary_res: u64 = if aop.main_mul || aop.main_div { 0 } else { 1 };

        let bus_res_low = secondary_res * (aop.d[0] + aop.d[1] * CHUNK_SIZE)
            + aop.main_mul as u64 * (aop.c[0] + aop.c[1] * CHUNK_SIZE)
            + aop.main_div as u64 * (aop.a[0] + aop.a[1] * CHUNK_SIZE);

        let bus_res_high_64 = secondary_res * (aop.d[2] + aop.d[3] * CHUNK_SIZE)
            + aop.main_mul as u64 * (aop.c[2] + aop.c[3] * CHUNK_SIZE)
            + aop.main_div as u64 * (aop.a[2] + aop.a[3] * CHUNK_SIZE);

        let bus_res_high = if aop.sext && !aop.div_overflow { 0xFFFF_FFFF } else { 0 }
            + (1 - aop.m32 as u64) * bus_res_high_64;

        let expected_a_low = a & 0xFFFF_FFFF;
        let expected_a_high = (a >> 32) & 0xFFFF_FFFF;
        let expected_b_low = b & 0xFFFF_FFFF;
        let expected_b_high = (b >> 32) & 0xFFFF_FFFF;
        let expected_res_low = c & 0xFFFF_FFFF;
        let expected_res_high = (c >> 32) & 0xFFFF_FFFF;

        assert_eq!(
            bus_a_low, expected_a_low,
            "bus_a_low: 0x{0:X}({0}) vs 0x{1:X}({1}) (expected)",
            bus_a_low, expected_a_low
        );
        assert_eq!(
            bus_a_high, expected_a_high,
            "bus_a_high: 0x{0:X}({0}) vs 0x{1:X}({1}) (expected)",
            bus_a_high, expected_a_high
        );
        assert_eq!(
            bus_b_low, expected_b_low,
            "bus_b_low: 0x{0:X}({0}) vs 0x{1:X}({1}) (expected)",
            bus_b_low, expected_b_low
        );
        assert_eq!(
            bus_b_high, expected_b_high,
            "bus_b_high: 0x{0:X}({0}) vs 0x{1:X}({1}) (expected)",
            bus_b_high, expected_b_high
        );
        assert_eq!(
            bus_res_low, expected_res_low,
            "bus_c_low: 0x{0:X}({0}) vs 0x{1:X}({1}) (expected)",
            bus_res_low, expected_res_low
        );
        assert_eq!(
            bus_res_high, expected_res_high,
            "bus_c_high: 0x{0:X}({0}) vs 0x{1:X}({1}) (expected)",
            bus_res_high, expected_res_high
        );
        for i in 0..7 {
            ArithRangeTableHelpers::get_row_carry_range_check(aop.carry[i]);
        }

        ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_ab, aop.a[3]);
        ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_ab + 26, aop.a[1]);
        ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_ab + 17, aop.b[3]);
        ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_ab + 9, aop.b[1]);

        ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_cd, aop.c[3]);
        ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_cd + 26, aop.c[1]);
        ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_cd + 17, aop.d[3]);
        ArithRangeTableHelpers::get_row_chunk_range_check(aop.range_cd + 9, aop.d[1]);

        for i in [0, 2] {
            ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.a[i]);
            ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.b[i]);
            ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.c[i]);
            ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.d[i]);
        }

        let row_1 = ArithTableHelpers::get_row(
            aop.op,
            aop.na,
            aop.nb,
            aop.np,
            aop.nr,
            aop.sext,
            aop.div_by_zero,
            aop.div_overflow,
            aop.m32,
            aop.div,
            aop.main_mul,
            aop.main_div,
            aop.signed,
            aop.range_ab as u16,
            aop.range_cd as u16,
        );
        self.table_rows[row_1] += 1;
    }
}

#[test]
fn test() {
    let mut test = ArithOperationTest::new();
    test.test();
    for i in 0..16 {
        if test.fail_by_op[i] == 0 {
            continue;
        }
        println!("fail_by_op[0x{:X}]: {}", i + 0xb0, test.fail_by_op[i]);
    }
    assert_eq!(test.fail, 0);
}
