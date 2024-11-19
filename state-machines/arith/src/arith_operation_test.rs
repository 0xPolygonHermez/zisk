use zisk_core::zisk_ops::*;

use crate::{
    arith_constants::*, arith_table_data, ArithOperation, ArithRangeTableHelpers, ArithTableHelpers,
};

struct TestParams {
    op: u8,
    a: u64,
    b: u64,
}
const TEST_COUNT: u32 = 3136;

const MIN_N_64: u64 = 0x8000_0000_0000_0000;
const MIN_N_32: u64 = 0x0000_0000_8000_0000;
const MAX_P_64: u64 = 0x7FFF_FFFF_FFFF_FFFF;
const MAX_P_32: u64 = 0x0000_0000_7FFF_FFFF;
const MAX_32: u64 = 0x0000_0000_FFFF_FFFF;
const MAX_64: u64 = 0xFFFF_FFFF_FFFF_FFFF;

// value cannot used as specific cases
const ALL_64: u64 = 0x0033;
const ALL_NZ_64: u64 = 0x0034;
const ALL_P_64: u64 = 0x0035;
const ALL_NZ_P_64: u64 = 0x0036;
const ALL_N_64: u64 = 0x0037;
const ALL_NON_MIN_64: u64 = 0x0038;

const ALL_32: u64 = 0x0043;
const ALL_NZ_32: u64 = 0x0044;
const ALL_P_32: u64 = 0x0045;
const ALL_N_32: u64 = 0x0046;
const ALL_NZ_P_32: u64 = 0x0047;
const ALL_NON_MIN_32: u64 = 0x0048;

const VALUES_END: u64 = 0x004D;

struct ArithOperationTest {
    count: u32,
    ok: u32,
    fail: u32,
    fail_range_check: u32,
    fail_table: u32,
    fail_bus: u32,
    fail_by_op: [u32; 16],
    table_rows: [u16; arith_table_data::ROWS],
}

