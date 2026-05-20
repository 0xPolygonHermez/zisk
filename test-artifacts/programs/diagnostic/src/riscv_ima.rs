#![cfg(all(target_os = "zkvm", target_vendor = "zisk"))]

use std::arch::asm;
use std::num::Wrapping;

pub fn diagnostic_riscv_ima() {
    // minu belongs to Zbb extension, not IMA

    // {
    //     let a: u64 = 0xFFFF_FFFF_FFFF_FFFF;
    //     let b: u64 = 0xFFFF_FFFF_FFFF_FFFE;
    //     let c: u64;

    //     // Use inline assembly to ensure minu instruction is called
    //     unsafe {
    //         std::arch::asm!(
    //             "minu {result}, {input1}, {input2}",
    //             result = out(reg) c,
    //             input1 = in(reg) a,
    //             input2 = in(reg) b,
    //         );
    //     }

    //     assert_eq!(c, 0xFFFF_FFFF_FFFF_FFFE);
    // }

    diagnostic_riscv_ima_branch();

    or(0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF);
    or(0xFFFF_FFFF_FFFF_FFF1, 0xFFFF_FFFF_FFFF_FFFE, 0xFFFF_FFFF_FFFF_FFFF);
    or(0xFFFF_0000_FFFF_0000, 0xFFFF_0000_0000_0000, 0xFFFF_0000_FFFF_0000);
    or(0x0000_0000_0000_0000, 0xFFFF_0000_0000_0000, 0xFFFF_0000_0000_0000);
    or(0x0000_0000_0000_0000, 0x0000_0000_0000_0000, 0x0000_0000_0000_0000); // FROP

    xor(0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF, 0x0000_0000_0000_0000);
    xor(0xFFFF_0000_FFFF_0000, 0xFFFF_FFFF_0000_0000, 0x0000_FFFF_FFFF_0000);
    xor(0x0000_0000_0000_0000, 0xFFFF_FFFF_0000_0000, 0xFFFF_FFFF_0000_0000);
    xor(0x0000_0000_0000_0000, 0x0000_0000_0000_0000, 0x0000_0000_0000_0000); // FROP

    and(0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF, 0xFFFF_FFFF_FFFF_FFFF);
    and(0xFFFF_0000_FFFF_0000, 0xFFFF_FFFF_0000_0000, 0xFFFF_0000_0000_0000);
    and(0x0000_0000_0000_0000, 0xFFFF_FFFF_0000_0000, 0x0000_0000_0000_0000);
    and(0x0000_0000_0000_0000, 0x0000_0000_0000_0000, 0x0000_0000_0000_0000); // FROP

    div(0xFFFF_FFFF_FFFF, 0x1_0000_0000, 0xFFFF);
    divu(0xFFFF_FFFF_FFFF_FFFF, 0x1_0000_0000, 0xFFFF_FFFF);
    div_w(0xFF_FFFF, 0x1_0000, 0xFF);
    divu_w(0xFF_FFFF, 0x1_0000, 0xFF);

    rem(0xFFFF_0000_FFFF, 0x1_0000_0000, 0xFFFF);
    remu(0xFFFF_0000_FFFF, 0x1_0000_0000, 0xFFFF);
    rem_w(0xFF_00FF, 0x1_0000, 0xFF);
    remu_w(0xFF_00FF, 0x1_0000, 0xFF); // TODO: This one seems not to work

    mul(0xFFFF_FFFF, 0x1_0000, 0xFFFF_FFFF_0000);
    mulh(0xFFFF_FFFF, 0x1_0000, 0x0);
    mulh(0xFFFF_FFFF_0000, 0x1_0000_0000, 0xFFFF);
    muluh(0xFFFF_FFFF_0000_0000, 0x1_0000_0000, 0xFFFF_FFFF);
    mulsuh(0xFFFF_FFFF_FFFF_FFFFu64 as i64, 0x1, 0xFFFF_FFFF_FFFF_FFFFu64 as i64);
    mul_w(0xFFFF, 0x100, 0xFF_FF00);

    sll_w(0x1_0000, 2, 0x4_0000);
    srl_w(0x4000_0000, 2, 0x1000_0000);
    srl_w(0x8000_0000, 0, 0xFFFF_FFFF_8000_0000);
    sra_w(0x8000_0000, 0, 0xFFFF_FFFF_8000_0000);

    add_w(0, 0, 0);
    add_w(1, 2, 3);
    add_w(2, 2, 4);
    add_w(0xFFFF, 0x1, 0x1_0000);
    add_w(0xFFFF_FFFF, 0x1, 0);

    sub_w(0, 0, 0);
    sub_w(3, 2, 1);
    sub_w(0x1_0000, 1, 0xFFFF);
    sub_w(0x1_0000, 0x1, 0xFFFF);
    sub_w(0, 0x1, 0xFFFF_FFFF_FFFF_FFFF);

    amomax_d(0x0000_0001, 0x0000_0002, 0x0000_0002);
    amomin_d(0x0000_0001, 0x0000_0002, 0x0000_0001);
    amomaxu_d(0x0000_0001, 0x0000_0002, 0x0000_0002);
    amominu_d(0x0000_0001, 0x0000_0002, 0x0000_0001);

    amomax_w(0x0000_0001, 0x0000_0002, 0x0000_0002);
    amomin_w(0x0000_0001, 0x0000_0002, 0x0000_0001);
    amomaxu_w(0x0000_0001, 0x0000_0002, 0x0000_0002);
    amominu_w(0x0000_0001, 0x0000_0002, 0x0000_0001);

    amoand_d(0x0000_0001, 0x0000_0002, 0x0000_0000);
    amoor_d(0x0000_0001, 0x0000_0002, 0x0000_0003);
    amoxor_d(0x1000_0001, 0x1000_0002, 0x0000_0003);

    amoand_w(0x0000_0000, 0x0000_0000, 0x0000_0000);
    amoand_w(0x0000_0001, 0x0000_0002, 0x0000_0000);
    amoand_w(0x0000_FFFF, 0x0000_FF00, 0x0000_FF00);
    amoand_w(0xFF00_FF00, 0x0000_FFFF, 0x0000_FF00);
    amoand_w(0xFFFF_FFFF, 0xFFFF_FFFF, 0xFFFF_FFFF);

    amoor_w(0x0000_0000, 0x0000_0000, 0x0000_0000);
    amoor_w(0x0000_0001, 0x0000_0002, 0x0000_0003);
    amoor_w(0x0000_FF00, 0x00FF_0000, 0x00FF_FF00);
    amoor_w(0xFFFF_FFFF, 0xFFFF_FFFF, 0xFFFF_FFFF);

    amoxor_w(0x0000_0000, 0x0000_0000, 0x0000_0000);
    amoxor_w(0x1000_0001, 0x1000_0002, 0x0000_0003);
    amoxor_w(0xFFFF_0000, 0xFF00_FF00, 0x00FF_FF00);
    amoxor_w(0xFFFF_FFFF, 0xFFFF_FFFF, 0x0000_0000);

    amoadd_d(0, 0, 0);
    amoadd_d(0, 1, 1);
    amoadd_d(1, 2, 3);
    amoadd_d(2, 2, 4);
    amoadd_d(0xFFFF_FFFF_FFFF_0000, 0xFFFF, 0xFFFF_FFFF_FFFF_FFFF);

    amoadd_w(0, 0, 0);
    amoadd_w(0, 1, 1);
    amoadd_w(1, 2, 3);
    amoadd_w(2, 2, 4);
    amoadd_w(0xFFFF_0000, 0xFFFF, 0xFFFF_FFFF);

    amoswap_d(1, 2, 1);
    amoswap_d(0, 0xFFFF_FFFF_FFFF_FFFF, 0);
    amoswap_d(0xFFFF_FFFF_FFFF_FFFF, 0, 0xFFFF_FFFF_FFFF_FFFF);

    amoswap_w(1, 2, 1);
    amoswap_w(0, 0xFFFF_FFFF, 0);
    amoswap_w(0xFFFF_FFFF, 0, 0xFFFF_FFFF);

    signextend_b(127, 127);
    signextend_b(1, 1);
    signextend_b(0, 0);
    signextend_b(-1, -1);
    signextend_b(-128, -128);

    signextend_h(32767, 32767);
    signextend_h(1, 1);
    signextend_h(0, 0);
    signextend_h(-1, -1);
    signextend_h(-32768, -32768);

    signextend_w(2147483647, 2147483647);
    signextend_w(1, 1);
    signextend_w(0, 0);
    signextend_w(-1, -1);
    signextend_w(-2147483648, -2147483648);

    // TODO: not mapped from RISCV to ZisK
    // leu, le, ltu_w, lt_w, eq_w, leu_w, le_w, mulu

    // TODO: they require Zbb extension
    // minu, min, maxu, max,

    riscv_xori();
    riscv_ori();
    riscv_fence();
    riscv_fence_i();
    riscv_ebreak();
    riscv_lr_d();
    riscv_lr_w();
    riscv_sc_d();
    riscv_sc_w();
    riscv_sll(1, 2, 4);
    riscv_srl(4, 2, 1);
    riscv_sra(4, 2, 1);
    riscv_slli();
    riscv_slliw();
    riscv_sraiw();
    riscv_srliw();
    riscv_slti();
    riscv_slt(2, 3, 1);
    riscv_csrrw();
    riscv_csrrwi();
    riscv_csrrs();
    riscv_csrrsi();
    riscv_csrrc();
    riscv_csrrci();

    println!("diagnostic_riscv_ima() success");
}

