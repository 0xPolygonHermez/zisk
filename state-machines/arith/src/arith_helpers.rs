use zisk_core::zisk_ops::*;

const MULU: u8 = 0xb0;
const MULUH: u8 = 0xb1;
const MULSUH: u8 = 0xb3;
const MUL: u8 = 0xb4;
const MULH: u8 = 0xb5;
const MUL_W: u8 = 0xb6;
const DIVU: u8 = 0xb8;
const REMU: u8 = 0xb9;
const DIV: u8 = 0xba;
const REM: u8 = 0xbb;
const DIVU_W: u8 = 0xbc;
const REMU_W: u8 = 0xbd;
const DIV_W: u8 = 0xbe;
const REM_W: u8 = 0xbf;

const FLAG_NAMES: [&str; 8] = ["m32", "div", "na", "nb", "np", "nr", "sext", "sec"];

pub trait ArithHelpers {
    fn get_row(op: u8, na: u64, nb: u64, np: u64, nr: u64, sext: u64) -> i16 {
        static arith_table_rows: [i16; 512] = [
            0, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, 2, 3, -1, -1, -1, 4, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 5, 6, 7, 8, -1,
            9, 10, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, 11, 12, 13, 14, -1, 15, 16, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 17, 18, 19, 20, -1, 21, 22,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, 23, 24, 25, 26, -1, 27, 28, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 29, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, 30, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 31, 32, 33, 34, 35, 36, 37, -1, -1, -1, -1,
            -1, 38, 39, 40, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 41,
            42, 43, 44, 45, 46, 47, -1, -1, -1, -1, -1, 48, 49, 50, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, 51, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, 52, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 53, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 54, -1, -1, -1, -1, -1, -1, -1, -1,
            -1, -1, -1, -1, -1, -1, -1, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, 62, 63, 64,
            -1, 65, 66, 67, 68, 69, 70, 71, -1, -1, -1, -1, -1, 72, 73, 74, -1, 75, 76, 77, 78, 79,
            80, 81, -1, -1, -1, -1, -1, 82, 83, 84, -1, 85, 86, 87, 88, 89, 90, 91, -1, -1, -1, -1,
            -1, 92, 93, 94, -1,
        ];

        let index = (op - 0xb0) as u64 * 32 + na + nb * 2 + np * 4 + nr * 8 + sext * 16;
        arith_table_rows[index as usize]
    }
    //         arith_sm
    fn sign32(abs_value: u64, negative: bool) -> u64 {
        assert!(0xFFFF_FFFF >= abs_value, "abs_value:0x{0:X}({0}) is too big", abs_value);
        if negative {
            (0xFFFF_FFFF - abs_value) + 1
        } else {
            abs_value
        }
    }
    fn sign64(abs_value: u64, negative: bool) -> u64 {
        if negative {
            (0xFFFF_FFFF_FFFF_FFFF - abs_value) + 1
        } else {
            abs_value
        }
    }
    fn sign128(abs_value: u128, negative: bool) -> u128 {
        let res = if negative {
            (0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF - abs_value) + 1
        } else {
            abs_value
        };
        println!("sign128({:X},{})={:X}", abs_value, negative, res);
        res
    }
    fn abs32(value: u64) -> [u64; 2] {
        let negative = if (value & 0x8000_0000) != 0 { 1 } else { 0 };
        let abs_value = if negative == 1 { (0xFFFF_FFFF - value) + 1 } else { value };
        // println!(
        //     "value:0x{0:X}({0}) abs_value:0x{1:X}({1}) negative:{2}",
        //     value, abs_value, negative
        // );
        [abs_value, negative]
    }
    fn abs64(value: u64) -> [u64; 2] {
        let negative = if (value & 0x8000_0000_0000_0000) != 0 { 1 } else { 0 };
        let abs_value = if negative == 1 { (0xFFFF_FFFF_FFFF_FFFF - value) + 1 } else { value };
        [abs_value, negative]
    }
    fn calculate_mul_w(a: u64, b: u64) -> u64 {
        (a & 0xFFFF_FFFF) * (b & 0xFFFF_FFFF)
    }

    fn calculate_mulsu(a: u64, b: u64) -> [u64; 2] {
        let [abs_a, na] = Self::abs64(a);
        let abs_c = abs_a as u128 * b as u128;
        let nc = if na == 1 && abs_c != 0 { 1 } else { 0 };
        let c = Self::sign128(abs_c, nc == 1);
        [c as u64, (c >> 64) as u64]
    }

    fn calculate_mul(a: u64, b: u64) -> [u64; 2] {
        let [abs_a, na] = Self::abs64(a);
        let [abs_b, nb] = Self::abs64(b);
        println!(
            "mul(a:0x{:X}, b:0x{:X} abs_a:0x{:X} na:{} abs_b:0x{:X} nb:{}",
            a, b, abs_a, na, abs_b, nb,
        );
        let abs_c = abs_a as u128 * abs_b as u128;
        let nc = if na != nb && abs_c != 0 { 1 } else { 0 };
        let c = Self::sign128(abs_c, nc == 1);
        [c as u64, (c >> 64) as u64]
    }

    fn calculate_div(a: u64, b: u64) -> u64 {
        let [abs_a, na] = Self::abs64(a);
        let [abs_b, nb] = Self::abs64(b);
        let abs_c = abs_a / abs_b;
        let nc = if na != nb && abs_c != 0 { 1 } else { 0 };
        Self::sign64(abs_c, nc == 1)
    }

    fn calculate_rem(a: u64, b: u64) -> u64 {
        let [abs_a, na] = Self::abs64(a);
        let [abs_b, _nb] = Self::abs64(b);
        let abs_c = abs_a % abs_b;
        let nc = if na == 1 && abs_c != 0 { 1 } else { 0 };
        Self::sign64(abs_c, nc == 1)
    }

    fn calculate_div_w(a: u64, b: u64) -> u64 {
        let [abs_a, na] = Self::abs32(a);
        let [abs_b, nb] = Self::abs32(b);
        let abs_c = abs_a / abs_b;
        let nc = if na != nb && abs_c != 0 { 1 } else { 0 };
        Self::sign32(abs_c, nc == 1)
    }