impl ArithOperationTest {
    // NOTE: use 0x0000_0000 instead of 0, to avoid auto-format in one line, 0 is too short.
    pub fn new() -> Self {
        ArithOperationTest {
            count: 0,
            ok: 0,
            fail: 0,
            fail_range_check: 0,
            fail_table: 0,
            fail_bus: 0,
            fail_by_op: [0; 16],
            table_rows: [0; arith_table_data::ROWS],
        }
    }
    fn test(&mut self) {
        let mut count = 0;
        let mut index: u32 = 0;

        #[derive(Debug, PartialEq)]
        struct TestDone {
            op: u8,
            a: u64,
            b: u64,
            index: u32,
            offset: u32,
        }

        let tests = [
            // 0 - MULU
            TestParams { op: MULU, a: ALL_64, b: ALL_64 },
            // 1 - MULUH
            TestParams { op: MULUH, a: ALL_64, b: ALL_64 },
            // 2 - MULSUH
            TestParams { op: MULSUH, a: ALL_P_64, b: ALL_64 },
            // 3 - MULSUH
            TestParams { op: MULSUH, a: ALL_N_64, b: ALL_NZ_64 },
            // 4 - MULSUH
            TestParams { op: MULSUH, a: ALL_N_64, b: 0x0000_0000 },
            // 5 - MUL
            TestParams { op: MUL, a: ALL_P_64, b: ALL_P_64 },
            // 6 - MUL
            TestParams { op: MUL, a: ALL_N_64, b: ALL_N_64 },
            // 7 - MUL
            TestParams { op: MUL, a: ALL_N_64, b: ALL_NZ_P_64 },
            // 8 - MUL
            TestParams { op: MUL, a: ALL_N_64, b: 0x0000_0000 },
            // 9 - MUL
            TestParams { op: MUL, a: ALL_NZ_P_64, b: ALL_N_64 },
            // 10 - MUL
            TestParams { op: MUL, a: 0x0000_0000, b: ALL_N_64 },
            // 11 - MULH
            TestParams { op: MULH, a: ALL_P_64, b: ALL_P_64 },
            // 12 - MULH
            TestParams { op: MULH, a: ALL_N_64, b: ALL_N_64 },
            // 13 - MULH
            TestParams { op: MULH, a: ALL_N_64, b: ALL_NZ_P_64 },
            // 14 - MULH
            TestParams { op: MULH, a: ALL_N_64, b: 0x0000_00000 },
            // 15 - MULH
            TestParams { op: MULH, a: ALL_NZ_P_64, b: ALL_N_64 },
            // 16 - MULH
            TestParams { op: MULH, a: 0x0000_0000, b: ALL_N_64 },
            // 17 - MUL_W
            TestParams { op: MUL_W, a: 0x0000_0000, b: 0x0000_0000 },
            // 18 - MUL_W: 0x00000002 (+/32 bits) * 0x40000000 (+/32 bits) = 0x80000000 (-/32 bits)
            TestParams { op: MUL_W, a: 0x0000_0002, b: 0x4000_0000 },
            // 19 - MUL_W
            TestParams { op: MUL_W, a: 0x0000_0002, b: 0x8000_0000 },
            // 20 - MUL_W
            TestParams { op: MUL_W, a: 0xFFFF_FFFF, b: 1 },
            // 21 - MUL_W
            TestParams { op: MUL_W, a: 0xFFFF_FFFF, b: 0x0000_00000 },
            // 22 - MUL_W
            TestParams { op: MUL_W, a: 0x7FFF_FFFF, b: 2 },
            // 23 - MUL_W
            TestParams { op: MUL_W, a: 0xBFFF_FFFF, b: 0x0000_0002 },
            // 24 - MUL_W: 0xFFFF_FFFF * 0xFFFF_FFFF = 0xFFFF_FFFE_0000_0001
            TestParams { op: MUL_W, a: 0xFFFF_FFFF, b: 0xFFFF_FFFF },
            // 25 - MUL_W: 0xFFFF_FFFF * 0x0FFF_FFFF = 0x0FFF_FFFE_F000_0001
            TestParams { op: MUL_W, a: 0xFFFF_FFFF, b: 0x0FFF_FFFF },
            // 26 - MUL_W: 0x8000_0000 * 0x8000_0000 = 0x4000_0000_0000_0000
            TestParams { op: MUL_W, a: 0x8000_0000, b: 0x8000_0000 },
            // 27 - DIVU
            TestParams { op: DIVU, a: ALL_64, b: ALL_NZ_64 },
            // 28 - REMU
            TestParams { op: REMU, a: ALL_64, b: ALL_NZ_64 },
            // 29 - DIV
            TestParams { op: DIV, a: MAX_P_64, b: MAX_P_64 },
            // 30 - DIV
            TestParams { op: DIV, a: MIN_N_64, b: MAX_P_64 },
            // 31 - DIV
            TestParams { op: DIV, a: MAX_P_64, b: MIN_N_64 },
            // 32 - DIV
            TestParams { op: DIV, a: MIN_N_64, b: MIN_N_64 },
            // 33 - DIV
            TestParams { op: DIV, a: 0x0000_0000, b: MAX_P_64 },
            // 34 - DIV
            TestParams { op: DIV, a: 0x0000_0000, b: MIN_N_64 },
            // 35 - REM
            TestParams { op: REM, a: MAX_P_64, b: MAX_P_64 },
            // 36 - REM
            TestParams { op: REM, a: MIN_N_64, b: MAX_P_64 },
            // 37 - REM
            TestParams { op: REM, a: MAX_P_64, b: MIN_N_64 },
            // 38 - REM
            TestParams { op: REM, a: MIN_N_64, b: MIN_N_64 },
            // 39 - REM
            TestParams { op: REM, a: 0x0000_0000, b: MAX_P_64 },
            // 40 - REM
            TestParams { op: REM, a: 0x0000_0000, b: MIN_N_64 },
            // 41 - DIVU_W
            TestParams { op: DIVU_W, a: 0xFFFF_FFFF, b: 0x0000_0001 },
            // 42 - DIVU_W
            TestParams { op: DIVU_W, a: ALL_NZ_32, b: 0x0000_00002 },
            // 43 - DIVU_W
            TestParams { op: DIVU_W, a: ALL_NZ_32, b: MAX_32 },
            // 44 - DIVU_W
            TestParams { op: DIVU_W, a: 0, b: ALL_NZ_32 },
            // 45 - REMU_W
            TestParams { op: REMU_W, a: 0xFFFF_FFFF, b: 0x0000_0001 },
            // 46 - REMU_W
            TestParams { op: REMU_W, a: ALL_32, b: 0x0000_00002 },
            // 47 - REMU_W
            TestParams { op: REMU_W, a: ALL_NZ_P_32, b: MAX_32 },
            // 48 - REMU_W
            TestParams { op: REMU_W, a: ALL_32, b: 0x8000_0000 },
            // 49 - REMU_W
            TestParams { op: REMU_W, a: 0, b: ALL_NZ_32 },
            // 50 - REMU_W
            TestParams { op: REMU_W, a: 0xFFFF_FFFE, b: 0xFFFF_FFFF },
            // 51 - REMU_W
            TestParams { op: REMU_W, a: 0xFFFF_FFFE, b: 0xFFFF_FFFE },
            // 52 - REMU_W
            TestParams { op: REMU_W, a: 0x8000_0000, b: 0x8000_0001 },
            // 53 - REMU_W
            TestParams { op: REMU_W, a: 0x8000_0001, b: 0x8000_0000 },
            // 54 - REMU_W
            TestParams { op: REMU_W, a: 0xFFFF_FFFF, b: 0x0000_0003 },
            // 55 - DIV_W (-1/1=-1 REM:0)
            TestParams { op: DIV_W, a: 0xFFFF_FFFF, b: 0x0000_0001 },
            // 56 - REM_W !!!
            TestParams { op: REM_W, a: 0xFFFF_FFFF, b: 0x0000_0001 },
            // 57 - DIV_W <======
            TestParams { op: DIV_W, a: 0xFFFF_FFFF, b: 0x0000_0002 },
            // 58 - REM_W
            TestParams { op: REM_W, a: 0xFFFF_FFFF, b: 0x0000_0002 },
            // 59 - DIV
            TestParams { op: DIV, a: ALL_NON_MIN_64, b: ALL_NZ_64 },
            // 60 - DIV_W
            TestParams { op: DIV_W, a: ALL_NON_MIN_32, b: ALL_NZ_32 },
            // 61 - REM
            TestParams { op: REM, a: ALL_NON_MIN_64, b: ALL_NZ_64 },
            // 62 - REM_W
            TestParams { op: REM_W, a: ALL_NON_MIN_32, b: ALL_NZ_32 },
            // 63 - DIV_W
            TestParams { op: DIV_W, a: 0x0000_0001, b: 0xFFFF_FFFF },
            // 64 - REM_W
            TestParams { op: REM_W, a: 0, b: 0x8000_0000 },
        ];

        let mut tests_done: Vec<TestDone> = Vec::new();
        let mut errors = 0;
        for test in tests {
            let a_values = Self::get_test_values(test.a);
            let mut offset = 0;
            for _a in a_values {
                if _a == VALUES_END {
                    break;
                }
                let b_values = Self::get_test_values(test.b);
                for _b in b_values {
                    if _b == VALUES_END {
                        break;
                    }
                    let test_info = TestDone { op: test.op, a: _a, b: _b, index, offset };
                    let previous = tests_done.iter().find(|&x| {
                        x.op == test_info.op && x.a == test_info.a && x.b == test_info.b
                    });
                    match previous {
                        Some(e) => {
                            println!(
                            "\x1B[35mDuplicated TEST #{} op:0x{:x} a:0x{:X} b:0x{:X} offset:{}\x1B[0m",
                            e.index, e.op, e.a, e.b, e.offset
                        );
                        }
                        None => {
                            tests_done.push(test_info);
                        }
                    }
                    println!(
                        "testing #{} op:0x{:x} with _a:0x{:X} _b:0x{:X}",
                        index, test.op, _a, _b
                    );
                    let (emu_c, emu_flag) = Self::calculate_emulator_res(test.op, _a, _b);
                    self.test_operation(test.op, _a, _b, emu_c, emu_flag);
                    offset += 1;
                    count += 1;
                }
            }
            index += 1;
        }
        for index in 0..arith_table_data::ROWS {
            if self.table_rows[index] == 0 {
                println!(
                    "\x1B[31mTable row {0} not tested op:0x{1:x}({1}) flags:{2}\x1B[0m",
                    index,
                    arith_table_data::ARITH_TABLE[index][0],
                    ArithTableHelpers::flags_to_string(arith_table_data::ARITH_TABLE[index][1]),
                );
                errors += 1;
            }
        }
        println!("TOTAL ERRORS: {}", self.fail);
        assert_eq!(count, TEST_COUNT, "Number of tests not matching");
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

    fn dump_test(
        &mut self,
        index: u32,
        op: u8,
        a: u64,
        b: u64,
        c: u64,
        flag: bool,
        range_ab: u64,
        range_cd: u64,
        flags: u64,
        aop: &ArithOperation,
    ) {
        println!("{:#?}", aop);
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

        let bus_res_high = aop.sext as u64 * 0xFFFF_FFFF + (1 - aop.m32 as u64) * bus_res_high_64;

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
        // check all chunks and carries
        // let carry_min_value: i64 = -0xEFFFF;
        // let carry_max_value: i64 = 0xF0000;
        // for i in 0..7 {
        //     // assert!(aop.carry[i] >= carry_min_value);
        //     // assert!(aop.carry[i] <= carry_max_value);
        // }
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

        ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.a[0]);
        ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.b[0]);
        ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.c[0]);
        ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.d[0]);

        ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.a[2]);
        ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.b[2]);
        ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.c[2]);
        ArithRangeTableHelpers::get_row_chunk_range_check(0, aop.d[2]);

        let _flags = aop.m32 as u32
            + 2 * aop.div as u32
            + 4 * aop.na as u32
            + 8 * aop.nb as u32
            + 16 * aop.np as u32
            + 32 * aop.nr as u32
            + 64 * aop.sext as u32
            + 128 * aop.main_mul as u32
            + 256 * aop.main_div as u32
            + 512 * aop.signed as u32;
        println!("TABLE {} {} {} {}", aop.op, _flags, aop.range_ab, aop.range_cd);

        let row_1 = ArithTableHelpers::get_row(
            aop.op,
            aop.na,
            aop.nb,
            aop.np,
            aop.nr,
            aop.sext,
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
    fn print_chunks(label: &str, chunks: [u64; 4]) {
        println!(
            "{0}: 0x{1:>04X} \x1B[32m{1:>5}\x1B[0m|0x{2:>04X} \x1B[32m{2:>5}\x1B[0m|0x{3::>04X} \x1B[32m{3:>5}\x1B[0m|0x{4:>04X} \x1B[32m{4:>5}\x1B[0m|",
            label, chunks[0], chunks[1], chunks[2], chunks[3]
        );
    }
    fn check_range(range_id: u64, value: u64) {
        assert!(
            range_id != 0 || (value >= 0 && value <= 0xFFFF),
            "0x{0:x}({0}) in [0x0000, 0xFFFF]",
            value
        );
        assert!(
            range_id != 1 || (value >= 0 && value <= 0x7FFF),
            "0x{0:x}({0}) in [0x0000, 0x7FFF]",
            value
        );
        assert!(
            range_id != 2 || (value >= 0x8000 && value <= 0xFFFF),
            "0x{0:x}({0}) in [0x8000, 0xFFFF]",
            value
        );
    }

    fn get_test_values(value: u64) -> [u64; 16] {
        match value {
            ALL_64 => [
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
            ],
            ALL_NON_MIN_64 => [
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
                MIN_N_64 + 1,
                MIN_N_64 + 2,
                MAX_64,
            ],
            ALL_NZ_64 => [
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
                VALUES_END,
            ],
            ALL_P_64 => [
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
                VALUES_END,
                0,
                0,
                0,
            ],
            ALL_NZ_P_64 => [
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
                VALUES_END,
                0,
                0,
                0,
                0,
            ],
            ALL_N_64 => [
                MIN_N_64,
                MIN_N_64 + 1,
                MIN_N_64 + 2,
                MIN_N_64 + 3,
                0x8000_0000_7FFF_FFFF,
                0x8FFF_FFFF_7FFF_FFFF,
                0xEFFF_FFFF_FFFF_FFFF,
                MAX_64 - 3,
                MAX_64 - 2,
                MAX_64 - 1,
                MAX_64,
                VALUES_END,
                0,
                0,
                0,
                0,
            ],
            ALL_32 => [
                0,
                1,
                2,
                3,
                MAX_P_32 - 1,
                MAX_P_32,
                MIN_N_32,
                MAX_32 - 1,
                MAX_32,
                VALUES_END,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            ALL_NON_MIN_32 => [
                0,
                1,
                2,
                3,
                MAX_P_32 - 1,
                MAX_P_32,
                MIN_N_32 + 1,
                MAX_32 - 1,
                MAX_32,
                VALUES_END,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            ALL_NZ_32 => [
                1,
                2,
                3,
                MAX_P_32 - 1,
                MAX_P_32,
                MIN_N_32,
                MAX_32 - 1,
                MAX_32,
                VALUES_END,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            ALL_P_32 => [
                0,
                1,
                2,
                3,
                0x0000_7FFF,
                0x0000_FFFF,
                MAX_P_32 - 1,
                MAX_P_32,
                MAX_P_32 - 1,
                MAX_P_32,
                VALUES_END,
                0,
                0,
                0,
                0,
                0,
            ],
            ALL_NZ_P_32 => [
                1,
                2,
                3,
                0x0000_7FFF,
                0x0000_FFFF,
                MAX_P_32 - 1,
                MAX_P_32,
                MAX_P_32 - 1,
                MAX_P_32,
                VALUES_END,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            ALL_N_32 => [
                MIN_N_32,
                MIN_N_32 + 1,
                MIN_N_32 + 2,
                MIN_N_32 + 3,
                MAX_32 - 3,
                MAX_32 - 2,
                MAX_32 - 1,
                MAX_32,
                VALUES_END,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ],
            _ => [value, VALUES_END, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        }
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
