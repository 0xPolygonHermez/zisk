//! Diagnostic coverage for the ZisK `Arith` state machine.
//!
//! Every RISC-V M-extension instruction that lands on `Arith` is run over **all ordered pairs** of a
//! set of representative values, and the result is compared against the pure-Rust reference in
//! `ops_core`. The point is to hit every corner the Arith AIR distinguishes: the sign flags
//! (`na`, `nb`, `np`, `nr`), the sign-extension flag (`sext`), `div_by_zero`, `div_overflow`, and the
//! `result_is_zero` / `remainder_is_zero` flags.
//!
//! RISC-V mnemonic -> ZisK op:
//!
//! ```text
//!   mul    -> Mul    (0xb4)      div    -> Div    (0xba)      divw   -> DivW   (0xbe)
//!   mulh   -> Mulh   (0xb5)      divu   -> Divu   (0xb8)      divuw  -> DivuW  (0xbc)
//!   mulhsu -> Mulsuh (0xb3)      rem    -> Rem    (0xbb)      remw   -> RemW   (0xbf)
//!   mulhu  -> Muluh  (0xb1)      remu   -> Remu   (0xb9)      remuw  -> RemuW  (0xbd)
//!   mulw   -> MulW   (0xb6)
//! ```
//!
//! `Mulu` (0xb0) is deliberately absent: no RISC-V mnemonic transpiles to it, so a guest program
//! cannot emit it. The other 13 Arith ops are all covered here.

// NOTE: like the other diagnostic programs, this is built for the guest target and does not depend
// on `zisk_core`; the reference implementations are included directly.
#[path = "../../../../core/src/ops_core.rs"]
mod ops_core;

use ops_core::*;

// Representative 64-bit values: 0, 1, 2, MAX/2 +- {0,1,2}, MAX - {0,1,2}, and the two's complement
// negation of each. MAX = u64::MAX, so MAX/2 = i64::MAX and MAX/2 + 1 = i64::MIN.
//
// The set is closed under negation (-1 = MAX, -2 = MAX-1, -3 = MAX-2, and i64::MIN negates to
// itself), which is what makes the signed and unsigned views of every op reachable from it.
const VALUES_64: [u64; 14] = [
    0x0000_0000_0000_0000, // 0
    0x0000_0000_0000_0001, // 1
    0x0000_0000_0000_0002, // 2
    0x0000_0000_0000_0003, // 3            = -(MAX - 2)
    0x7FFF_FFFF_FFFF_FFFD, // MAX/2 - 2    = -(MAX/2 + 4)
    0x7FFF_FFFF_FFFF_FFFE, // MAX/2 - 1
    0x7FFF_FFFF_FFFF_FFFF, // MAX/2        = i64::MAX
    0x8000_0000_0000_0000, // MAX/2 + 1    = i64::MIN, negates to itself
    0x8000_0000_0000_0001, // MAX/2 + 2    = -i64::MAX
    0x8000_0000_0000_0002, // MAX/2 + 3    = -(MAX/2 - 1)
    0x8000_0000_0000_0003, // MAX/2 + 4    = -(MAX/2 - 2)
    0xFFFF_FFFF_FFFF_FFFD, // MAX - 2      = -3
    0xFFFF_FFFF_FFFF_FFFE, // MAX - 1      = -2
    0xFFFF_FFFF_FFFF_FFFF, // MAX          = -1
];

// The same pattern for the 32-bit (_W) instructions, with MAX = u32::MAX. These operate on the low
// 32 bits of the registers, so they need their own edge cases: in particular i32::MIN / -1, the
// 32-bit division overflow.
const VALUES_32: [u32; 14] = [
    0x0000_0000, // 0
    0x0000_0001, // 1
    0x0000_0002, // 2
    0x0000_0003, // 3          = -(MAX - 2)
    0x7FFF_FFFD, // MAX/2 - 2
    0x7FFF_FFFE, // MAX/2 - 1
    0x7FFF_FFFF, // MAX/2      = i32::MAX
    0x8000_0000, // MAX/2 + 1  = i32::MIN, negates to itself
    0x8000_0001, // MAX/2 + 2  = -i32::MAX
    0x8000_0002, // MAX/2 + 3
    0x8000_0003, // MAX/2 + 4
    0xFFFF_FFFD, // MAX - 2    = -3
    0xFFFF_FFFE, // MAX - 1    = -2
    0xFFFF_FFFF, // MAX        = -1
];

