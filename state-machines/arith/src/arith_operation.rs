//! The `ArithOperation` module provides a comprehensive implementation for arithmetic operations,
//! including support for signed and unsigned arithmetic, multiplication, division, and modular
//! arithmetic. It manages inputs, intermediate states, flags, and results to facilitate the
//! execution of complex operations.

use zisk_core::zisk_ops::ZiskOp;

use crate::arith_range_table_helpers::*;
use std::fmt;

/// Represents an arithmetic operation, including its inputs, results, and intermediate flags.
pub struct ArithOperation {
    pub op: u8,
    pub input_a: u64,
    pub input_b: u64,
    pub a: [u64; 4],
    pub b: [u64; 4],
    pub c: [u64; 4],
    pub d: [u64; 4],
    pub carry: [i64; 7],
    pub m32: bool,
    pub div: bool,
    pub na: bool,
    pub nb: bool,
    pub np: bool,
    pub nr: bool,
    pub sext: bool,
    pub main_mul: bool,
    pub main_div: bool,
    pub signed: bool,
    pub range_ab: u8,
    pub range_cd: u8,
    pub div_by_zero: bool,
    pub div_overflow: bool,
}

/// Provides a default implementation for `ArithOperation`.
impl Default for ArithOperation {
    fn default() -> Self {
        Self::new()
    }
}
impl fmt::Debug for ArithOperation {
    /// Formats the `ArithOperation` for debugging, including flags, inputs, and chunk values.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut flags = String::new();
        if self.m32 {
            flags += "m32 "
        };
        if self.div {
            flags += "div "
        };
        if self.na {
            flags += "na "
        };
        if self.nb {
            flags += "nb "
        };
        if self.np {
            flags += "np "
        };
        if self.nr {
            flags += "nr "
        };
        if self.sext {
            flags += "sext "
        };
        if self.div_by_zero {
            flags += "div_by_zero "
        };
        if self.div_overflow {
            flags += "div_overflow "
        };
        if self.main_mul {
            flags += "main_mul "
        };
        if self.main_div {
            flags += "main_div "
        };
        if self.signed {
            flags += "signed "
        };
        writeln!(f, "operation 0x{:x} flags={}", self.op, flags)?;
        writeln!(f, "input_a: 0x{0:x}({0})", self.input_a)?;
        writeln!(f, "input_b: 0x{0:x}({0})", self.input_b)?;
        self.dump_chunks(f, "a", &self.a)?;
        self.dump_chunks(f, "b", &self.b)?;
        self.dump_chunks(f, "c", &self.c)?;
        self.dump_chunks(f, "d", &self.d)?;
        writeln!(
            f,
            "carry: [0x{0:X}({0}), 0x{1:X}({1}), 0x{2:X}({2}), 0x{3:X}({3}), 0x{4:X}({4}), 0x{5:X}({5}), 0x{6:X}({6})]",
            self.carry[0], self.carry[1], self.carry[2], self.carry[3], self.carry[4], self.carry[5], self.carry[6]
        )?;
        writeln!(
            f,
            "range_ab: 0x{0:X} {1}, range_cd:0x{2:X} {3}",
            self.range_ab,
            ArithRangeTableHelpers::get_range_name(self.range_ab),
            self.range_cd,
            ArithRangeTableHelpers::get_range_name(self.range_cd)
        )
    }
}

impl ArithOperation {
    /// Dumps the chunks of a specified value into the provided formatter.
    fn dump_chunks(&self, f: &mut fmt::Formatter, name: &str, value: &[u64; 4]) -> fmt::Result {
        writeln!(
            f,
            "{0}: [0x{1:X}({1}), 0x{2:X}({2}), 0x{3:X}({3}), 0x{4:X}({4})]",
            name, value[0], value[1], value[2], value[3]
        )
    }

    /// Creates a new instance of `ArithOperation` with default values.
    pub fn new() -> Self {
        Self {
            op: 0,
            input_a: 0,
            input_b: 0,
            a: [0, 0, 0, 0],
            b: [0, 0, 0, 0],
            c: [0, 0, 0, 0],
            d: [0, 0, 0, 0],
            carry: [0, 0, 0, 0, 0, 0, 0],
            m32: false,
            div: false,
            na: false,
            nb: false,
            np: false,
            nr: false,
            sext: false,
            div_by_zero: false,
            div_overflow: false,
            main_mul: false,
            main_div: false,
            signed: false,
            range_ab: 0,
            range_cd: 0,
        }
    }