    fn calculate_rem_w(a: u64, b: u64) -> u64 {
        let [abs_a, na] = Self::abs32(a);
        let [abs_b, _nb] = Self::abs32(b);
        let abs_c = abs_a % abs_b;
        let nc = if na == 1 && abs_c != 0 { 1 } else { 0 };
        Self::sign32(abs_c, nc == 1)
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

    fn calculate_abcd_from_ab(op: u8, a: u64, b: u64) -> [u64; 4] {
        match op {
            MULU | MULUH => {
                let c: u128 = a as u128 * b as u128;
                [a, b, c as u64, (c >> 64) as u64]
            }
            MULSUH => {
                let [c, d] = Self::calculate_mulsu(a, b);
                [a, b, c, d]
            }
            MUL | MULH => {
                let [c, d] = Self::calculate_mul(a, b);
                [a, b, c, d]
            }
            MUL_W => [a, b, Self::calculate_mul_w(a, b), 0],
            DIVU | REMU | DIVU_W | REMU_W => [a / b, b, a, a % b],
            DIV | REM => [Self::calculate_div(a, b), b, a, Self::calculate_rem(a, b)],
            DIV_W | REM_W => [Self::calculate_div_w(a, b), b, a, Self::calculate_rem_w(a, b)],
            _ => {
                panic!("Invalid opcode");
            }
        }
    }
    fn decode_one_range(range_xy: u64) -> [u64; 4] {
        if range_xy == 9 {
            [0, 0, 0, 0]
        } else if range_xy > 9 {
            let x = (range_xy - 8) / 3;
            let y = (range_xy - 8) % 3;
            [0, 0, x, y]
        } else {
            let x = range_xy / 3;
            let y = range_xy % 3;
            [x, y, 0, 0]
        }
    }
    fn decode_ranges(range_ab: u64, range_cd: u64) -> [u64; 8] {
        let ab = Self::decode_one_range(range_ab);
        let cd = Self::decode_one_range(range_cd);
        [ab[0], ab[1], cd[0], cd[1], ab[2], ab[3], cd[2], cd[3]]
    }
    fn calculate_flags_and_ranges(op: u8, a: u64, b: u64, c: u64, d: u64) -> [u64; 11] {
        let mut m32: u64 = 0;
        let mut div: u64 = 0;
        let mut np: u64 = 0;
        let mut nr: u64 = 0;
        let mut sext: u64 = 0;
        let mut secondary_res: u64 = 0;

        let mut range_a1: u64 = 0;
        let mut range_b1: u64 = 0;
        let mut range_c1: u64 = 0;
        let mut range_d1: u64 = 0;
        let mut range_a3: u64 = 0;
        let mut range_b3: u64 = 0;
        let mut range_c3: u64 = 0;
        let mut range_d3: u64 = 0;

        // direct table opcode(14), signed 2 or 4 cases (0,na,nb,na+nb)
        // 6 * 1 + 7 * 4 + 1 * 2 = 36 entries,
        // no compacted => 16 x 4 = 64, key = (op - 0xb0) * 4 + na * 2 + nb
        // output: div, m32, sa, sb, nr, np, na, na32, nd32, range x 2 x 4

        // alternative: switch operation,

        let mut sa = false;
        let mut sb = false;
        let mut rem = false;

        match op {
            MULU => {}
            MULUH => {
                secondary_res = 1;
            }
            MULSUH => {
                sa = true;
                secondary_res = 1;
            }
            MUL => {
                sa = true;
                sb = true;
            }
            MULH => {
                sa = true;
                sb = true;
                secondary_res = 1;
            }
            MUL_W => {
                m32 = 1;
                sext = if ((a * b) & 0xFFFF_FFFF) & 0x8000_0000 != 0 { 1 } else { 0 };
            }
            DIVU => {
                div = 1;
                assert!(b != 0, "Error on DIVU a:{:x}({}) b:{:x}({})", a, b, a, b);
            }
            REMU => {
                div = 1;
                rem = true;
                secondary_res = 1;
            }
            DIV => {
                sa = true;
                sb = true;
                div = 1;
            }
            REM => {
                sa = true;
                sb = true;
                rem = true;
                div = 1;
                secondary_res = 1;
            }
            DIVU_W => {
                // divu_w, remu_w
                div = 1;
                m32 = 1;
                // use a in bus
                sext = if (a & 0x8000_0000) != 0 { 1 } else { 0 };
            }
            REMU_W => {
                // divu_w, remu_w
                div = 1;
                m32 = 1;
                rem = true;
                // use d in bus
                sext = if (d & 0x8000_0000) != 0 { 1 } else { 0 };
                secondary_res = 1;
            }
            DIV_W => {
                // div_w, rem_w
                sa = true;
                sb = true;
                div = 1;
                m32 = 1;
                // use a in bus
                sext = if (a & 0x8000_0000) != 0 { 1 } else { 0 };
            }
            REM_W => {
                // div_w, rem_w
                sa = true;
                sb = true;
                div = 1;
                m32 = 1;
                rem = true;
                // use d in bus
                sext = if (d & 0x8000_0000) != 0 { 1 } else { 0 };
                secondary_res = 1;
            }
            _ => {
                panic!("Invalid opcode");
            }
        }
        let sign_mask: u64 = if m32 == 1 { 0x8000_0000 } else { 0x8000_0000_0000_0000 };
        let sign_c_mask: u64 =
            if m32 == 1 && div == 1 { 0x8000_0000 } else { 0x8000_0000_0000_0000 };
        let na = if sa && (a & sign_mask) != 0 { 1 } else { 0 };
        let nb = if sb && (b & sign_mask) != 0 { 1 } else { 0 };
        // a sign => b sign
        let nc = if sa && (c & sign_c_mask) != 0 { 1 } else { 0 };
        let nd = if sa && (d & sign_mask) != 0 { 1 } else { 0 };

        // a == 0 || b == 0 => np == 0 ==> how was a signed operation
        // after that sign of np was verified with range check.
        // TODO: review if secure
        if div == 1 {
            np = nc; //if c != 0 { na ^ nb } else { 0 };
            nr = nd;
        } else {
            np = if m32 == 1 { nc } else { nd }; // if (c != 0) || (d != 0) { na ^ nb } else { 0 }
            nr = 0;
        }
        if m32 == 1 {
            // mulw, divu_w, remu_w, div_w, rem_w
            range_a1 = if sa {
                1 + na
            } else if div == 1 && !rem {
                1 + sext
            } else {
                0
            };
            range_b1 = if sb { 1 + nb } else { 0 };
            // m32 && div == 0 => mulw
            range_c1 = if div == 0 {
                sext + 1
            } else if sa {
                1 + np
            } else {
                0
            };
            range_d1 = if rem {
                sext + 1
            } else if sa {
                1 + nr
            } else {
                0
            };
        } else {
            // mulu, muluh, mulsuh, mul, mulh, div, rem, divu, remu
            if sa {
                // mulsuh, mul, mulh, div, rem
                range_a3 = 1 + na;
                if div == 1 {
                    // div, rem
                    range_c3 = 1 + np;
                    range_d3 = 1 + nr;
                } else {
                    range_d3 = 1 + np;
                }
            }
            // sb => mul, mulh, div, rem
            range_b3 = if sb { 1 + nb } else { 0 };
        }

        // range_ab / range_cd
        //
        //     a3 a1 b3 b1
        // rid c3 c1 d3 d1 range 2^16 2^15 notes
        // --- -- -- -- -- ----- ---- ---- -------------------------
        //   0  F  F  F  F ab cd    4    0
        //   1  F  F  +  F    cd    3    1 b3 sign => a3 sign
        //   2  F  F  -  F    cd    3    1 b3 sign => a3 sign
        //   3  +  F  F  F ab       3    1 c3 sign => d3 sign
        //   4  +  F  +  F ab cd    2    2
        //   5  +  F  -  F ab cd    2    2
        //   6  -  F  F  F ab       3    1 c3 sign => d3 sign
        //   7  -  F  +  F ab cd    2    2
        //   8  -  F  -  F ab cd    2    2
        //   9  F  F  F  +    cd           a1 sign <=> b1 sign / d1 sign => c1 sign
        //  10  F  F  F  -    cd           a1 sign <=> b1 sign / d1 sign => c1 sign
        //  11  F  +  F  F    cd    3    1 a1 sign <=> b1 sign
        //  12  F  +  F  + ab cd    2    2
        //  13  F  +  F  - ab cd    2    2
        //  14  F  -  F  F    cd    3    1 a1 sign <=> b1 sign
        //  15  F  -  F  + ab cd    2    2
        //  16  F  -  F  - ab cd    2    2

        assert!(range_a1 == 0 || range_a3 == 0, "range_a1:{} range_a3:{}", range_a1, range_a3);
        assert!(range_b1 == 0 || range_b3 == 0, "range_b1:{} range_b3:{}", range_b1, range_b3);
        assert!(range_c1 == 0 || range_c3 == 0, "range_c1:{} range_c3:{}", range_c1, range_c3);
        assert!(range_d1 == 0 || range_d3 == 0, "range_d1:{} range_d3:{}", range_d1, range_d3);

        let range_ab = (range_a3 + range_a1) * 3
            + range_b3
            + range_b1
            + if (range_a1 + range_b1) > 0 { 8 } else { 0 };

        let range_cd = (range_c3 + range_c1) * 3
            + range_d3
            + range_d1
            + if (range_c1 + range_d1) > 0 { 8 } else { 0 };

        let ranges = range_a3 * 1000_0000
            + range_b3 * 100_0000
            + range_c3 * 10_0000
            + range_d3 * 1000
            + range_a1 * 1000
            + range_b1 * 100
            + range_c1 * 10
            + range_d1;
        [m32, div, na, nb, np, nr, sext, secondary_res, range_ab, range_cd, ranges]
    }

    fn calculate_chunks(
        a: [i64; 4],
        b: [i64; 4],
        c: [i64; 4],
        d: [i64; 4],
        m32: i64,
        div: i64,
        na: i64,
        nb: i64,
        np: i64,
        nr: i64,
        fab: i64,
        secondary_res: i64,
        sext: i64,
    ) -> [i64; 8] {
        // TODO: unroll this function in variants (div,m32) and (na,nb,nr,np)
        // div, m32, na, nb === f(div,m32,na,nb) => fa, nb, nr
        // unroll means 16 variants ==> but more performance

        let mut chunks: [i64; 8] = [0, 0, 0, 0, 0, 0, 0, 0];

        let na_fb = na * (1 - 2 * nb);
        let nb_fa = nb * (1 - 2 * na);

        chunks[0] = fab * a[0] * b[0]  // chunk0
            - c[0]
            + 2 * np * c[0]
            + div * d[0]
            - 2 * nr * d[0];

        chunks[1] = fab * a[1] * b[0]    // chunk1
            + fab * a[0] * b[1]
            - c[1]
            + 2 * np * c[1]
            + div * d[1]
            - 2 * nr * d[1];

        chunks[2] = fab * a[2] * b[0]    // chunk2
            + fab * a[1] * b[1]
            + fab * a[0] * b[2]
            + a[0] * nb_fa * m32
            + b[0] * na_fb * m32
            - c[2]
            + 2 * np * c[2]
            + div * d[2]
            - 2 * nr * d[2]
            - np * div * m32
            + nr * m32; // div == 0 ==> nr = 0

        chunks[3] = fab * a[3] * b[0]    // chunk3
            + fab * a[2] * b[1]
            + fab * a[1] * b[2]
            + fab * a[0] * b[3]
            + a[1] * nb_fa * m32
            + b[1] * na_fb * m32
            - c[3]
            + 2 * np * c[3]
            + div * d[3]
            - 2 * nr * d[3];

        chunks[4] = fab * a[3] * b[1]    // chunk4
            + fab * a[2] * b[2]
            + fab * a[1] * b[3]
            + na * nb * m32
            // + b[0] * na * (1 - 2 * nb)
            // + a[0] * nb * (1 - 2 * na)
            + b[0] * na_fb * (1 - m32)
            + a[0] * nb_fa * (1 - m32)
            // high bits ^^^
            // - np * div
            // + np * div * m32
            // - 2 * div * m32 * np
            - np * m32 * (1 - div)  //
            - np * (1 - m32) * div // 2^64 (np)
            + nr * (1 - m32)  // 2^64 (nr)
            // high part d
            - d[0] * (1 - div)           // m32 == 1 and div == 0 => d = 0
            + 2 * np * d[0] * (1 - div); //

        chunks[5] = fab * a[3] * b[2]    // chunk5
            + fab * a[2] * b[3]
            + a[1] * nb_fa * (1 - m32)
            + b[1] * na_fb * (1 - m32)
            - d[1] * (1 - div)
            + d[1] * 2 * np * (1 - div);

        chunks[6] = fab as i64 * a[3] * b[3]    // chunk6
            + a[2] * nb_fa * (1 - m32)
            + b[2] * na_fb * (1 - m32)
            - d[2] * (1 - div)
            + d[2] * 2 * np * (1 - div);

        // 0x4000_0000_0000_0000__8000_0000_0000_0000
        chunks[7] = 0x10000 * na * nb  * (1 - m32)  // chunk7
            + a[3] * nb_fa * (1 - m32)
            + b[3] * na_fb * (1 - m32)
            - 0x10000 * np * (1 - div) * (1 - m32)
            - d[3] * (1 - div)
            + d[3] * 2 * np * (1 - div);

        chunks
    }
    fn u64_to_chunks(a: u64) -> [i64; 4] {
        [
            (a & 0xFFFF) as i64,
            ((a >> 16) & 0xFFFF) as i64,
            ((a >> 32) & 0xFFFF) as i64,
            ((a >> 48) & 0xFFFF) as i64,
        ]
    }
    fn execute_chunks(
        a: u64,
        b: u64,
        c: u64,
        d: u64,
        m32: u64,
        div: u64,
        na: u64,
        nb: u64,
        np: u64,
        nr: u64,
        secondary_res: u64,
        sext: u64,
        range_ab: u64,
        range_cd: u64,
        bus: [u64; 8],
    ) -> bool {
        let fab: i64 = 1 - 2 * na as i64 - 2 * nb as i64 + 4 * na as i64 * nb as i64;
        let a_chunks = Self::u64_to_chunks(a);
        let b_chunks = Self::u64_to_chunks(b);
        let c_chunks = Self::u64_to_chunks(c);
        let d_chunks = Self::u64_to_chunks(d);
        println!(
            "A: 0x{0:>04X} \x1B[32m{0:>5}\x1B[0m|0x{1:>04X} \x1B[32m{1:>5}\x1B[0m|0x{2::>04X} \x1B[32m{2:>5}\x1B[0m|0x{3:>04X} \x1B[32m{3:>5}\x1B[0m|",
            a_chunks[0], a_chunks[1], a_chunks[2], a_chunks[3]
        );
        println!(
            "B: 0x{0:>04X} \x1B[32m{0:>5}\x1B[0m|0x{1:>04X} \x1B[32m{1:>5}\x1B[0m|0x{2::>04X} \x1B[32m{2:>5}\x1B[0m|0x{3:>04X} \x1B[32m{3:>5}\x1B[0m|",
            b_chunks[0], b_chunks[1], b_chunks[2], b_chunks[3]
        );
        println!(
            "C: 0x{0:>04X} \x1B[32m{0:>5}\x1B[0m|0x{1:>04X} \x1B[32m{1:>5}\x1B[0m|0x{2::>04X} \x1B[32m{2:>5}\x1B[0m|0x{3:>04X} \x1B[32m{3:>5}\x1B[0m|",
            c_chunks[0], c_chunks[1], c_chunks[2], c_chunks[3]
        );
        println!(
            "D: 0x{0:>04X} \x1B[32m{0:>5}\x1B[0m|0x{1:>04X} \x1B[32m{1:>5}\x1B[0m|0x{2::>04X} \x1B[32m{2:>5}\x1B[0m|0x{3:>04X} \x1B[32m{3:>5}\x1B[0m|",
            d_chunks[0], d_chunks[1], d_chunks[2], d_chunks[3]
        );

        let mut chunks = Self::calculate_chunks(
            a_chunks,
            b_chunks,
            c_chunks,
            d_chunks,
            m32 as i64,
            div as i64,
            na as i64,
            nb as i64,
            np as i64,
            nr as i64,
            fab,
            secondary_res as i64,
            sext as i64,
        );
        let mut carry: i64 = 0;
        println!(
            "0x{0:X}({0}),0x{1:X}({1}),0x{2:X}({2}),0x{3:X}({3}),0x{4:X}({4}),0x{5:X}({5}),0x{6:X}{6},0x{7:X}({7}) fab:{8:X}",
            chunks[0], chunks[1], chunks[2], chunks[3], chunks[4], chunks[5], chunks[6], chunks[7], fab
        );
        let mut carrys: [i64; 8] = [0, 0, 0, 0, 0, 0, 0, 0];
        for _index in 0..8 {
            println!(
                "APPLY CARRY:{0} CHUNK[{1}]:{2:X} ({2}) {3:X}({3})",
                carry,
                _index,
                chunks[_index],
                chunks[_index] + carry
            );
            let chunk_value = chunks[_index] + carry;
            carry = chunk_value / 0x10000;
            chunks[_index] = chunk_value - carry * 0x10000;
            carrys[_index] = carry;
        }
        println!(
            "CARRY 0x{0:X}({0}),0x{1:X}({1}),0x{2:X}({2}),0x{3:X}({3}),0x{4:X}({4}),0x{5:X}({5}),0x{6:X}{6},0x{7:X}({7}) fab:{8:X}",
            carrys[0], carrys[1], carrys[2], carrys[3], carrys[4], carrys[5], carrys[6], carrys[7], fab
        );
        println!(
            "0x{:X},0x{:X},0x{:X},0x{:X},0x{:X},0x{:X},0x{:X},0x{:X} carry:0x{:X}",
            chunks[0],
            chunks[1],
            chunks[2],
            chunks[3],
            chunks[4],
            chunks[5],
            chunks[6],
            chunks[7],
            carry
        );
        println!(
            "{} {} {} {} {} {} {} {} {}",
            chunks[0],
            chunks[1],
            chunks[2],
            chunks[3],
            chunks[4],
            chunks[5],
            chunks[6],
            chunks[7],
            carry
        );
        let mut passed = if chunks[0] != 0
            || chunks[1] != 0
            || chunks[2] != 0
            || chunks[3] != 0
            || chunks[4] != 0
            || chunks[5] != 0
            || chunks[6] != 0
            || chunks[7] != 0
            || carry != 0
        {
            println!("[\x1B[31mFAIL\x1B[0m]");
            false
        } else {
            println!("[\x1B[32mOK\x1B[0m]");
            true
        };
        const CHUNK_SIZE: i64 = 0x10000;
        let bus_a_low: i64 = div as i64 * (c_chunks[0] + c_chunks[1] * CHUNK_SIZE)
            + (1 - div as i64) * (a_chunks[0] + a_chunks[1] * CHUNK_SIZE);
        let bus_a_high: i64 = div as i64 * (c_chunks[2] + c_chunks[3] * CHUNK_SIZE)
            + (1 - div as i64) * (a_chunks[2] + a_chunks[3] * CHUNK_SIZE);

        let bus_b_low: i64 = b_chunks[0] + CHUNK_SIZE * b_chunks[1];
        let bus_b_high: i64 = b_chunks[2] + CHUNK_SIZE * b_chunks[3];

        let res2_low: i64 = d_chunks[0] + CHUNK_SIZE * d_chunks[1];
        let res2_high: i64 = d_chunks[2] + CHUNK_SIZE * d_chunks[3];

        let res_low: i64 = secondary_res as i64 * res2_low
            + (1 - secondary_res as i64)
                * (a_chunks[0] + c_chunks[0] + CHUNK_SIZE * (a_chunks[1] + c_chunks[1])
                    - bus_a_low);
        println!(
            "RES_LOW: 0x{0:X}({0}) 0x{1:X}({1}) 0x{2:X}({2})",
            res_low,
            a_chunks[2] + c_chunks[2] + CHUNK_SIZE * (a_chunks[3] + c_chunks[3]),
            bus_a_high
        );
        let res_high: i64 = (1 - m32 as i64)
            * (secondary_res as i64 * res2_high
                + (1 - secondary_res as i64)
                    * ((a_chunks[2] + c_chunks[2] + CHUNK_SIZE * (a_chunks[3] + c_chunks[3]))
                        - bus_a_high))
            + sext as i64 * 0xFFFFFFFF;
        passed = passed
            && if bus[1] != bus_a_low as u64
                || bus[2] != bus_a_high as u64
                || bus[3] != bus_b_low as u64
                || bus[4] != bus_b_high as u64
                || bus[5] != res_low as u64
                || bus[6] != res_high as u64
            {
                println!("0x{0:X} ({0}) vs 0x{1:X} ({1})", bus[1], bus_a_low);
                println!("0x{0:X} ({0}) vs 0x{1:X} ({1})", bus[2], bus_a_high);
                println!("0x{0:X} ({0}) vs 0x{1:X} ({1})", bus[3], bus_b_low);
                println!("0x{0:X} ({0}) vs 0x{1:X} ({1})", bus[4], bus_b_high);
                println!("0x{0:X} ({0}) vs 0x{1:X} ({1})", bus[5], res_low);
                println!("0x{0:X} ({0}) vs 0x{1:X} ({1})", bus[6], res_high);
                println!("[\x1B[31mFAIL BUS\x1B[0m]");
                false
            } else {
                println!("[\x1B[32mOK BUS\x1B[0m]");
                true
            };
        // check all chunks and carries
        let carry_min_value: i64 = -0x0F_FFFF;
        let carry_max_value: i64 = 0x0F_FFFF;
        for index in 0..8 {
            passed = passed
                && if carrys[index] > carry_max_value || carrys[index] < carry_min_value {
                    println!("[\x1B[31mFAIL CARRY RANGE CHECK\x1B[0m]");
                    false
                } else {
                    println!("[\x1B[32mOK CARRY RANGE CHECK\x1B[0m]");
                    true
                };
        }
        let ranges = Self::decode_ranges(range_ab, range_cd);
        Self::check_range(0, a_chunks[0]);
        Self::check_range(0, b_chunks[0]);
        Self::check_range(0, c_chunks[0]);
        Self::check_range(0, d_chunks[0]);

        Self::check_range(ranges[4], a_chunks[1]);
        Self::check_range(ranges[5], b_chunks[1]);
        Self::check_range(ranges[6], c_chunks[1]);
        Self::check_range(ranges[7], d_chunks[1]);

        Self::check_range(0, a_chunks[2]);
        Self::check_range(0, b_chunks[2]);
        Self::check_range(0, c_chunks[2]);
        Self::check_range(0, d_chunks[2]);

        Self::check_range(ranges[0], a_chunks[3]);
        Self::check_range(ranges[1], b_chunks[3]);
        Self::check_range(ranges[2], c_chunks[3]);
        Self::check_range(ranges[3], d_chunks[3]);

        passed
    }
    fn check_range(range_id: u64, value: i64) {
        assert!(range_id != 0 || (value >= 0 && value <= 0xFFFF));
        assert!(range_id != 1 || (value >= 0 && value <= 0x7FFF));
        assert!(range_id != 2 || (value >= 0x8000 && value <= 0xFFFF));
    }
}

fn flags_to_strings(mut flags: u64, flag_names: &[&str]) -> String {
    let mut res = String::new();

    for flag_name in flag_names {
        if (flags & 1u64) != 0 {
            if !res.is_empty() {
                res = res + ",";
            }
            res = res + *flag_name;
        }
        flags >>= 1;
        if flags == 0 {
            break;
        };
    }
    res
}

const F_M32: u64 = 0x0001;
const F_DIV: u64 = 0x0002;
const F_NA: u64 = 0x0004;
const F_NB: u64 = 0x0008;
const F_NP: u64 = 0x0010;
const F_NR: u64 = 0x0020;
const F_SEXT: u64 = 0x0040;
const F_SEC: u64 = 0x0080;

// range_ab / range_cd
//
//     a3 a1 b3 b1
// rid c3 c1 d3 d1 range 2^16 2^15 notes
// --- -- -- -- -- ----- ---- ---- -------------------------

const R_FF: u64 = 0; //   0  F  F  F  F ab cd    4    0
const R_3FP: u64 = 1; //   1  F  F  +  F    cd    3    1 b3 sign => a3 sign
const R_3FN: u64 = 2; //   2  F  F  -  F    cd    3    1 b3 sign => a3 sign
const R_3PF: u64 = 3; //   3  +  F  F  F ab       3    1 c3 sign => d3 sign
const R_3PP: u64 = 4; //   4  +  F  +  F ab cd    2    2
const R_3PN: u64 = 5; //   5  +  F  -  F ab cd    2    2
const R_3NF: u64 = 6; //   6  -  F  F  F ab       3    1 c3 sign => d3 sign
const R_3NP: u64 = 7; //   7  -  F  +  F ab cd    2    2
const R_3NN: u64 = 8; //   8  -  F  -  F ab cd    2    2
const R_1FP: u64 = 9; //   9  F  F  F  +    cd           a1 sign <=> b1 sign / d1 sign => c1 sign
const R_1FN: u64 = 10; //  10  F  F  F  -    cd           a1 sign <=> b1 sign / d1 sign => c1 sign
const R_1PF: u64 = 11; //  11  F  +  F  F    cd    3    1 a1 sign <=> b1 sign
const R_1PP: u64 = 12; //  12  F  +  F  + ab cd    2    2
const R_1PN: u64 = 13; //  13  F  +  F  - ab cd    2    2
const R_1NF: u64 = 14; //  14  F  -  F  F    cd    3    1 a1 sign <=> b1 sign
const R_1NP: u64 = 15; //  15  F  -  F  + ab cd    2    2
const R_1NN: u64 = 16; //  16  F  -  F  - ab cd    2    2

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

const ALL_32: u64 = 0x0043;
const ALL_NZ_32: u64 = 0x0044;
const ALL_P_32: u64 = 0x0045;
const ALL_N_32: u64 = 0x0046;
const ALL_NZ_P_32: u64 = 0x0047;

const VALUES_END: u64 = 0x004D;

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
#[test]
fn test_calculate_range_checks() {
    struct TestArithHelpers {}
    impl ArithHelpers for TestArithHelpers {}
    struct TestParams {
        op: u8,
        a: u64,
        b: u64,
        flags: u64,
        range_ab: u64,
        range_cd: u64,
    }

    // NOTE: update TEST_COUNT with number of tests, ALL,ALL => 3*3 = 9
    const TEST_COUNT: u32 = 2510;

    // NOTE: use 0x0000_0000 instead of 0, to avoid auto-format in one line, 0 is too short.
    let tests = [
        // 0 - MULU
        TestParams {
            op: MULU,
            a: ALL_64,
            b: ALL_64,
            flags: 0x0000,
            range_ab: R_FF,
            range_cd: R_FF,
        },
        // 1 - MULUH
        TestParams {
            op: MULUH,
            a: ALL_64,
            b: ALL_64,
            flags: F_SEC,
            range_ab: R_FF,
            range_cd: R_FF,
        },
        // 2 - MULSUH
        TestParams {
            op: MULSUH,
            a: ALL_P_64,
            b: ALL_64,
            flags: F_SEC,
            range_ab: R_3PF,
            range_cd: R_3FP,
        },
        // 3 - MULSUH
        TestParams {
            op: MULSUH,
            a: ALL_N_64,
            b: ALL_NZ_64,
            flags: F_NA + F_NP + F_SEC,
            range_ab: R_3NF,
            range_cd: R_3FN,
        },
        // 4 - MULSUH
        TestParams {
            op: MULSUH,
            a: ALL_N_64,
            b: 0x0000_0000,
            flags: F_NA + F_SEC,
            range_ab: R_3NF,
            range_cd: R_3FP,
        },
        // 5 - MUL
        TestParams {
            op: MUL,
            a: ALL_P_64,
            b: ALL_P_64,
            flags: 0,
            range_ab: R_3PP,
            range_cd: R_3FP,
        },
        // 6 - MUL
        TestParams {
            op: MUL,
            a: ALL_N_64,
            b: ALL_N_64,
            flags: F_NA + F_NB,
            range_ab: R_3NN,
            range_cd: R_3FP,
        },
        // 7 - MUL
        TestParams {
            op: MUL,
            a: ALL_N_64,
            b: ALL_NZ_P_64,
            flags: F_NA + F_NP,
            range_ab: R_3NP,
            range_cd: R_3FN,
        },
        // 8 - MUL
        TestParams {
            op: MUL,
            a: ALL_N_64,
            b: 0x0000_0000,
            flags: F_NA,
            range_ab: R_3NP,
            range_cd: R_3FP,
        },
        // 9 - MUL
        TestParams {
            op: MUL,
            a: ALL_NZ_P_64,
            b: ALL_N_64,
            flags: F_NB + F_NP,
            range_ab: R_3PN,
            range_cd: R_3FN,
        },
        // 10 - MUL
        TestParams {
            op: MUL,
            a: 0x0000_0000,
            b: ALL_N_64,
            flags: F_NB,
            range_ab: R_3PN,
            range_cd: R_3FP,
        },
        // 11 - MULH
        TestParams {
            op: MULH,
            a: ALL_P_64,
            b: ALL_P_64,
            flags: F_SEC,
            range_ab: R_3PP,
            range_cd: R_3FP,
        },
        // 12 - MULH
        TestParams {
            op: MULH,
            a: ALL_N_64,
            b: ALL_N_64,
            flags: F_NA + F_NB + F_SEC,
            range_ab: R_3NN,
            range_cd: R_3FP,
        },
        // 13 - MULH
        TestParams {
            op: MULH,
            a: ALL_N_64,
            b: ALL_NZ_P_64,
            flags: F_NA + F_NP + F_SEC,
            range_ab: R_3NP,
            range_cd: R_3FN,
        },
        // 14 - MULH
        TestParams {
            op: MULH,
            a: ALL_N_64,
            b: 0x0000_00000,
            flags: F_NA + F_SEC,
            range_ab: R_3NP,
            range_cd: R_3FP,
        },
        // 15 - MULH
        TestParams {
            op: MULH,
            a: ALL_NZ_P_64,
            b: ALL_N_64,
            flags: F_NB + F_NP + F_SEC,
            range_ab: R_3PN,
            range_cd: R_3FN,
        },
        // 16 - MULH
        TestParams {
            op: MULH,
            a: 0x0000_0000,
            b: ALL_N_64,
            flags: F_NB + F_SEC,
            range_ab: R_3PN,
            range_cd: R_3FP,
        },
        // 17 - MUL_W
        TestParams {
            op: MUL_W,
            a: 0x0000_0000,
            b: 0x0000_0000,
            flags: F_M32,
            range_ab: R_FF,
            range_cd: R_1PF,
        },
        // 18 - MUL_W: 0x00000002 (+/32 bits) * 0x40000000 (+/32 bits) = 0x80000000 (-/32 bits)
        TestParams {
            op: MUL_W,
            a: 0x0000_0002,
            b: 0x4000_0000,
            flags: F_M32 + F_SEXT,
            range_ab: R_FF,
            range_cd: R_1NF,
        },
        // 19 - MUL_W
        TestParams {
            op: MUL_W,
            a: 0x0000_0002,
            b: 0x8000_0000,
            flags: F_M32,
            range_ab: R_FF,
            range_cd: R_1PF,
        },
        // 20 - MUL_W
        TestParams {
            op: MUL_W,
            a: 0xFFFF_FFFF,
            b: 1,
            flags: F_M32 + F_SEXT,
            range_ab: R_FF,
            range_cd: R_1NF,
        },
        // 21 - MUL_W
        TestParams {
            op: MUL_W,
            a: 0xFFFF_FFFF,
            b: 0x0000_00000,
            flags: F_M32,
            range_ab: R_FF,
            range_cd: R_1PF,
        },
        // 22 - MUL_W
        TestParams {
            op: MUL_W,
            a: 0x7FFF_FFFF,
            b: 2,
            flags: F_M32 + F_SEXT,
            range_ab: R_FF,
            range_cd: R_1NF,
        },
        // 23 - MUL_W
        TestParams {
            op: MUL_W,
            a: 0xBFFF_FFFF,
            b: 0x0000_0002,
            flags: F_M32,
            range_ab: R_FF,
            range_cd: R_1PF,
        },
        // 24 - MUL_W: 0xFFFF_FFFF * 0xFFFF_FFFF = 0xFFFF_FFFE_0000_0001
        TestParams {
            op: MUL_W,
            a: 0xFFFF_FFFF,
            b: 0xFFFF_FFFF,
            flags: F_M32,
            range_ab: R_FF,
            range_cd: R_1PF,
        },
        // 25 - MUL_W: 0xFFFF_FFFF * 0x0FFF_FFFF = 0x0FFF_FFFE_F000_0001
        TestParams {
            op: MUL_W,
            a: 0xFFFF_FFFF,
            b: 0x0FFF_FFFF,
            flags: F_M32 + F_SEXT,
            range_ab: R_FF,
            range_cd: R_1NF,
        },
        // 26 - MUL_W: 0x8000_0000 * 0x8000_0000 = 0x4000_0000_0000_0000
        TestParams {
            op: MUL_W,
            a: 0x8000_0000,
            b: 0x8000_0000,
            flags: F_M32,
            range_ab: R_FF,
            range_cd: R_1PF,
        },
        // 27 - DIVU
        TestParams {
            op: DIVU,
            a: ALL_64,
            b: ALL_NZ_64,
            flags: F_DIV + 0,
            range_ab: R_FF,
            range_cd: R_FF,
        },
        // 28 - REMU
        TestParams {
            op: REMU,
            a: ALL_64,
            b: ALL_NZ_64,
            flags: F_DIV + F_SEC,
            range_ab: R_FF,
            range_cd: R_FF,
        },
        // 29 - DIV
        TestParams {
            op: DIV,
            a: MAX_P_64,
            b: MAX_P_64,
            flags: F_DIV,
            range_ab: R_3PP,
            range_cd: R_3PP,
        },
        // 30 - DIV
        TestParams {
            op: DIV,
            a: MIN_N_64,
            b: MAX_P_64,
            flags: F_DIV + F_NA + F_NP + F_NR,
            range_ab: R_3NP,
            range_cd: R_3NN,
        },
        // 31 - DIV
        TestParams {
            op: DIV,
            a: MAX_P_64,
            b: MIN_N_64,
            flags: F_DIV + F_NB, // a/b = 0 ➜ np = 0
            range_ab: R_3PN,
            range_cd: R_3PP,
        },
        // 32 - DIV
        TestParams {
            op: DIV,
            a: MIN_N_64,
            b: MIN_N_64,
            flags: F_DIV + F_NB + F_NP, // a/b = 1 ➜ 1 * b_neg ➜ np = 1
            range_ab: R_3PN,
            range_cd: R_3NP,
        },
        // 33 - DIV
        TestParams {
            op: DIV,
            a: 0x0000_0000,
            b: MAX_P_64,
            flags: F_DIV,
            range_ab: R_3PP,
            range_cd: R_3PP,
        },
        // 34 - DIV
        TestParams {
            op: DIV,
            a: 0x0000_0000,
            b: MIN_N_64,
            flags: F_DIV + F_NB,
            range_ab: R_3PN,
            range_cd: R_3PP,
        },
        // 35 - REM
        TestParams {
            op: REM,
            a: MAX_P_64,
            b: MAX_P_64,
            flags: F_DIV + F_SEC,
            range_ab: R_3PP,
            range_cd: R_3PP,
        },
        // 36 - REM
        TestParams {
            op: REM,
            a: MIN_N_64,
            b: MAX_P_64,
            flags: F_DIV + F_NA + F_NP + F_NR + F_SEC,
            range_ab: R_3NP,
            range_cd: R_3NN,
        },
        // 37 - REM
        TestParams {
            op: REM,
            a: MAX_P_64,
            b: MIN_N_64,
            flags: F_DIV + F_NB + F_SEC,
            range_ab: R_3PN,
            range_cd: R_3PP,
        },
        // 38 - REM
        TestParams {
            op: REM,
            a: MIN_N_64,
            b: MIN_N_64,
            flags: F_DIV + F_NB + F_NP + F_SEC,
            range_ab: R_3PN,
            range_cd: R_3NP,
        },
        // 39 - REM
        TestParams {
            op: REM,
            a: 0x0000_0000,
            b: MAX_P_64,
            flags: F_DIV + F_SEC,
            range_ab: R_3PP,
            range_cd: R_3PP,
        },
        // 40 - REM
        TestParams {
            op: REM,
            a: 0x0000_0000,
            b: MIN_N_64,
            flags: F_DIV + F_NB + F_SEC,
            range_ab: R_3PN,
            range_cd: R_3PP,
        },
        // 41 - DIVU_W
        TestParams {
            op: DIVU_W,
            a: 0xFFFF_FFFF,
            b: 0x0000_0001,
            flags: F_DIV + F_M32 + F_SEXT,
            range_ab: R_1NF,
            range_cd: R_FF,
        },
        // 42 - DIVU_W
        TestParams {
            op: DIVU_W,
            a: ALL_NZ_32,
            b: 0x0000_00002,
            flags: F_DIV + F_M32,
            range_ab: R_1PF,
            range_cd: R_FF,
        },
        // 43 - DIVU_W
        TestParams {
            op: DIVU_W,
            a: ALL_NZ_32,
            b: MAX_32,
            flags: F_DIV + F_M32,
            range_ab: R_1PF,
            range_cd: R_FF,
        },
        // 44 - DIVU_W
        TestParams {
            op: DIVU_W,
            a: 0,
            b: ALL_NZ_32,
            flags: F_DIV + F_M32,
            range_ab: R_1PF,
            range_cd: R_FF,
        },
        // 45 - REMU_W
        TestParams {
            op: REMU_W,
            a: 0xFFFF_FFFF,
            b: 0x0000_0001,
            flags: F_DIV + F_M32 + F_SEC,
            range_ab: R_FF,
            range_cd: R_1FP,
        },
        // 46 - REMU_W
        TestParams {
            op: REMU_W,
            a: ALL_32,
            b: 0x0000_00002,
            flags: F_DIV + F_M32 + F_SEC,
            range_ab: R_FF,
            range_cd: R_1FP,
        },
        // 47 - REMU_W
        TestParams {
            op: REMU_W,
            a: ALL_NZ_P_32,
            b: MAX_32,
            flags: F_DIV + F_M32 + F_SEC,
            range_ab: R_FF,
            range_cd: R_1FP,
        },
        // 48 - REMU_W
        TestParams {
            op: REMU_W,
            a: ALL_32,
            b: 0x8000_0000,
            flags: F_DIV + F_M32 + F_SEC,
            range_ab: R_FF,
            range_cd: R_1FP,
        },
        // 49 - REMU_W
        TestParams {
            op: REMU_W,
            a: 0,
            b: ALL_NZ_32,
            flags: F_DIV + F_M32 + F_SEC,
            range_ab: R_FF,
            range_cd: R_1FP,
        },
        // 50 - REMU_W
        TestParams {
            op: REMU_W,
            a: 0xFFFF_FFFE,
            b: 0xFFFF_FFFF,
            flags: F_DIV + F_M32 + F_SEXT + F_SEC,
            range_ab: R_FF,
            range_cd: R_1FN,
        },
        // 51 - REMU_W
        TestParams {
            op: REMU_W,
            a: 0xFFFF_FFFE,
            b: 0xFFFF_FFFE,
            flags: F_DIV + F_M32 + F_SEC,
            range_ab: R_FF,
            range_cd: R_1FP,
        },
        // 52 - REMU_W
        TestParams {
            op: REMU_W,
            a: 0x8000_0000,
            b: 0x8000_0001,
            flags: F_DIV + F_M32 + F_SEXT + F_SEC,
            range_ab: R_FF,
            range_cd: R_1FN,
        },
        // 53 - REMU_W
        TestParams {
            op: REMU_W,
            a: 0x8000_0001,
            b: 0x8000_0000,
            flags: F_DIV + F_M32 + F_SEC,
            range_ab: R_FF,
            range_cd: R_1FP,
        },
        // 54 - REMU_W
        TestParams {
            op: REMU_W,
            a: 0xFFFF_FFFF,
            b: 0x0000_0003,
            flags: F_DIV + F_M32 + F_SEC,
            range_ab: R_FF,
            range_cd: R_1FP,
        },
        // 55 - DIV_W (-1/1=-1 REM:0)
        TestParams {
            op: DIV_W,
            a: 0xFFFF_FFFF,
            b: 0x0000_0001,
            flags: F_DIV + F_NA + F_NP + F_M32 + F_SEXT,
            range_ab: R_1NP,
            range_cd: R_1NP,
        },
        // 56 - REM_W !!!
        TestParams {
            op: REM_W,
            a: 0xFFFF_FFFF,
            b: 0x0000_0001,
            flags: F_DIV + F_NA + F_NP + F_M32 + F_SEC,
            range_ab: R_1NP,
            range_cd: R_1NP,
        },
        // 57 - DIV_W <======
        TestParams {
            op: DIV_W,
            a: 0xFFFF_FFFF,
            b: 0x0000_0002,
            flags: F_DIV + F_NP + F_NR + F_M32,
            range_ab: R_1PP,
            range_cd: R_1NN,
        },
        // 58 - REM_W
        TestParams {
            op: REM_W,
            a: 0xFFFF_FFFF,
            b: 0x0000_0002,
            flags: F_DIV + F_NP + F_NR + F_M32 + F_SEC + F_SEXT,
            range_ab: R_1PP,
            range_cd: R_1NN,
        },
    ];

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

    let mut tests_done: Vec<TestDone> = Vec::new();
    let mut errors = 0;
    for test in tests {
        let a_values = get_test_values(test.a);
        let mut offset = 0;
        for _a in a_values {
            if _a == VALUES_END {
                break;
            }
            let b_values = get_test_values(test.b);
            for _b in b_values {
                if _b == VALUES_END {
                    break;
                }
                let test_info = TestDone { op: test.op, a: _a, b: _b, index, offset };
                let previous = tests_done
                    .iter()
                    .find(|&x| x.op == test_info.op && x.a == test_info.a && x.b == test_info.b);
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
                println!("testing #{} op:0x{:x} with _a:0x{:X} _b:0x{:X}", index, test.op, _a, _b);
                let (emu_c, emu_flag) = TestArithHelpers::calculate_emulator_res(test.op, _a, _b);
                let [a, b, c, d] = TestArithHelpers::calculate_abcd_from_ab(test.op, _a, _b);

                let [m32, div, na, nb, np, nr, sext, sec, range_ab, range_cd, ranges] =
                    TestArithHelpers::calculate_flags_and_ranges(test.op, a, b, c, d);

                let flags =
                    m32 + div * 2 + na * 4 + nb * 8 + np * 16 + nr * 32 + sext * 64 + sec * 128;

                let row = TestArithHelpers::get_row(test.op, na, nb, np, nr, sext);
                println!(
                    "#{} op:0x{:x} na:{} nb:{} np:{} nr:{} sext:{}",
                    row, test.op, na, nb, np, nr, sext
                );
                assert_eq!(
                    [flags, range_ab, range_cd],
                    [test.flags, test.range_ab, test.range_cd],
                    "testing #{} op:0x{:x} with _a:0x{:X} _b:0x{:X} a:0x{:X} b:0x{:X} c:0x{:X} d:0x{:X} EMU:0x{:X} flags:{:b}[{}]/{:b}[{}] range_ab:{}/{}  range_cd:{}/{} ranges:{}",
                    index,
                    test.op,
                    _a,
                    _b,
                    a,
                    b,
                    c,
                    d,
                    emu_c,
                    flags,
                    flags_to_strings(flags, &FLAG_NAMES),
                    test.flags,
                    flags_to_strings(test.flags, &FLAG_NAMES),
                    range_ab,
                    test.range_ab,
                    range_cd,
                    test.range_cd,
                    ranges
                );
                println!("testing #{} op:0x{:x} with _a:0x{:X} _b:0x{:X} a:0x{:X} b:0x{:X} c:0x{:X} d:0x{:X} EMU:0x{:X} flags:{:b}[{}]/{:b}[{}] range_ab:{}/{}  range_cd:{}/{} ranges:{}",
                    index,
                    test.op,
                    _a,
                    _b,
                    a,
                    b,
                    c,
                    d,
                    emu_c,
                    flags,
                    flags_to_strings(flags, &FLAG_NAMES),
                    test.flags,
                    flags_to_strings(test.flags, &FLAG_NAMES),
                    range_ab,
                    test.range_ab,
                    range_cd,
                    test.range_cd,
                    ranges
                );
                assert_ne!(row, -1);
                let bus: [u64; 8] = [
                    test.op as u64,
                    _a & 0xFFFF_FFFF,
                    _a >> 32,
                    _b & 0xFFFF_FFFF,
                    _b >> 32,
                    emu_c & 0xFFFF_FFFF,
                    emu_c >> 32,
                    if emu_flag { 1 } else { 0 },
                ];
                if !TestArithHelpers::execute_chunks(
                    a, b, c, d, m32, div, na, nb, np, nr, sec, sext, range_ab, range_cd, bus,
                ) {
                    errors += 1;
                    println!("TOTAL ERRORS: {}", errors);
                }
                offset += 1;
                count += 1;
            }
        }
        index += 1;
    }
    println!("TOTAL ERRORS: {}", errors);
    assert_eq!(count, TEST_COUNT, "Number of tests not matching");
}
