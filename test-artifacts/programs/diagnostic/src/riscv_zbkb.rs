//use std::arch::asm;

pub fn diagnostic_riscv_zbkb() {
    rev8(0x0102030405060708, 0x0807060504030201);
    rev8(0xF0F0F0F0F0F0F0F0, 0xF0F0F0F0F0F0F0F0);
    rev8(0x0000000000000000, 0x0000000000000000);
    rev8(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);
    rev8(0x0123456789ABCDEF, 0xEFCDAB8967452301);
    rev8(0x00000000FFFFFFFF, 0xFFFFFFFF00000000);
    rev8(0xFFFFFFFF00000000, 0x00000000FFFFFFFF);

    brev8(0x0102040810204080, 0x8040201008040201);
    brev8(0xF0F0F0F0F0F0F0F0, 0x0F0F0F0F0F0F0F0F);
    brev8(0x0000000000000000, 0x0000000000000000);
    brev8(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);
    brev8(0x0123456789ABCDEF, 0x80C4A2E691D5B3F7);
    brev8(0x00000000FFFFFFFF, 0x00000000FFFFFFFF);
    brev8(0xFFFFFFFF00000000, 0xFFFFFFFF00000000);

    andn(0xF0F0F0F0F0F0F0F0, 0x0F0F0F0F0F0F0F0F, 0xF0F0F0F0F0F0F0F0);
    andn(0xFFFFFFFFFFFFFFFF, 0x0000000000000000, 0xFFFFFFFFFFFFFFFF);
    andn(0x0000000000000000, 0xFFFFFFFFFFFFFFFF, 0x0000000000000000);
    andn(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0x0000000000000000);
    andn(0x0000000000000000, 0x0000000000000000, 0x0000000000000000);

    orn(0x0F0F0F0F0F0F0F0F, 0x0F0F0F0F0F0F0F0F, 0xFFFFFFFFFFFFFFFF);
    orn(0xFFFFFFFFFFFFFFFF, 0x0000000000000000, 0xFFFFFFFFFFFFFFFF);
    orn(0x0000000000000000, 0xFFFFFFFFFFFFFFFF, 0x0000000000000000);
    orn(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);
    orn(0x0000000000000000, 0x0000000000000000, 0xFFFFFFFFFFFFFFFF);

    xnor(0x0F0F0F0F0F0F0F0F, 0x0F0F0F0F0F0F0F0F, 0xFFFFFFFFFFFFFFFF);
    xnor(0xFFFFFFFFFFFFFFFF, 0x0000000000000000, 0x0000000000000000);
    xnor(0x0000000000000000, 0xFFFFFFFFFFFFFFFF, 0x0000000000000000);
    xnor(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);
    xnor(0x0000000000000000, 0x0000000000000000, 0xFFFFFFFFFFFFFFFF);

    pack(0x00000000FFFFFFFF, 0x00000000FFFFFFFF, 0xFFFFFFFFFFFFFFFF);
    pack(0xFFFFFFFF00000000, 0xFFFFFFFF00000000, 0x0000000000000000);
    pack(0xFFFFFFFF11223344, 0xFFFFFFFF55667788, 0x5566778811223344);

    packh(0x00000000000000FF, 0x00000000000000FF, 0xFFFFFFFFFFFFFFFF);
    packh(0xFFFFFFFFFFFFFF00, 0xFFFFFFFFFFFFFF00, 0x0000000000000000);
    packh(0xFFFFFFFFFFFFFF11, 0xFFFFFFFFFFFFFF22, 0x0000000000002211);

    packw(0x000000000000FFFF, 0x000000000000FFFF, 0xFFFFFFFFFFFFFFFF);
    packw(0xFFFFFFFFFFFF0000, 0xFFFFFFFFFFFF0000, 0x0000000000000000);
    packw(0xFFFFFFFFFFFF1111, 0xFFFFFFFFFFFF2222, 0x0000000022221111);

    rol(0x8000000000000001, 1, 0x0000000000000003);
    rol(0x0000000000000001, 1, 0x0000000000000002);
    rol(0x0000000000000003, 1, 0x0000000000000006);
    rol(0x0000000000000003, 2, 0x000000000000000C);

    rolw(0x0000000080000001, 1, 0x0000000000000003);
    rolw(0x0000000000000001, 1, 0x0000000000000002);
    rolw(0x0000000000000003, 1, 0x0000000000000006);
    rolw(0x0000000000000003, 2, 0x000000000000000C);
    rolw(0x00000000FFFFFFFF, 1, 0xFFFFFFFFFFFFFFFF);

    ror(0x0000000000000003, 1, 0x8000000000000001);
    ror(0x0000000000000003, 2, 0xC000000000000000);
    ror(0x0000000000000003, 3, 0x6000000000000000);
    ror(0x0000000000000003, 4, 0x3000000000000000);

    rorw(0x0000000000000003, 1, 0xFFFFFFFF80000001);
    rorw(0x0000000000000003, 2, 0xFFFFFFFFC0000000);
    rorw(0x0000000000000003, 3, 0x0000000060000000);
    rorw(0x0000000000000003, 4, 0x0000000030000000);

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