    /// Performs the arithmetic operation based on the opcode and inputs.
    pub fn calculate(&mut self, op: u8, input_a: u64, input_b: u64) {
        self.op = op;
        self.input_a = input_a;
        self.input_b = input_b;
        self.div_by_zero = input_b == 0
            && (op == ZiskOp::Div.code()
                || op == ZiskOp::Rem.code()
                || op == ZiskOp::DivW.code()
                || op == ZiskOp::RemW.code()
                || op == ZiskOp::Divu.code()
                || op == ZiskOp::Remu.code()
                || op == ZiskOp::DivuW.code()
                || op == ZiskOp::RemuW.code());

        self.div_overflow = ((op == ZiskOp::Div.code() || op == ZiskOp::Rem.code())
            && input_a == 0x8000_0000_0000_0000
            && input_b == 0xFFFF_FFFF_FFFF_FFFF)
            || ((op == ZiskOp::DivW.code() || op == ZiskOp::RemW.code())
                && input_a == 0x8000_0000
                && input_b == 0xFFFF_FFFF);

        let [a, b, c, d] = Self::calculate_abcd_from_ab(op, input_a, input_b);
        self.a = Self::u64_to_chunks(a);
        self.b = Self::u64_to_chunks(b);
        self.c = Self::u64_to_chunks(c);
        self.d = Self::u64_to_chunks(d);
        self.update_flags_and_ranges(op, a, b, c, d);
        let chunks = self.calculate_chunks();
        self.update_carries(&chunks);
    }

