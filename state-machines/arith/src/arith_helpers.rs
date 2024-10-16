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

pub trait ArithHelpers {
    fn calculate_flags_and_ranges(
        a: u64,
        b: u64,
        op: u8,
        div: &mut u64,
        m32: &mut u64,
        na: &mut u64,
        nb: &mut u64,
        nr: &mut u64,
        np: &mut u64,
        na32: &mut u64,
        nd32: &mut u64,
    ) -> [u64; 8] {
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

        let mut sa: u64 = 0;
        let mut sb: u64 = 0;

        match op {
            MULU | MULUH => {}
            MULSUH => {
                sa = 1;
            }
            MUL | MULH => {
                sa = 1;
                sb = 1;
            }
            MUL_W => {
                *m32 = 1;
                sa = 1;
                sb = 1;
            }
            DIVU | REMU => {
                *div = 1;
            }
            DIV | REM => {
                sa = 1;
                sb = 1;
                *div = 1;
            }
            DIVU_W | REMU_W => {
                // divu_w, remu_w
                *div = 1;
                *m32 = 1;
            }
            DIV_W | REM_W => {
                // div_w, rem_w
                sa = 1;
                sb = 1;
                *div = 1;
                *m32 = 1;
            }
            _ => {
                panic!("Invalid opcode");
            }
        }
        *na = if sa == 1 && (a as i64) < 0 { 1 } else { 0 };
        *nb = if sb == 1 && (b as i64) < 0 { 1 } else { 0 };
        *np = *na ^ *nb;
        *nr = if *div == 1 { *na } else { 0 };
        *na32 = if *m32 == 1 { *na } else { 0 };
        *nd32 = if *m32 == 1 { *nr } else { 0 };

        if *m32 == 1 {
            range_a1 = sa + *na;
            range_b1 = sb + *nb;

            if *div == 1 {
                range_c1 = if *np == 1 || *na32 == 1 { 2 } else { 1 };
                range_d1 = if (*np == 1 && sa == 1) || *nd32 == 1 { 1 } else { 2 };
            } else {
                range_c1 = 1 + *na32;
            }
        } else {
            // m32 = 0
            range_b3 = if sb == 1 { 1 + *na } else { 0 };
            if sa == 1 {
                // !m32 && sa
                range_a3 = 1 + *na;
                if *div == 1 {
                    // !m32 && sa && div
                    range_c3 = 1 + *np;
                    range_d3 = range_c3;
                }
            }
        }

        [range_a1, range_b1, range_c1, range_d1, range_a3, range_b3, range_c3, range_d3]
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
        div: i64,
        fab: i64,
        na: i64,
        nb: i64,
        np: i64,
        nr: i64,
        m32: i64,
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

#[test]
fn test_calculate_range_checks() {
    struct TestArithHelpers {}
    impl ArithHelpers for TestArithHelpers {}

    const MIN_N_64: u64 = 0x8000_0000_0000_0000;
    const MAX_P_64: u64 = 0x7FFF_FFFF_FFFF_FFFF;
    const MAX_64: u64 = 0xFFFF_FFFF_FFFF_FFFF;

    const ALL: u64 = 0x0033;
    const ALL_P_64: u64 = 0x0034;
    const ALL_N_64: u64 = 0x0035;

    const END: u64 = 0x0036;
    const ALL_P_64_VALUES: [u64; 5] = [0, 1, MAX_P_64, END, 0];
    const ALL_N_64_VALUES: [u64; 5] = [MIN_N_64, MAX_64, END, 0, 0];
    const ALL_64_VALUES: [u64; 5] = [0, 1, MAX_P_64, MAX_64, MIN_N_64];

    const F_M32: u64 = 0x0001;
    const F_DIV: u64 = 0x0002;
    const F_NA: u64 = 0x0004;
    const F_NB: u64 = 0x0008;
    const F_NP: u64 = 0x0010;
    const F_NR: u64 = 0x0020;
    const F_NA32: u64 = 0x0040;
    const F_ND32: u64 = 0x0080;

    struct TestParams {
        op: u8,
        a: u64,
        b: u64,
        flags: u64,
    }

    // NOTE: update TEST_COUNT with number of tests, ALL,ALL => 3*3 = 9
    const TEST_COUNT: u32 = 20;

    let tests = [
        // flags: div, m32, sa, sb, na, nr, np, np, na32, nd32
        TestParams { op: MULU, a: ALL, b: ALL, flags: 0 },
        TestParams { op: MULUH, a: ALL, b: ALL, flags: 0 },
        TestParams { op: MULSUH, a: ALL_P_64, b: ALL, flags: 0 },
        TestParams { op: MULSUH, a: ALL_N_64, b: ALL, flags: F_NA + F_NP },
        TestParams { op: MUL_W, a: ALL_P_64, b: ALL_P_64, flags: F_M32 },
        TestParams { op: MUL_W, a: ALL_N_64, b: ALL_P_64, flags: F_M32 + F_NA + F_NP },
        TestParams { op: MUL_W, a: ALL_P_64, b: ALL_N_64, flags: F_M32 + F_NB + F_NP },
        TestParams { op: MUL_W, a: ALL_N_64, b: ALL_N_64, flags: F_M32 + F_NA + F_NB },
        TestParams { op: DIV, a: 0, b: 0, flags: F_DIV },
        TestParams { op: DIV, a: MIN_N_64, b: MAX_P_64, flags: F_DIV + F_NA + F_NP + F_NR },
    ];

    let mut count = 0;
    let mut index: u32 = 0;
    for test in tests {
        let a_values = if test.a == ALL {
            ALL_64_VALUES
        } else if test.a == ALL_N_64 {
            ALL_N_64_VALUES
        } else if test.a == ALL_P_64 {
            ALL_P_64_VALUES
        } else {
            [test.a, END, 0, 0, 0]
        };
        for a in a_values {
            if a == END {
                break;
            };
            let b_values = if test.b == ALL {
                ALL_64_VALUES
            } else if test.b == ALL_N_64 {
                ALL_N_64_VALUES
            } else if test.b == ALL_P_64 {
                ALL_P_64_VALUES
            } else {
                [test.b, END, 0, 0, 0]
            };
            for b in b_values {
                if b == END {
                    break;
                };
                let mut div: u64 = 0;
                let mut m32: u64 = 0;
                let mut na: u64 = 0;
                let mut nb: u64 = 0;
                let mut nr: u64 = 0;
                let mut np: u64 = 0;
                let mut na32: u64 = 0;
                let mut nd32: u64 = 0;

                TestArithHelpers::calculate_flags_and_ranges(
                    a, b, test.op, &mut div, &mut m32, &mut na, &mut nb, &mut nr, &mut np,
                    &mut na32, &mut nd32,
                );
                let flags =
                    m32 + div * 2 + na * 4 + nb * 8 + np * 16 + nr * 32 + na32 * 64 + nd32 * 128;

                assert_eq!(
                    flags,
                    test.flags,
                    "testing #{} op:0x{:x} with a:0x{:X} b:0x{:X} flags:{:b} vs {:b} [div, m32, sa, sb, na, nb, np, nr, na32, nd32]",
                    index,
                    test.op,
                    a,
                    b,
                    flags,
                    test.flags,
                );
                count += 1;
            }
        }
        index += 1;
    }
    assert_eq!(count, TEST_COUNT, "Number of tests not matching");
}
