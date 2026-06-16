//use std::arch::asm;

pub fn diagnostic_riscv_zbkb() {
    rev8(0x0102030405060708, 0x0807060504030201);
    brev8(0x0102040810204080, 0x8040201008040201);
    andn(0xF0F0F0F0F0F0F0F0, 0x0F0F0F0F0F0F0F0F, 0xF0F0F0F0F0F0F0F0);
    orn(0x0F0F0F0F0F0F0F0F, 0x0F0F0F0F0F0F0F0F, 0xFFFFFFFFFFFFFFFF);
    xnor(0x0F0F0F0F0F0F0F0F, 0x0F0F0F0F0F0F0F0F, 0xFFFFFFFFFFFFFFFF);
    pack(0x00000000FFFFFFFF, 0x00000000FFFFFFFF, 0xFFFFFFFFFFFFFFFF);
    packh(0x00000000000000FF, 0x00000000000000FF, 0xFFFFFFFFFFFFFFFF);
    packw(0x000000000000FFFF, 0x000000000000FFFF, 0xFFFFFFFFFFFFFFFF);
    rol(0x8000000000000001, 1, 0x0000000000000003);
    rolw(0x0000000080000001, 1, 0x0000000000000003);
    ror(0x0000000000000003, 1, 0x8000000000000001);
    rorw(0x0000000000000003, 1, 0xFFFFFFFF80000001);
    rori();
    roriw();
    println!("All RISC-V Zbkb extension diagnostics passed!");
}

fn rev8(input_a: u64, expected_c: u64) {
    let mut a: u64 = input_a;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "rev8 {input1}, {input1}",
            input1 = inout(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(a, expected_c);
}

fn brev8(input_a: u64, expected_c: u64) {
    let mut a: u64 = input_a;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "brev8 {input1}, {input1}",
            input1 = inout(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(a, expected_c);
}

fn andn(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "andn {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn orn(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "orn {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn xnor(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "xnor {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn pack(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "pack {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn packh(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "packh {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn packw(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "packw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn rol(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "rol {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn rolw(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "rolw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn ror(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "ror {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn rorw(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "rorw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn rori() {
    let a: u64 = 0x0000000000000003;
    let c: u64;
    let expected_c: u64 = 0x8000000000000001;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "rori {result}, {input1}, 1",
            result = out(reg) c,
            input1 = in(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn roriw() {
    let a: u64 = 0x0000000000000003;
    let c: u64;
    let expected_c: u64 = 0xFFFFFFFF80000001;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "roriw {result}, {input1}, 1",
            result = out(reg) c,
            input1 = in(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}