    /// Updates carry values based on computed chunks.
    fn update_carries(&mut self, chunks: &[i64; 8]) {
        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_value = chunk + if i > 0 { self.carry[i - 1] } else { 0 };
            if i >= 7 {
                continue;
            }
            self.carry[i] = chunk_value / 0x10000;
        }
    }

    /// Computes the signed representation of a 32-bit value.
    fn sign32(abs_value: u64, negative: bool) -> u64 {
        assert!(0xFFFF_FFFF >= abs_value, "abs_value:0x{abs_value:X}({abs_value}) is too big");
        if negative {
            (0xFFFF_FFFF - abs_value) + 1
        } else {
            abs_value
        }
    }

    /// Computes the signed representation of a 64-bit value.
    fn sign64(abs_value: u64, negative: bool) -> u64 {
        if negative {
            (0xFFFF_FFFF_FFFF_FFFF - abs_value) + 1
        } else {
            abs_value
        }
    }

    /// Computes the signed representation of a 128-bit value.
    fn sign128(abs_value: u128, negative: bool) -> u128 {
        if negative {
            (0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF - abs_value) + 1
        } else {
            abs_value
        }
    }

    /// Extracts the absolute value and sign bit of a 32-bit input.
    fn abs32(value: u64) -> [u64; 2] {
        let negative = if (value & 0x8000_0000) != 0 { 1 } else { 0 };
        let abs_value = if negative == 1 { (0xFFFF_FFFF - value) + 1 } else { value };
        [abs_value, negative]
    }

    /// Extracts the absolute value and sign bit of a 64-bit input.
    fn abs64(value: u64) -> [u64; 2] {
        let negative = if (value & 0x8000_0000_0000_0000) != 0 { 1 } else { 0 };
        let abs_value = if negative == 1 { (0xFFFF_FFFF_FFFF_FFFF - value) + 1 } else { value };
        [abs_value, negative]
    }

    /// Computes the product of the lower 32 bits of two inputs.
    fn calculate_mul_w(a: u64, b: u64) -> u64 {
        (a & 0xFFFF_FFFF) * (b & 0xFFFF_FFFF)
    }

    /// Computes the signed multiplication of a 64-bit and unsigned 64-bit value.
    fn calculate_mulsu(a: u64, b: u64) -> [u64; 2] {
        let [abs_a, na] = Self::abs64(a);
        let abs_c = abs_a as u128 * b as u128;
        let nc = if na == 1 && abs_c != 0 { 1 } else { 0 };
        let c = Self::sign128(abs_c, nc == 1);
        [c as u64, (c >> 64) as u64]
    }

    /// Computes the signed multiplication of two 64-bit values.
    fn calculate_mul(a: u64, b: u64) -> [u64; 2] {
        let [abs_a, na] = Self::abs64(a);
        let [abs_b, nb] = Self::abs64(b);
        let abs_c = abs_a as u128 * abs_b as u128;
        let nc = if na != nb && abs_c != 0 { 1 } else { 0 };
        let c = Self::sign128(abs_c, nc == 1);
        [c as u64, (c >> 64) as u64]
    }

    /// Computes the signed division of two 64-bit values.
    fn calculate_div(a: u64, b: u64) -> u64 {
        let [abs_a, na] = Self::abs64(a);
        let [abs_b, nb] = Self::abs64(b);
        if abs_b == 0 {
            0xFFFF_FFFF_FFFF_FFFF
        } else {
            let abs_c = abs_a / abs_b;
            let nc = if na != nb && abs_c != 0 { 1 } else { 0 };
            Self::sign64(abs_c, nc == 1)
        }
    }

    /// Computes the signed division of two 32-bit values.
    fn calculate_div_w(a: u64, b: u64) -> u64 {
        let [abs_a, na] = Self::abs32(a);
        let [abs_b, nb] = Self::abs32(b);
        if abs_b == 0 {
            0xFFFF_FFFF
        } else {
            let abs_c = abs_a / abs_b;
            let nc = if na != nb && abs_c != 0 { 1 } else { 0 };
            Self::sign32(abs_c, nc == 1)
        }
    }

    /// Computes the unsigned division of two 64-bit values.
    fn calculate_divu(a: u64, b: u64) -> u64 {
        if b == 0 {
            0xFFFF_FFFF_FFFF_FFFF
        } else {
            a / b
        }
    }

    /// Computes the unsigned division of two 32-bit values.
    fn calculate_divu_w(a: u64, b: u64) -> u64 {
        if b == 0 {
            0xFFFF_FFFF
        } else {
            a / b
        }
    }

    /// Computes the unsigned remainder of two 64-bit values.
    fn calculate_remu(a: u64, b: u64) -> u64 {
        if b == 0 {
            a
        } else {
            a % b
        }
    }

    /// Computes the unsigned remainder of two 32-bit values.
    fn calculate_remu_w(a: u64, b: u64) -> u64 {
        if b == 0 {
            a
        } else {
            a % b
        }
    }

    /// Computes the signed remainder of two 64-bit values.
    fn calculate_rem(a: u64, b: u64) -> u64 {
        let [abs_a, na] = Self::abs64(a);
        let [abs_b, _nb] = Self::abs64(b);
        if abs_b == 0 {
            a
        } else {
            let abs_c = abs_a % abs_b;
            let nc = if na == 1 && abs_c != 0 { 1 } else { 0 };
            Self::sign64(abs_c, nc == 1)
        }
    }

    /// Computes the signed remainder of two 32-bit values.
    fn calculate_rem_w(a: u64, b: u64) -> u64 {
        let [abs_a, na] = Self::abs32(a);
        let [abs_b, _nb] = Self::abs32(b);
        if abs_b == 0 {
            a
        } else {
            let abs_c = abs_a % abs_b;
            let nc = if na == 1 && abs_c != 0 { 1 } else { 0 };
            Self::sign32(abs_c, nc == 1)
        }
    }

    /// Calculates intermediate results (A, B, C, D) from inputs (A, B) based on the operation.
    fn calculate_abcd_from_ab(op: u8, a: u64, b: u64) -> [u64; 4] {
        let zisk_op = ZiskOp::try_from_code(op).unwrap();
        match zisk_op {
            ZiskOp::Mulu | ZiskOp::Muluh => {
                let c: u128 = a as u128 * b as u128;
                [a, b, c as u64, (c >> 64) as u64]
            }
            ZiskOp::Mulsuh => {
                let [c, d] = Self::calculate_mulsu(a, b);
                [a, b, c, d]
            }
            ZiskOp::Mul | ZiskOp::Mulh => {
                let [c, d] = Self::calculate_mul(a, b);
                [a, b, c, d]
            }
            ZiskOp::MulW => [a, b, Self::calculate_mul_w(a, b), 0],
            ZiskOp::Divu | ZiskOp::Remu => {
                [Self::calculate_divu(a, b), b, a, Self::calculate_remu(a, b)]
            }
            ZiskOp::DivuW | ZiskOp::RemuW => {
                [Self::calculate_divu_w(a, b), b, a, Self::calculate_remu_w(a, b)]
            }
            ZiskOp::Div | ZiskOp::Rem => {
                [Self::calculate_div(a, b), b, a, Self::calculate_rem(a, b)]
            }
            ZiskOp::DivW | ZiskOp::RemW => {
                [Self::calculate_div_w(a, b), b, a, Self::calculate_rem_w(a, b)]
            }
            _ => {
                panic!("ArithOperation::calculate_abcd_from_ab() Invalid opcode={op}");
            }
        }
    }

    /// Updates the operation's flags and range values based on inputs and results.
    fn update_flags_and_ranges(&mut self, op: u8, a: u64, b: u64, c: u64, d: u64) {
        self.m32 = false;
        self.div = false;
        self.np = false;
        self.nr = false;
        self.sext = false;
        self.main_mul = false;
        self.main_div = false;
        self.signed = false;

        let mut range_a1: u8 = 0;
        let mut range_b1: u8 = 0;
        let mut range_c1: u8 = 0;
        let mut range_d1: u8 = 0;
        let mut range_a3: u8 = 0;
        let mut range_b3: u8 = 0;
        let mut range_c3: u8 = 0;
        let mut range_d3: u8 = 0;

        // direct table opcode(14), signed 2 or 4 cases (0,na,nb,na+nb)
        // 6 * 1 + 7 * 4 + 1 * 2 = 36 entries,
        // no compacted => 16 x 4 = 64, key = (op - 0xb0) * 4 + na * 2 + nb
        // output: div, m32, sa, sb, nr, np, na, na32, nd32, range x 2 x 4

        // alternative: switch operation,

        let mut sa = false;
        let mut sb = false;
        let mut rem = false;

        let zisk_op = ZiskOp::try_from_code(op).unwrap();
        match zisk_op {
            ZiskOp::Mulu => {
                self.main_mul = true;
            }
            ZiskOp::Muluh => {}
            ZiskOp::Mulsuh => {
                sa = true;
            }
            ZiskOp::Mul => {
                sa = true;
                sb = true;
                self.main_mul = true;
            }
            ZiskOp::Mulh => {
                sa = true;
                sb = true;
            }
            ZiskOp::MulW => {
                self.m32 = true;
                self.sext = ((a * b) & 0xFFFF_FFFF) & 0x8000_0000 != 0;
                self.main_mul = true;
            }
            ZiskOp::Divu => {
                self.div = true;
                self.main_div = true;
            }
            ZiskOp::Remu => {
                self.div = true;
                rem = true;
            }
            ZiskOp::Div => {
                sa = true;
                sb = true;
                self.div = true;
                self.main_div = true;
            }
            ZiskOp::Rem => {
                sa = true;
                sb = true;
                rem = true;
                self.div = true;
            }
            ZiskOp::DivuW => {
                // divu_w, remu_w
                self.div = true;
                self.m32 = true;
                // use a in bus
                self.sext = (a & 0x8000_0000) != 0;
                self.main_div = true;
            }
            ZiskOp::RemuW => {
                // divu_w, remu_w
                self.div = true;
                self.m32 = true;
                rem = true;
                // use d in bus
                self.sext = (d & 0x8000_0000) != 0;
            }
            ZiskOp::DivW => {
                // div_w, rem_w
                sa = true;
                sb = true;
                self.div = true;
                self.m32 = true;
                // use a in bus
                self.sext = (a & 0x8000_0000) != 0;
                self.main_div = true;
            }
            ZiskOp::RemW => {
                // div_w, rem_w
                sa = true;
                sb = true;
                self.div = true;
                self.m32 = true;
                rem = true;
                // use d in bus
                self.sext = (d & 0x8000_0000) != 0;
            }
            _ => {
                panic!("ArithOperation::update_flags_and_ranges() Invalid opcode={op}");
            }
        }
        self.signed = sa || sb;

        let sign_mask: u64 = if self.m32 { 0x8000_0000 } else { 0x8000_0000_0000_0000 };
        let sign_c_mask: u64 =
            if self.m32 && self.div { 0x8000_0000 } else { 0x8000_0000_0000_0000 };
        self.na = sa && (a & sign_mask) != 0;
        self.nb = sb && (b & sign_mask) != 0;
        // a sign => b sign
        let nc = sa && (c & sign_c_mask) != 0;
        let nd = sa && (d & sign_mask) != 0;

        // a == 0 || b == 0 => np == 0 ==> how was a signed operation
        // after that sign of np was verified with range check.
        // TODO: review if secure
        if self.div {
            self.np = nc; //if c != 0 { na ^ nb } else { 0 };
            self.nr = nd;
        } else {
            self.np = if self.m32 { nc } else { nd }; // if (c != 0) || (d != 0) { na ^ nb } else { 0 }
            self.nr = false;
        }
        if self.m32 {
            // mulw, divu_w, remu_w, div_w, rem_w
            range_a1 = if sa {
                if self.na {
                    2
                } else {
                    1
                }
            } else if self.div && !rem {
                if self.sext {
                    2
                } else {
                    1
                }
            } else {
                0
            };
            range_b1 = if sb {
                if self.nb {
                    2
                } else {
                    1
                }
            } else {
                0
            };
            // m32 && div == 0 => mulw
            range_c1 = if !self.div {
                if self.sext {
                    2
                } else {
                    1
                }
            } else if sa {
                if self.np {
                    2
                } else {
                    1
                }
            } else {
                0
            };
            range_d1 = if rem {
                if self.sext {
                    2
                } else {
                    1
                }
            } else if sa {
                if self.nr {
                    2
                } else {
                    1
                }
            } else {
                0
            };
        } else {
            // mulu, muluh, mulsuh, mul, mulh, div, rem, divu, remu
            if sa {
                // mulsuh, mul, mulh, div, rem
                range_a3 = if self.na { 2 } else { 1 };
                if self.div {
                    // div, rem
                    range_c3 = if self.np { 2 } else { 1 };
                    range_d3 = if self.nr { 2 } else { 1 }
                } else {
                    range_d3 = if self.np { 2 } else { 1 }
                }
            }
            // sb => mul, mulh, div, rem
            range_b3 = if sb {
                if self.nb {
                    2
                } else {
                    1
                }
            } else {
                0
            };
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

        assert!(range_a1 == 0 || range_a3 == 0, "range_a1:{range_a1} range_a3:{range_a3}");
        assert!(range_b1 == 0 || range_b3 == 0, "range_b1:{range_b1} range_b3:{range_b3}");
        assert!(range_c1 == 0 || range_c3 == 0, "range_c1:{range_c1} range_c3:{range_c3}");
        assert!(range_d1 == 0 || range_d3 == 0, "range_d1:{range_d1} range_d3:{range_d3}");

        self.range_ab = (range_a3 + range_a1) * 3
            + range_b3
            + range_b1
            + if (range_a1 + range_b1) > 0 { 8 } else { 0 };

        self.range_cd = (range_c3 + range_c1) * 3
            + range_d3
            + range_d1
            + if (range_c1 + range_d1) > 0 { 8 } else { 0 };
    }

    /// Calculates the operation's output chunks for further processing.
    pub fn calculate_chunks(&self) -> [i64; 8] {
        // TODO: unroll this function in variants (div,m32) and (na,nb,nr,np)
        // div, m32, na, nb === f(div,m32,na,nb) => fa, nb, nr
        // unroll means 16 variants ==> but more performance

        let mut chunks: [i64; 8] = [0, 0, 0, 0, 0, 0, 0, 0];

        let fab = if self.na != self.nb { -1 } else { 1 };

        let a = [self.a[0] as i64, self.a[1] as i64, self.a[2] as i64, self.a[3] as i64];
        let b = [self.b[0] as i64, self.b[1] as i64, self.b[2] as i64, self.b[3] as i64];
        let c = [self.c[0] as i64, self.c[1] as i64, self.c[2] as i64, self.c[3] as i64];
        let d = [self.d[0] as i64, self.d[1] as i64, self.d[2] as i64, self.d[3] as i64];

        let na = self.na as i64;
        let nb = self.nb as i64;
        let np = self.np as i64;
        let nr = self.nr as i64;
        let m32 = self.m32 as i64;
        let div = self.div as i64;

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
            - np * (1 - m32) * div  // 2^64 (np)
            + nr * (1 - m32)  // 2^64 (nr)
            // high part d
            - d[0] * (1 - div)          // m32 == 1 and div == 0 => d = 0
            + 2 * np * d[0] * (1 - div); //

        chunks[5] = fab * a[3] * b[2]    // chunk5
            + fab * a[2] * b[3]
            + a[1] * nb_fa * (1 - m32)
            + b[1] * na_fb * (1 - m32)
            - d[1] * (1 - div)
            + d[1] * 2 * np * (1 - div);

        chunks[6] = fab * a[3] * b[3]    // chunk6
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

    /// Converts a 64-bit value into its four 16-bit chunks.
    fn u64_to_chunks(a: u64) -> [u64; 4] {
        [a & 0xFFFF, (a >> 16) & 0xFFFF, (a >> 32) & 0xFFFF, (a >> 48) & 0xFFFF]
    }
}