#[allow(dead_code)]
pub fn diagnostic_riscv_ima_combinations() {
    let values: [u8; 7] = [0, 1, 0x7F, 0x80, 0x81, 0xFE, 0xFF];
    for a_byte_0 in values {
        for a_byte_3 in values {
            for a_byte_4 in values {
                for a_byte_7 in values {
                    let a: u64 = (a_byte_0 as u64) << 0
                        | (a_byte_3 as u64) << 24
                        | (a_byte_4 as u64) << 32
                        | (a_byte_7 as u64) << 56;
                    for b_byte_0 in values {
                        for b_byte_3 in values {
                            for b_byte_4 in values {
                                for b_byte_7 in values {
                                    let b: u64 = (b_byte_0 as u64) << 0
                                        | (b_byte_3 as u64) << 24
                                        | (b_byte_4 as u64) << 32
                                        | (b_byte_7 as u64) << 56;
                                    and_no_check(a, b);
                                    // or_no_check(a, b);
                                    // xor_no_check(a, b);
                                    // add_no_check(a, b);
                                    // add_w_no_check(a, b);
                                    // sub_no_check(a, b);
                                    // sub_w_no_check(a, b);
                                    // sll_no_check(a, b);
                                    // sll_w_no_check(a, b);
                                    // sra_no_check(a, b);
                                    // sra_w_no_check(a, b);
                                    // srl_no_check(a, b);
                                    // srl_w_no_check(a, b);

                                    // eq_no_check(a, b);
                                    // ltu_no_check(a, b);
                                    // lt_no_check(a, b);

                                    // minu_no_check(a, b);
                                    // min_no_check(a, b);
                                    // minu_w_no_check(a as u32, b as u32);
                                    // min_w_no_check(a as u32, b as u32);
                                    // maxu_no_check(a, b);
                                    // max_no_check(a, b);
                                    // maxu_w_no_check(a as u32, b as u32);
                                    // max_w_no_check(a as u32, b as u32);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
/******************/
/* or / xor / and */
/******************/

// or (RISCV) -> or (ZisK)
fn or(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "or {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

#[allow(dead_code)]
fn or_no_check(a: u64, b: u64) {
    let _c: u64;
    unsafe {
        std::arch::asm!(
            "or {result}, {input1}, {input2}",
            result = out(reg) _c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }
}

// xor (RISCV) -> xor (ZisK)
fn xor(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "xor {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

#[allow(dead_code)]
fn xor_no_check(a: u64, b: u64) {
    let _c: u64;
    unsafe {
        std::arch::asm!(
            "xor {result}, {input1}, {input2}",
            result = out(reg) _c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }
}

// and (RISCV) -> and (ZisK)
fn and(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "and {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

#[allow(dead_code)]
fn and_no_check(a: u64, b: u64) {
    let _c: u64;
    unsafe {
        std::arch::asm!(
            "and {result}, {input1}, {input2}",
            result = out(reg) _c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }
}

/*******/
/* div */
/*******/

// div (RISCV) -> div (ZisK)
fn div(input_a: i64, input_b: i64, expected_c: i64) {
    let a: i64 = input_a;
    let b: i64 = input_b;
    let c: i64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "div {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

// divu (RISCV) -> divu (ZisK)
fn divu(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "divu {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

// divw (RISCV) -> div_w (ZisK)
fn div_w(input_a: i32, input_b: i32, expected_c: i32) {
    let a: i32 = input_a;
    let b: i32 = input_b;
    let c: i32;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "divw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

// divuw (RISCV) -> divu_w (ZisK)
fn divu_w(input_a: u32, input_b: u32, expected_c: u32) {
    let a: u32 = input_a;
    let b: u32 = input_b;
    let c: u32;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "divuw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

/*******/
/* rem */
/*******/

// rem (RISCV) -> rem (ZisK)
fn rem(input_a: i64, input_b: i64, expected_c: i64) {
    let a: i64 = input_a;
    let b: i64 = input_b;
    let c: i64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "rem {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

// remu (RISCV) -> remu (ZisK)
fn remu(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "remu {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

// remw (RISCV) -> rem_w (ZisK)
fn rem_w(input_a: i32, input_b: i32, expected_c: i32) {
    let a: i32 = input_a;
    let b: i32 = input_b;
    let c: i32;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "remw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

// remu_w (RISCV) -> remu_w (ZisK)
fn remu_w(input_a: u32, input_b: u32, expected_c: u32) {
    let a: u32 = input_a;
    let b: u32 = input_b;
    let c: u32;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "remuw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

/*******/
/* mul */
/*******/

// mul (RISCV) -> mul (ZisK)
fn mul(input_a: i64, input_b: i64, expected_c: i64) {
    let a: i64 = input_a;
    let b: i64 = input_b;
    let c: i64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "mul {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

// mulh (RISCV) -> mulh (ZisK)
fn mulh(input_a: i64, input_b: i64, expected_c: i64) {
    let a: i64 = input_a;
    let b: i64 = input_b;
    let c: i64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "mulh {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

// mulhu (RISCV) -> muluh (ZisK)
fn muluh(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "mulhu {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

// mulhsu (RISCV) -> mulsuh (ZisK)
fn mulsuh(input_a: i64, input_b: u64, expected_c: i64) {
    let a: i64 = input_a;
    let b: u64 = input_b;
    let c: i64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "mulhsu {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

// mulw (RISCV) -> mul_w (ZisK)
fn mul_w(input_a: i32, input_b: i32, expected_c: i32) {
    let a: i32 = input_a;
    let b: i32 = input_b;
    let c: i32;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "mulw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

/*********/
/* shift */
/*********/

// sllw (RISCV) -> sll_w (ZisK)
fn sll_w(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "sllw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

#[allow(dead_code)]
fn sll_w_no_check(a: u64, b: u64) {
    let _c: u64;
    unsafe {
        std::arch::asm!(
            "sllw {result}, {input1}, {input2}",
            result = out(reg) _c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }
}

// srlw (RISCV) -> srl_w (ZisK)
fn srl_w(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "srlw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

#[allow(dead_code)]
fn srl_w_no_check(a: u64, b: u64) {
    let _c: u64;
    unsafe {
        std::arch::asm!(
            "srlw {result}, {input1}, {input2}",
            result = out(reg) _c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }
}

// sraw (RISCV) -> sra_w (ZisK)
fn sra_w(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "sraw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

#[allow(dead_code)]
fn sra_w_no_check(a: u64, b: u64) {
    let _c: u64;
    unsafe {
        std::arch::asm!(
            "sraw {result}, {input1}, {input2}",
            result = out(reg) _c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }
}

/*************/
/* add / sub */
/*************/

#[allow(dead_code)]
fn add_no_check(a: u64, b: u64) {
    let _c: u64;
    unsafe {
        std::arch::asm!(
            "add {result}, {input1}, {input2}",
            result = out(reg) _c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }
}

#[allow(dead_code)]
fn add_w_no_check(a: u64, b: u64) {
    let _c = (Wrapping(a as i32) + Wrapping(b as i32)).0 as u64;
}

#[allow(dead_code)]
fn sub_no_check(a: u64, b: u64) {
    let _c: u64;
    unsafe {
        std::arch::asm!(
            "sub {result}, {input1}, {input2}",
            result = out(reg) _c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }
}

#[allow(dead_code)]
fn sub_w_no_check(a: u64, b: u64) {
    let _c = (Wrapping(a as i32) - Wrapping(b as i32)).0 as u64;
}

// subw (RISCV) -> sub_w (ZisK)
fn sub_w(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "subw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}
// addw (RISCV) -> add_w (ZisK)
fn add_w(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "addw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

/*******************/
/* amomin / amomax */
/*******************/

fn amomax_d(input_a: i64, input_b: i64, expected_c: i64) {
    let a: i64 = input_a;
    let b: i64 = input_b;
    let c: i64;
    unsafe {
        asm!(
            "amomax.d {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amomin_d(input_a: i64, input_b: i64, expected_c: i64) {
    let a: i64 = input_a;
    let b: i64 = input_b;
    let c: i64;
    unsafe {
        asm!(
            "amomin.d {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amomaxu_d(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;
    unsafe {
        asm!(
            "amomaxu.d {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amominu_d(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;
    unsafe {
        asm!(
            "amominu.d {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amomax_w(input_a: i32, input_b: i32, expected_c: i32) {
    let a: i32 = input_a;
    let b: i32 = input_b;
    let c: i32;
    unsafe {
        asm!(
            "amomax.w {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amomin_w(input_a: i32, input_b: i32, expected_c: i32) {
    let a: i32 = input_a;
    let b: i32 = input_b;
    let c: i32;
    unsafe {
        asm!(
            "amomin.w {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amomaxu_w(input_a: u32, input_b: u32, expected_c: u32) {
    let a: u32 = input_a;
    let b: u32 = input_b;
    let c: u32;
    unsafe {
        asm!(
            "amomaxu.w {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amominu_w(input_a: u32, input_b: u32, expected_c: u32) {
    let a: u32 = input_a;
    let b: u32 = input_b;
    let c: u32;
    unsafe {
        asm!(
            "amominu.w {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

/***************************/
/* amoand / amoor / amoxor */
/***************************/

fn amoand_d(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;
    unsafe {
        asm!(
            "amoand.d {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amoor_d(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;
    unsafe {
        asm!(
            "amoor.d {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amoxor_d(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;
    unsafe {
        asm!(
            "amoxor.d {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amoand_w(input_a: u32, input_b: u32, expected_c: u32) {
    let a: u32 = input_a;
    let b: u32 = input_b;
    let c: u32;
    unsafe {
        asm!(
            "amoand.w {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amoor_w(input_a: u32, input_b: u32, expected_c: u32) {
    let a: u32 = input_a;
    let b: u32 = input_b;
    let c: u32;
    unsafe {
        asm!(
            "amoor.w {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amoxor_w(input_a: u32, input_b: u32, expected_c: u32) {
    let a: u32 = input_a;
    let b: u32 = input_b;
    let c: u32;
    unsafe {
        asm!(
            "amoxor.w {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

/**********/
/* amoadd */
/**********/

fn amoadd_d(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;
    unsafe {
        asm!(
            "amoadd.d {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amoadd_w(input_a: u32, input_b: u32, expected_c: u32) {
    let a: u32 = input_a;
    let b: u32 = input_b;
    let c: u32;
    unsafe {
        asm!(
            "amoadd.w {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

/***********/
/* amoswap */
/***********/

fn amoswap_d(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;
    unsafe {
        asm!(
            "amoswap.d {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

fn amoswap_w(input_a: u32, input_b: u32, expected_c: u32) {
    let a: u32 = input_a;
    let b: u32 = input_b;
    let c: u32;
    unsafe {
        asm!(
            "amoswap.w {result}, {value}, ({ptr})",
            result = out(reg) c,
            value = in(reg) a,
            ptr = in(reg) &b,
        );
    }
    assert_eq!(c, input_b);
    assert_eq!(b, expected_c);
}

/**************/
/* signextend */
/**************/

fn signextend_b(input_a: i8, expected_c: i64) {
    let a: i8 = input_a;
    let c: i64;
    unsafe {
        asm!(
            "lb {result}, 0({ptr})",
            result = out(reg) c,
            ptr = in(reg) &a,
        );
    }
    assert_eq!(c, expected_c);
}

fn signextend_h(input_a: i16, expected_c: i64) {
    let a: i16 = input_a;
    let c: i64;
    unsafe {
        asm!(
            "lh {result}, 0({ptr})",
            result = out(reg) c,
            ptr = in(reg) &a,
        );
    }
    assert_eq!(c, expected_c);
}

fn signextend_w(input_a: i32, expected_c: i64) {
    let a: i32 = input_a;
    let c: i64;
    unsafe {
        asm!(
            "lw {result}, 0({ptr})",
            result = out(reg) c,
            ptr = in(reg) &a,
        );
    }
    assert_eq!(c, expected_c);
}

/**********/
/* branch */
/**********/

fn diagnostic_riscv_ima_branch() {
    // bltu (RISCV) -> ltu (ZisK)
    {
        let a: u64 = 0xFFFF_FFFF_FFFF_FFFF;
        let b: u64 = 0xFFFF_FFFF_FFFF_FFFE;
        let c: u64;

        // Use RISCV inline assembly to ensure ZisK instruction is called
        unsafe {
            std::arch::asm!(
                "mv {result}, {input1}",          // Initialize result with a
                "bltu {input2}, {input1}, 2f",     // If b < a, skip next instruction
                "mv {result}, {input2}",          // Otherwise, set result to b (minimum)
                "2:",                             // Label for branch target
                result = out(reg) c,
                input1 = in(reg) a,
                input2 = in(reg) b,
            );
        }

        assert_eq!(c, 0xFFFF_FFFF_FFFF_FFFF); // Check result
    }
    println!("diagnostic_riscv_ima() success");

    // blt (RISCV) -> lt (ZisK)
    {
        let a: i64 = 0xFF_FFFF_FFFF_FFFF;
        let b: i64 = 0xFF_FFFF_FFFF_FFFE;
        let c: i64;

        // Use RISCV inline assembly to ensure ZisK instruction is called
        unsafe {
            std::arch::asm!(
                "mv {result}, {input1}",          // Initialize result with a
                "blt {input2}, {input1}, 2f",     // If b < a, skip next instruction
                "mv {result}, {input2}",          // Otherwise, set result to b (minimum)
                "2:",                             // Label for branch target
                result = out(reg) c,
                input1 = in(reg) a,
                input2 = in(reg) b,
            );
        }

        assert_eq!(c, 0xFF_FFFF_FFFF_FFFF); // Check result
    }
}

/**********/
/* RISC-V */
/**********/

fn riscv_xori() {
    let a: u64 = 0xFFFF_FFFF_0000_0000;
    const B: u64 = 0xFFu64; // immediate
    let expected_c: u64 = 0xFFFF_FFFF_0000_00FF;
    let c: u64;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "xori {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = const B, // immediate
        );
    }

    assert_eq!(c, expected_c); // Check result
}

fn riscv_ori() {
    let a: u64 = 0xFFFF_FFFF_0000_0000;
    const B: u64 = 0xFFu64; // immediate
    let expected_c: u64 = 0xFFFF_FFFF_0000_00FF;
    let c: u64;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "ori {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = const B, // immediate
        );
    }

    assert_eq!(c, expected_c); // Check result
}

fn riscv_fence() {
    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!("fence",);
    }
}

fn riscv_fence_i() {
    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!("fence.i",);
    }
}

fn riscv_ebreak() {
    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!("ebreak",);
    }
}

fn riscv_lr_d() {
    let a: u64 = 0xFFFF_FFFF_0000_0000;
    let expected_c: u64 = a;
    let c: u64;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "lr.d {result}, 0({ptr})",
            result = out(reg) c,
            ptr = in(reg) &a,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

fn riscv_lr_w() {
    let a: u32 = 0xFFFF_FFFF;
    let expected_c: u32 = a;
    let c: u32;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "lr.w {result}, 0({ptr})",
            result = out(reg) c,
            ptr = in(reg) &a,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

fn riscv_sc_d() {
    let a: u64 = 0xFFFF_FFFF_0000_0000;
    let b: u64;
    let expected_c: u64 = a;
    let c: u64;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "lr.d {result}, 0({ptr})",
            "sc.d {result2}, {result}, 0({ptr})",
            result = out(reg) c,
            result2 = out(reg) b,
            ptr = in(reg) &a,
        );
    }

    assert_eq!(c, expected_c); // Check result
    assert_eq!(b, 0); // Check result
}

fn riscv_sc_w() {
    let a: u32 = 0xFFFF_FFFF;
    let b: u32;
    let expected_c: u32 = a;
    let c: u32;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "lr.w {result}, 0({ptr})",
            "sc.w {result2}, {result}, 0({ptr})",
            result = out(reg) c,
            result2 = out(reg) b,
            ptr = in(reg) &a,
        );
    }

    assert_eq!(c, expected_c); // Check result
    assert_eq!(b, 0); // Check result
}

fn riscv_sll(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "sll {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

#[allow(dead_code)]
fn sll_no_check(a: u64, b: u64) {
    let _c: u64;
    unsafe {
        std::arch::asm!(
            "sll {result}, {input1}, {input2}",
            result = out(reg) _c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }
}

fn riscv_srl(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "srl {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

#[allow(dead_code)]
fn srl_no_check(a: u64, b: u64) {
    let _c: u64;
    unsafe {
        std::arch::asm!(
            "srl {result}, {input1}, {input2}",
            result = out(reg) _c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }
}

fn riscv_sra(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "sra {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

#[allow(dead_code)]
fn sra_no_check(a: u64, b: u64) {
    let _c: u64;
    unsafe {
        std::arch::asm!(
            "sra {result}, {input1}, {input2}",
            result = out(reg) _c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }
}

fn riscv_slli() {
    let a: u64 = 1;
    const B: u64 = 3;
    let expected_c: u64 = 8;
    let c: u64;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "slli {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = const B,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

fn riscv_slliw() {
    let a: u32 = 1;
    const B: u32 = 3;
    let expected_c: u32 = 8;
    let c: u32;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "slliw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = const B,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

fn riscv_sraiw() {
    let a: u32 = 8;
    const B: u32 = 3;
    let expected_c: u32 = 1;
    let c: u32;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "sraiw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = const B,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

fn riscv_srliw() {
    let a: u32 = 8;
    const B: u32 = 3;
    let expected_c: u32 = 1;
    let c: u32;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "srliw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = const B,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

fn riscv_slti() {
    let a: u32 = 2;
    const B: u32 = 3;
    let expected_c: u32 = 1;
    let c: u32;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "slti {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = const B,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

fn riscv_slt(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "slt {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    assert_eq!(c, expected_c); // Check result
}

fn riscv_csrrw() {
    // csrrw rd, csr, rs1 - Read old, write new
    // csrrw rd, csr, x0 - Read only (write zero)
    // csrrw x0, csr, rs1 - Write only (discard old value)

    {
        let a: u64 = 3;
        let c: u64;

        // Use RISCV inline assembly to ensure RISC-V instruction is called
        unsafe {
            std::arch::asm!(
                "csrrw {result}, 3, {input1}",
                result = out(reg) c,
                input1 = in(reg) a,
            );
        }

        assert_eq!(c, 0); // Check result
    }
    {
        let a: u64 = 0;
        let c: u64;

        // Use RISCV inline assembly to ensure RISC-V instruction is called
        unsafe {
            std::arch::asm!(
                "csrrw {result}, 3, {input1}",
                result = out(reg) c,
                input1 = in(reg) a,
            );
        }

        assert_eq!(c, 3); // Check result
    }
}

fn riscv_csrrwi() {
    {
        // Use RISCV inline assembly to ensure RISC-V instruction is called
        unsafe {
            std::arch::asm!("csrrwi x0, 3, 0",);
        }
    }
}

fn riscv_csrrs() {
    {
        let a: u64 = 3;
        let c: u64;

        // Use RISCV inline assembly to ensure RISC-V instruction is called
        unsafe {
            std::arch::asm!(
                "csrrs {result}, 3, {input1}",
                result = out(reg) c,
                input1 = in(reg) a,
            );
        }

        assert_eq!(c, 0); // Check result
    }
    {
        let a: u64 = 0;
        let c: u64;

        // Use RISCV inline assembly to ensure RISC-V instruction is called
        unsafe {
            std::arch::asm!(
                "csrrs {result}, 3, {input1}",
                result = out(reg) c,
                input1 = in(reg) a,
            );
        }

        assert_eq!(c, 3); // Check result
    }
}

fn riscv_csrrsi() {
    {
        let _c: u64;

        // Use RISCV inline assembly to ensure RISC-V instruction is called
        unsafe {
            std::arch::asm!(
                "csrrsi {result}, 3, 0",
                result = out(reg) _c,
            );
        }
    }
}

fn riscv_csrrc() {
    // csrrc rd, csr, rs1 - Read old value, clear bits set in rs1
    // csrrc rd, csr, x0 - Read only (clear no bits)
    // csrrc x0, csr, rs1 - Clear bits only (discard old value)

    let _c: u64;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "csrrc {result}, 3, x0",
            result = out(reg) _c,
        );
    }
}

fn riscv_csrrci() {
    let _c: u64;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "csrrci {result}, 3, 0",
            result = out(reg) _c,
        );
    }
}