pub fn diagnostic_zisk_arith() {
    println!("diagnostic_zisk_arith() start");

    // 64-bit operations
    for a in VALUES_64 {
        for b in VALUES_64 {
            mul(a, b, op_mul(a, b).0);
            mulh(a, b, op_mulh(a, b).0);
            mulhsu(a, b, op_mulsuh(a, b).0);
            mulhu(a, b, op_muluh(a, b).0);

            // b = 0 covers div_by_zero; (i64::MIN, -1) covers div_overflow
            div(a, b, op_div(a, b).0);
            divu(a, b, op_divu(a, b).0);
            rem(a, b, op_rem(a, b).0);
            remu(a, b, op_remu(a, b).0);
        }
    }

    // 32-bit (_W) operations
    for a in VALUES_32 {
        for b in VALUES_32 {
            let (a64, b64) = (a as u64, b as u64);

            mulw(a, b, op_mul_w(a64, b64).0 as u32);

            // b = 0 covers div_by_zero; (i32::MIN, -1) covers the 32-bit div_overflow
            divw(a, b, op_div_w(a64, b64).0 as u32);
            divuw(a, b, op_divu_w(a64, b64).0 as u32);
            remw(a, b, op_rem_w(a64, b64).0 as u32);
            remuw(a, b, op_remu_w(a64, b64).0 as u32);
        }
    }

    println!("diagnostic_zisk_arith() success");
}

/// Asserts on a 64-bit result, reporting the inputs so a failure inside the loops is diagnosable.
fn check64(name: &str, a: u64, b: u64, got: u64, expected: u64) {
    assert_eq!(
        got, expected,
        "{name}: a={a:#018x} b={b:#018x} -> {got:#018x}, expected {expected:#018x}"
    );
}

/// Asserts on a 32-bit result. `got` is the low half of the destination register; the _W
/// instructions sign-extend into the upper half, which `check32_sext` covers separately.
fn check32(name: &str, a: u32, b: u32, got: u32, expected: u32) {
    assert_eq!(
        got, expected,
        "{name}: a={a:#010x} b={b:#010x} -> {got:#010x}, expected {expected:#010x}"
    );
}

/// The _W instructions write a sign-extended 32-bit result, so the whole 64-bit register must equal
/// the sign extension of the expected low half. This is what exercises the `sext` flag of the AIR.
fn check32_sext(name: &str, a: u32, b: u32, got: u64, expected: u32) {
    let expected64 = expected as i32 as i64 as u64;
    assert_eq!(
        got, expected64,
        "{name} (sign extension): a={a:#010x} b={b:#010x} -> {got:#018x}, expected {expected64:#018x}"
    );
}

/*******/
/* mul */
/*******/

// mul (RISCV) -> Mul (ZisK)
fn mul(input_a: u64, input_b: u64, expected_c: u64) {
    let c: u64;
    unsafe {
        std::arch::asm!(
            "mul {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) input_a,
            input2 = in(reg) input_b,
        );
    }
    check64("mul", input_a, input_b, c, expected_c);
}

// mulh (RISCV) -> Mulh (ZisK)
fn mulh(input_a: u64, input_b: u64, expected_c: u64) {
    let c: u64;
    unsafe {
        std::arch::asm!(
            "mulh {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) input_a,
            input2 = in(reg) input_b,
        );
    }
    check64("mulh", input_a, input_b, c, expected_c);
}

// mulhsu (RISCV) -> Mulsuh (ZisK): a is signed, b is unsigned
fn mulhsu(input_a: u64, input_b: u64, expected_c: u64) {
    let c: u64;
    unsafe {
        std::arch::asm!(
            "mulhsu {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) input_a,
            input2 = in(reg) input_b,
        );
    }
    check64("mulhsu", input_a, input_b, c, expected_c);
}

