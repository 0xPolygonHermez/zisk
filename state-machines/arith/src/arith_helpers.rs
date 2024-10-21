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

const FLAG_NAMES: [&str; 7] = ["m32", "div", "na", "nb", "np", "nr", "sext"];

pub trait ArithHelpers {
    fn calculate_flags_and_ranges(a: u64, b: u64, op: u8) -> [u64; 10] {
        let mut m32: u64 = 0;
        let mut div: u64 = 0;
        let mut na: u64 = 0;
        let mut nb: u64 = 0;
        let mut np: u64 = 0;
        let mut nr: u64 = 0;
        let mut sext: u64 = 0;

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
        let mut rem32 = false;

        match op {
            MULU => {}
            MULUH => {}
            MULSUH => {
                sa = true;
            }
            MUL => {
                sa = true;
                sb = true;
            }
            MULH => {
                sa = true;
                sb = true;
            }
            MUL_W => {
                m32 = 1;
                sext = if ((a * b) & 0xFFFF_FFFF) & 0x8000_0000 != 0 { 1 } else { 0 };
            }
            DIVU => {
                div = 1;
            }
            REMU => {
                div = 1;
            }
            DIV => {
                sa = true;
                sb = true;
                div = 1;
            }
            REM => {
                sa = true;
                sb = true;
                div = 1;
            }
            DIVU_W => {
                // divu_w, remu_w
                div = 1;
                m32 = 1;
                sext = if ((a as u32 / b as u32) as i32) < 0 { 1 } else { 0 };
            }
            REMU_W => {
                // divu_w, remu_w
                div = 1;
                m32 = 1;
                rem32 = true;
                sext = if ((a as u32 % b as u32) as i32) < 0 { 1 } else { 0 };
            }
            DIV_W => {
                // div_w, rem_w
                sa = true;
                sb = true;
                div = 1;
                m32 = 1;
                sext = if (a as i32 / b as i32) < 0 { 1 } else { 0 };
            }
            REM_W => {
                // div_w, rem_w
                sa = true;
                sb = true;
                div = 1;
                m32 = 1;
                rem32 = true;
                sext = if (a as i32 % b as i32) < 0 { 1 } else { 0 };
            }
            _ => {
                panic!("Invalid opcode");
            }
        }
        if sa {
            na = if m32 == 1 {
                if (a as i32) < 0 {
                    1
                } else {
                    0
                }
            } else {
                if (a as i64) < 0 {
                    1
                } else {
                    0
                }
            }
        }
        if sb {
            nb = if m32 == 1 {
                if (b as i32) < 0 {
                    1
                } else {
                    0
                }
            } else {
                if (b as i64) < 0 {
                    1
                } else {
                    0
                }
            }
        }

        np = na ^ nb;
        nr = if div == 1 { na } else { 0 };

        if m32 == 1 {
            // mulw, divu_w, remu_w, div_w, rem_w
            range_a1 = if sa { 1 + na } else { 0 };
            range_b1 = if sb { 1 + nb } else { 0 };
            range_c1 = if !rem32 {
                sext + 1
            } else if sa {
                1 + np
            } else {
                0
            };
            range_d1 = if rem32 {
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
        [m32, div, na, nb, np, nr, sext, range_ab, range_cd, ranges]
    }
    /*
    fn calculate_flags(
        &self,
        op: u8,
        a: u64,
        b: u64,
        na: &mut i64,
        nb: &mut i64,
        nr: &mut i64,
        np: &mut i64,
        na32: &mut i64,
        nd32: &mut i64,
        m32: &mut i64,
        div: &mut i64,
        fab: &mut i64,
    ) -> [u64; 8] {
        let MUL_W = 1;
                match (op) {
                    MUL_W=> {
                        let na = if (a as i32) < 0 { 1 } else { 0 };
                        let nb = if (b as i32) < 0 { 1 } else { 0 };
                        let c = (a as i32 * b as i32);
                        let nc = if c < 0 { 1 } else { 0 };
                    }
                    MULSUH => {
                        let na = if (a as i64) < 0 { 1 } else { 0 };
                        let _na = input.a & (2n**63n) ? 1n : 0n;
                        let _a = _na ? 2n ** 64n - a : a;
                        let _prod = _a * b;
                        let _nc = _prod && _na;

                        _prod = _nc ? 2n**128n - _prod : _prod;
                        c = _prod & (2n**64n - 1n);
                        d = _prod >> 64n;
                        // console.log(input.c.toString(16), c.toString(16));
                        break;
                    }
                    case 'divu':
                    case 'divu_w': {
                        this.log(opdef.n,a,b);
                        const div = a / b;
                        const rem = a % b;
                        c = a;
                        a = div;
                        d = rem;
                        break;
                    }
                    case 'div': {
                        this.log('div',a,b);
                        let _na = input.a & (2n**63n) ? 1n : 0n;
                        let _a = _na ? 2n ** 64n - a : a;
                        let _nb = input.b & (2n**63n) ? 1n : 0n;
                        let _b = _nb ? 2n ** 64n - b : b;
                        const div = _a / _b;
                        const rem = _a % _b;
                        c = a;
                        a = (div && _na ^ _nb) ? 2n**64n - div : div;
                        d = (rem && _na) ? 2n**64n - rem : rem;
                        break;
                    }
                    case 'div_w': {
                        this.log('div_w',a,b);
                        let _na = input.a & (2n**31n) ? 1n : 0n;
                        let _a = _na ? 2n ** 32n - a : a;
                        let _nb = input.b & (2n**31n) ? 1n : 0n;
                        let _b = _nb ? 2n ** 32n - b : b;
                        this.log([_a,_b].map(x => x.toString(16)).join(' '));
                        const div = _a / _b;
                        const rem = _a % _b;
                        this.log(div, rem, _na, _nb)
                        c = a;
                        a = (div && (_na ^ _nb)) ? 2n**32n - div : div;
                        d = (rem && _na) ? 2n**32n - rem : rem;
                        this.log('[a,b,c,d]='+[a,b,c,d].map(x => x.toString(16)).join(' '));
                        break;
                    }
                }
                if (m32) {
                    this.log(opdef.a_signed, opdef.b_signed, a.toString(16), (a & 0x80000000n).toString(16));
                    a = (opdef.a_signed && a & 0x80000000n) ? a | 0xFFFFFFFF00000000n : a;
                    b = (opdef.b_signed && b & 0x80000000n) ? b | 0xFFFFFFFF00000000n : b;
                }

                return [a,b,c,d];
        [0, 0, 0, 0, 0, 0, 0, 0]
    } */
    fn calculate_chunks(
        &self,
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
    ) -> [i64; 8] {
        // TODO: unroll this function in variants (div,m32) and (na,nb,nr,np)
        // div, m32, na, nb === f(div,m32,na,nb) => fa, nb, nr
        // unroll means 16 variants ==> but more performance

        let mut chunks: [i64; 8] = [0, 0, 0, 0, 0, 0, 0, 0];

        chunks[0] = fab * a[0] * b[0]  // chunk9
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
            - c[2]
            + (2 * np) * c[2]
            + div * d[2]
            - 2 * nr * d[2]
            - np * div * m32
            + nr * m32;

        chunks[3] = fab * a[3] * b[0]    // chunk3
            + fab * a[2] * b[1]
            + fab * a[1] * b[2]
            + fab * a[0] * b[3]
            - c[3]
            + 2 * np * c[3]
            + div * d[3]
            - 2 * nr * d[3];

        chunks[4] = fab * a[3] * b[1]    // chunk4
            + fab * a[2] * b[2]
            + fab * a[1] * b[3]
            + b[0] * na * (1 - 2 * nb)
            + a[0] * nb * (1 - 2 * na)
            - np * div
            + m32
            - 2 * div * m32
            + nr * (1 - m32)
            - d[0] * (1 - div)
            + d[0] * 2 * np * (1 - div);

        chunks[5] = fab * a[3] * b[2]    // chunk5
            + fab * a[2] * b[3]
            + a[1] * nb * (1 - 2 * na)
            + b[1] * na * (1 - 2 * nb)
            - d[1] * (1 - div)
            + d[1] * 2 * np * (1 - div);

        chunks[6] = fab as i64 * a[3] * b[3]    // chunk6
            + a[2] * nb * (1 - 2 * na)
            + b[2] * na * (1 - 2 * nb)
            - d[2] * (1 - div)
            + d[2] * 2 * np * (1 - div);

        chunks[7] = 0x10000 * na * nb    // chunk7
            + b[3] * na * (1 - 2 * nb)
            + a[3] * nb * (1 - 2 * na)
            - 0x10000 * np * (1 - div) * (1 - m32)
            - d[3] * (1 - div)
            + d[3] * 2 * np * (1 - div);

        chunks
    }
    fn me() -> i32 {
        13
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
#[test]
fn test_calculate_range_checks() {
    struct TestArithHelpers {}
    impl ArithHelpers for TestArithHelpers {}

    const MIN_N_64: u64 = 0x8000_0000_0000_0000;
    const MAX_P_64: u64 = 0x7FFF_FFFF_FFFF_FFFF;
    const MAX_P_32: u64 = 0x0000_0000_FFFF_FFFF;
    const MAX_64: u64 = 0xFFFF_FFFF_FFFF_FFFF;

    const ALL_64: u64 = 0x0033;
    const ALL_P_64: u64 = 0x0034;
    const ALL_N_64: u64 = 0x0035;

    const END: u64 = 0x0036;
    const ALL_P_64_VALUES: [u64; 6] = [0, 1, MAX_P_32, MAX_P_64, 0, END];
    const ALL_N_64_VALUES: [u64; 6] = [MIN_N_64, MAX_64, END, 0, 0, 0];
    const ALL_64_VALUES: [u64; 6] = [0, 1, MAX_P_64, MAX_64, MIN_N_64, MAX_P_32];

    const F_M32: u64 = 0x0001;
    const F_DIV: u64 = 0x0002;
    const F_NA: u64 = 0x0004;
    const F_NB: u64 = 0x0008;
    const F_NP: u64 = 0x0010;
    const F_NR: u64 = 0x0020;
    const F_SEXT: u64 = 0x0040;

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

    struct TestParams {
        op: u8,
        a: u64,
        b: u64,
        flags: u64,
        range_ab: u64,
        range_cd: u64,
    }

    // NOTE: update TEST_COUNT with number of tests, ALL,ALL => 3*3 = 9
    const TEST_COUNT: u32 = 295;

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
        // 1 - MULU
        TestParams {
            op: MULUH,
            a: ALL_64,
            b: ALL_64,
            flags: 0x0000,
            range_ab: R_FF,
            range_cd: R_FF,
        },
        // 2 - MULSHU
        TestParams {
            op: MULSUH,
            a: ALL_P_64,
            b: ALL_64,
            flags: 0x0000,
            range_ab: R_3PF,
            range_cd: R_3FP,
        },
        // 3 - MULSHU
        TestParams {
            op: MULSUH,
            a: ALL_N_64,
            b: ALL_64,
            flags: F_NA + F_NP,
            range_ab: R_3NF,
            range_cd: R_3FN,
        },
        // 4 - MUL
        TestParams {
            op: MUL,
            a: ALL_P_64,
            b: ALL_P_64,
            flags: 0,
            range_ab: R_3PP,
            range_cd: R_3FP,
        },
        // 5 - MUL
        TestParams {
            op: MUL,
            a: ALL_N_64,
            b: ALL_N_64,
            flags: F_NA + F_NB,
            range_ab: R_3NN,
            range_cd: R_3FP,
        },
        // 6 - MUL
        TestParams {
            op: MUL,
            a: ALL_N_64,
            b: ALL_P_64,
            flags: F_NA + F_NP,
            range_ab: R_3NP,
            range_cd: R_3FN,
        },
        // 7 - MUL
        TestParams {
            op: MUL,
            a: ALL_P_64,
            b: ALL_N_64,
            flags: F_NB + F_NP,
            range_ab: R_3PN,
            range_cd: R_3FN,
        },
        // 8 - MULH
        TestParams {
            op: MULH,
            a: ALL_P_64,
            b: ALL_P_64,
            flags: 0,
            range_ab: R_3PP,
            range_cd: R_3FP,
        },
        // 9 - MULH
        TestParams {
            op: MULH,
            a: ALL_N_64,
            b: ALL_N_64,
            flags: F_NA + F_NB,
            range_ab: R_3NN,
            range_cd: R_3FP,
        },
        // 10 - MULH
        TestParams {
            op: MULH,
            a: ALL_N_64,
            b: ALL_P_64,
            flags: F_NA + F_NP,
            range_ab: R_3NP,
            range_cd: R_3FN,
        },
        // 11 - MULH
        TestParams {
            op: MULH,
            a: ALL_P_64,
            b: ALL_N_64,
            flags: F_NB + F_NP,
            range_ab: R_3PN,
            range_cd: R_3FN,
        },
        // 12 - MULW
        TestParams {
            op: MUL_W,
            a: 0x0000_0000,
            b: 0x0000_0000,
            flags: F_M32,
            range_ab: R_FF,
            range_cd: R_1PF,
        },
        // 13 - MUL: 0x00000002 (+/32 bits) * 0x40000000 (+/32 bits) = 0x80000000 (-/32 bits)
        TestParams {
            op: MUL_W,
            a: 0x0000_0002,
            b: 0x4000_0000,
            flags: F_M32 + F_SEXT,
            range_ab: R_FF,
            range_cd: R_1NF,
        },
        // 14 - MUL
        TestParams {
            op: MUL_W,
            a: 0x0000_0002,
            b: 0x8000_0000,
            flags: F_M32,
            range_ab: R_FF,
            range_cd: R_1PF,
        },
        // 15 - MUL
        TestParams {
            op: MUL_W,
            a: 0xFFFF_FFFF,
            b: 1,
            flags: F_M32 + F_SEXT,
            range_ab: R_FF,
            range_cd: R_1NF,
        },
        // 16 - MUL
        TestParams {
            op: MUL_W,
            a: 0xFFFF_FFFF,
            b: 0x0000_00000,
            flags: F_M32,
            range_ab: R_FF,
            range_cd: R_1PF,
        },
        // 17 - MUL
        TestParams {
            op: MUL_W,
            a: 0x7FFF_FFFF,
            b: 2,
            flags: F_M32 + F_SEXT,
            range_ab: R_FF,
            range_cd: R_1NF,
        },
        // 18 - MUL
        TestParams {
            op: MUL_W,
            a: 0xBFFF_FFFF,
            b: 0x0000_0002,
            flags: F_M32,
            range_ab: R_FF,
            range_cd: R_1PF,
        },
        // 19 - MUL: 0xFFFF_FFFF * 0xFFFF_FFFF = 0xFFFF_FFFE_0000_0001
        TestParams {
            op: MUL_W,
            a: 0xFFFF_FFFF,
            b: 0xFFFF_FFFF,
            flags: F_M32,
            range_ab: R_FF,
            range_cd: R_1PF,
        },
        // 20 - MUL: 0xFFFF_FFFF * 0x0FFF_FFFF = 0x0FFF_FFFE_F000_0001
        TestParams {
            op: MUL_W,
            a: 0xFFFF_FFFF,
            b: 0x0FFF_FFFF,
            flags: F_M32 + F_SEXT,
            range_ab: R_FF,
            range_cd: R_1NF,
        },
        // 21 - MUL: 0x8000_0000 * 0x8000_0000 = 0x4000_0000_0000_0000
        TestParams {
            op: MUL_W,
            a: 0x8000_0000,
            b: 0x8000_0000,
            flags: F_M32,
            range_ab: R_FF,
            range_cd: R_1PF,
        },
        // 22 - DIVU
        TestParams { op: DIVU, a: ALL_64, b: ALL_64, flags: F_DIV, range_ab: R_FF, range_cd: R_FF },
        // 23 - REMU
        TestParams { op: DIVU, a: ALL_64, b: ALL_64, flags: F_DIV, range_ab: R_FF, range_cd: R_FF },
        // 24 - DIV
        TestParams {
            op: DIV,
            a: MAX_P_64,
            b: MAX_P_64,
            flags: F_DIV,
            range_ab: R_3PP,
            range_cd: R_3PP,
        },
        // 25 - DIV
        TestParams {
            op: DIV,
            a: MIN_N_64,
            b: MAX_P_64,
            flags: F_DIV + F_NA + F_NP + F_NR,
            range_ab: R_3NP,
            range_cd: R_3NN,
        },
        // 26 - DIV
        TestParams {
            op: DIV,
            a: MAX_P_64,
            b: MIN_N_64,
            flags: F_DIV + F_NB + F_NP,
            range_ab: R_3PN,
            range_cd: R_3NP,
        },
        // 27 - DIV
        TestParams {
            op: DIV,
            a: MIN_N_64,
            b: MIN_N_64,
            flags: F_DIV + F_NA + F_NB + F_NR,
            range_ab: R_3NN,
            range_cd: R_3PN,
        },
        // REM
        // DIVU_W
        // REMU_W
        // DIV_W
        // REM_W
    ];

    let mut count = 0;
    let mut index: u32 = 0;
    for test in tests {
        let a_values = if test.a == ALL_64 {
            ALL_64_VALUES
        } else if test.a == ALL_N_64 {
            ALL_N_64_VALUES
        } else if test.a == ALL_P_64 {
            ALL_P_64_VALUES
        } else {
            [test.a, END, 0, 0, 0, 0]
        };
        for a in a_values {
            if a == END {
                break;
            };
            let b_values = if test.b == ALL_64 {
                ALL_64_VALUES
            } else if test.b == ALL_N_64 {
                ALL_N_64_VALUES
            } else if test.b == ALL_P_64 {
                ALL_P_64_VALUES
            } else {
                [test.b, END, 0, 0, 0, 0]
            };
            for b in b_values {
                if b == END {
                    break;
                };
                let [m32, div, na, nb, np, nr, sext, range_ab, range_cd, ranges] =
                    TestArithHelpers::calculate_flags_and_ranges(a, b, test.op);

                let flags = m32 + div * 2 + na * 4 + nb * 8 + np * 16 + nr * 32 + sext * 64;

                assert_eq!(
                    [flags, range_ab, range_cd],
                    [test.flags, test.range_ab, test.range_cd],
                    "testing #{} op:0x{:x} with a:0x{:X} b:0x{:X} flags:{:b}[{}]/{:b}[{}] range_ab:{}/{}  range_cd:{}/{} ranges:{}",
                    index,
                    test.op,
                    a,
                    b,
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
                count += 1;
            }
        }
        index += 1;
    }
    assert_eq!(count, TEST_COUNT, "Number of tests not matching");
}