// mulhu (RISCV) -> Muluh (ZisK)
fn mulhu(input_a: u64, input_b: u64, expected_c: u64) {
    let c: u64;
    unsafe {
        std::arch::asm!(
            "mulhu {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) input_a,
            input2 = in(reg) input_b,
        );
    }
    check64("mulhu", input_a, input_b, c, expected_c);
}

// mulw (RISCV) -> MulW (ZisK)
fn mulw(input_a: u32, input_b: u32, expected_c: u32) {
    let c: u64;
    unsafe {
        std::arch::asm!(
            "mulw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) input_a,
            input2 = in(reg) input_b,
        );
    }
    check32("mulw", input_a, input_b, c as u32, expected_c);
    check32_sext("mulw", input_a, input_b, c, expected_c);
}

/*******/
/* div */
/*******/

// div (RISCV) -> Div (ZisK)
fn div(input_a: u64, input_b: u64, expected_c: u64) {
    let c: u64;
    unsafe {
        std::arch::asm!(
            "div {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) input_a,
            input2 = in(reg) input_b,
        );
    }
    check64("div", input_a, input_b, c, expected_c);
}

// divu (RISCV) -> Divu (ZisK)
fn divu(input_a: u64, input_b: u64, expected_c: u64) {
    let c: u64;
    unsafe {
        std::arch::asm!(
            "divu {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) input_a,
            input2 = in(reg) input_b,
        );
    }
    check64("divu", input_a, input_b, c, expected_c);
}

// divw (RISCV) -> DivW (ZisK)
fn divw(input_a: u32, input_b: u32, expected_c: u32) {
    let c: u64;
    unsafe {
        std::arch::asm!(
            "divw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) input_a,
            input2 = in(reg) input_b,
        );
    }
    check32("divw", input_a, input_b, c as u32, expected_c);
    check32_sext("divw", input_a, input_b, c, expected_c);
}

// divuw (RISCV) -> DivuW (ZisK)
fn divuw(input_a: u32, input_b: u32, expected_c: u32) {
    let c: u64;
    unsafe {
        std::arch::asm!(
            "divuw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) input_a,
            input2 = in(reg) input_b,
        );
    }
    check32("divuw", input_a, input_b, c as u32, expected_c);
    check32_sext("divuw", input_a, input_b, c, expected_c);
}

/*******/
/* rem */
/*******/

// rem (RISCV) -> Rem (ZisK)
fn rem(input_a: u64, input_b: u64, expected_c: u64) {
    let c: u64;
    unsafe {
        std::arch::asm!(
            "rem {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) input_a,
            input2 = in(reg) input_b,
        );
    }
    check64("rem", input_a, input_b, c, expected_c);
}

// remu (RISCV) -> Remu (ZisK)
fn remu(input_a: u64, input_b: u64, expected_c: u64) {
    let c: u64;
    unsafe {
        std::arch::asm!(
            "remu {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) input_a,
            input2 = in(reg) input_b,
        );
    }
    check64("remu", input_a, input_b, c, expected_c);
}

// remw (RISCV) -> RemW (ZisK)
fn remw(input_a: u32, input_b: u32, expected_c: u32) {
    let c: u64;
    unsafe {
        std::arch::asm!(
            "remw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) input_a,
            input2 = in(reg) input_b,
        );
    }
    check32("remw", input_a, input_b, c as u32, expected_c);
    check32_sext("remw", input_a, input_b, c, expected_c);
}

// remuw (RISCV) -> RemuW (ZisK)
fn remuw(input_a: u32, input_b: u32, expected_c: u32) {
    let c: u64;
    unsafe {
        std::arch::asm!(
            "remuw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) input_a,
            input2 = in(reg) input_b,
        );
    }
    check32("remuw", input_a, input_b, c as u32, expected_c);
    check32_sext("remuw", input_a, input_b, c, expected_c);
}
