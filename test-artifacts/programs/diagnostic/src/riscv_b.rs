#![cfg(feature = "bit_manipulation_extensions")]

pub fn diagnostic_riscv_b() {
    // B bit manipulation extensions: Zbb, Zba, Zbs, Zbc, Zbkb, Zbkc, Zbkx

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

    packh(0x00000000000000FF, 0x00000000000000FF, 0x000000000000FFFF);
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

    min(0x0000000000000001, 0x0000000000000002, 0x0000000000000001);
    min(0x0000000000000002, 0x0000000000000001, 0x0000000000000001);
    min(0xFFFFFFFFFFFFFFFF, 0x0000000000000000, 0xFFFFFFFFFFFFFFFF);
    min(0x0000000000000000, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);
    min(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);

    minu(0x0000000000000001, 0x0000000000000002, 0x0000000000000001);
    minu(0x0000000000000002, 0x0000000000000001, 0x0000000000000001);
    minu(0xFFFFFFFFFFFFFFFF, 0x0000000000000000, 0x0000000000000000);
    minu(0x0000000000000000, 0xFFFFFFFFFFFFFFFF, 0x0000000000000000);
    minu(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);

    max(0x0000000000000001, 0x0000000000000002, 0x0000000000000002);
    max(0x0000000000000002, 0x0000000000000001, 0x0000000000000002);
    max(0xFFFFFFFFFFFFFFFF, 0x0000000000000000, 0x0000000000000000);
    max(0x0000000000000000, 0xFFFFFFFFFFFFFFFF, 0x0000000000000000);
    max(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);

    maxu(0x0000000000000001, 0x0000000000000002, 0x0000000000000002);
    maxu(0x0000000000000002, 0x0000000000000001, 0x0000000000000002);
    maxu(0xFFFFFFFFFFFFFFFF, 0x0000000000000000, 0xFFFFFFFFFFFFFFFF);
    maxu(0x0000000000000000, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);
    maxu(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);

    sext_b(0x00000000000000FF, 0xFFFFFFFFFFFFFFFF);
    sext_b(0x000000000000007F, 0x000000000000007F);
    sext_b(0x0000000000000080, 0xFFFFFFFFFFFFFF80);
    sext_b(0x0000000000000000, 0x0000000000000000);

    sext_h(0x000000000000FFFF, 0xFFFFFFFFFFFFFFFF);
    sext_h(0x0000000000007FFF, 0x0000000000007FFF);
    sext_h(0x0000000000008000, 0xFFFFFFFFFFFF8000);
    sext_h(0x0000000000000000, 0x0000000000000000);

    zext_h(0x000000000000FFFF, 0x000000000000FFFF);
    zext_h(0x0000000000007FFF, 0x0000000000007FFF);
    zext_h(0x0000000000008000, 0x0000000000008000);
    zext_h(0x0000000000000000, 0x0000000000000000);

    clz(0x0000000000000000, 64);
    clz(0x0000000000000001, 63);
    clz(0x0000000000000002, 62);
    clz(0x0100000000000000, 7);
    clz(0x1000000000000000, 3);
    clz(0xFFFFFFFFFFFFFFFF, 0);

    clz_w(0x0000000000000000, 32);
    clz_w(0x0000000000000001, 31);
    clz_w(0x0000000000000002, 30);
    clz_w(0x0000000001000000, 7);
    clz_w(0x0000000010000000, 3);
    clz_w(0x00000000FFFFFFFF, 0);

    ctz(0x0000000000000000, 64);
    ctz(0x0000000000000001, 0);
    ctz(0x0000000000000002, 1);
    ctz(0x0000000000000004, 2);
    ctz(0x0100000000000000, 56);
    ctz(0x1000000000000000, 60);
    ctz(0xFFFFFFFFFFFFFFFF, 0);

    ctz_w(0x0000000000000000, 32);
    ctz_w(0x0000000000000001, 0);
    ctz_w(0x0000000000000002, 1);
    ctz_w(0x0000000000000004, 2);
    ctz_w(0x0000000001000000, 24);
    ctz_w(0x0000000010000000, 28);
    ctz_w(0x00000000FFFFFFFF, 0);

    cpop(0x0000000000000000, 0);
    cpop(0x0000000000000001, 1);
    cpop(0x0000000000000003, 2);
    cpop(0x000000000000000F, 4);
    cpop(0x01000000000000FF, 9);
    cpop(0xFFFFFFFFFFFFFFFF, 64);

    cpop_w(0x0000000000000000, 0);
    cpop_w(0x0000000000000001, 1);
    cpop_w(0x0000000000000003, 2);
    cpop_w(0x000000000000000F, 4);
    cpop_w(0x00000000010000FF, 9);
    cpop_w(0x00000000FFFFFFFF, 32);

    orc_b(0x0000000000000000, 0x0000000000000000);
    orc_b(0x00000000000000FF, 0x00000000000000FF);
    orc_b(0x0000000000000100, 0x000000000000FF00);
    orc_b(0x0100000000000000, 0xFF00000000000000);
    orc_b(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);

    bclr(0xFFFFFFFFFFFFFFFF, 0, 0xFFFFFFFFFFFFFFFE);
    bclr(0xFFFFFFFFFFFFFFFF, 1, 0xFFFFFFFFFFFFFFFD);
    bclr(0xFFFFFFFFFFFFFFFF, 2, 0xFFFFFFFFFFFFFFFB);
    bclr(0xFFFFFFFFFFFFFFFF, 3, 0xFFFFFFFFFFFFFFF7);
    bclr(0xFFFFFFFFFFFFFFFF, 63, 0x7FFFFFFFFFFFFFFF);

    bclri();

    bext(0xFFFFFFFFFFFFFFFF, 0, 1);
    bext(0xFFFFFFFFFFFFFFFF, 1, 1);
    bext(0xFFFFFFFFFFFFFFFF, 2, 1);
    bext(0xFFFFFFFFFFFFFFFF, 63, 1);
    bext(0x0000000000000000, 32, 0);

    bexti();

    binv(0xFFFFFFFFFFFFFFFF, 0, 0xFFFFFFFFFFFFFFFE);
    binv(0xFFFFFFFFFFFFFFFF, 1, 0xFFFFFFFFFFFFFFFD);
    binv(0xFFFFFFFFFFFFFFFF, 2, 0xFFFFFFFFFFFFFFFB);
    binv(0xFFFFFFFFFFFFFFFF, 3, 0xFFFFFFFFFFFFFFF7);
    binv(0xFFFFFFFFFFFFFFFF, 63, 0x7FFFFFFFFFFFFFFF);
    binv(0x0000000000000000, 32, 0x0000000100000000);

    binvi();

    bset(0x0000000000000000, 0, 0x0000000000000001);
    bset(0x0000000000000000, 1, 0x0000000000000002);
    bset(0x0000000000000000, 2, 0x0000000000000004);
    bset(0x0000000000000000, 3, 0x0000000000000008);
    bset(0x0000000000000000, 63, 0x8000000000000000);

    bseti();

    add_u_w(0x00000000FFFFFFFF, 0x0000000000000001, 0x0000000100000000);
    add_u_w(0x0000000000000001, 0x0000000000000001, 0x0000000000000002);
    add_u_w(0x0000000000000002, 0x0000000000000002, 0x0000000000000004);
    add_u_w(0x0000000000000004, 0x0000000000000004, 0x0000000000000008);
    add_u_w(0xFFFFFFFFFFFFFFFF, 0x0000000000000001, 0x0000000100000000);
    add_u_w(0xFFFFFFFF00000001, 0x0000000000000001, 0x0000000000000002);
    add_u_w(0xFFFFFFFF00000002, 0x0000000000000002, 0x0000000000000004);
    add_u_w(0xFFFFFFFF00000004, 0x0000000000000004, 0x0000000000000008);

    sh1add(0x0000000000000001, 0x0000000000000001, 0x0000000000000003);
    sh1add(0x0000000000000002, 0x0000000000000002, 0x0000000000000006);
    sh1add(0x0000000000000004, 0x0000000000000004, 0x000000000000000C);
    sh1add(0x0000000000000008, 0x0000000000000008, 0x0000000000000018);
    sh1add(0x4000000000000000, 0x7FFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);

    sh1add_u_w(0x0000000000000001, 0x0000000000000001, 0x0000000000000003);
    sh1add_u_w(0x0000000000000002, 0x0000000000000002, 0x0000000000000006);
    sh1add_u_w(0x0000000000000004, 0x0000000000000004, 0x000000000000000C);
    sh1add_u_w(0x0000000000000008, 0x0000000000000008, 0x0000000000000018);
    sh1add_u_w(0x4000000000000000, 0x7FFFFFFFFFFFFFFF, 0x7FFFFFFFFFFFFFFF);

    sh2add(0x0000000000000001, 0x0000000000000001, 0x0000000000000005);
    sh2add(0x0000000000000002, 0x0000000000000002, 0x000000000000000A);
    sh2add(0x0000000000000004, 0x0000000000000004, 0x0000000000000014);
    sh2add(0x0000000000000008, 0x0000000000000008, 0x0000000000000028);
    sh2add(0x2000000000000000, 0x7FFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);

    sh2add_u_w(0x0000000000000001, 0x0000000000000001, 0x0000000000000005);
    sh2add_u_w(0x0000000000000002, 0x0000000000000002, 0x000000000000000A);
    sh2add_u_w(0x0000000000000004, 0x0000000000000004, 0x0000000000000014);
    sh2add_u_w(0x0000000000000008, 0x0000000000000008, 0x0000000000000028);
    sh2add_u_w(0x2000000000000000, 0x7FFFFFFFFFFFFFFF, 0x7FFFFFFFFFFFFFFF);

    sh3add(0x0000000000000001, 0x0000000000000001, 0x0000000000000009);
    sh3add(0x0000000000000002, 0x0000000000000002, 0x0000000000000012);
    sh3add(0x0000000000000004, 0x0000000000000004, 0x0000000000000024);
    sh3add(0x0000000000000008, 0x0000000000000008, 0x0000000000000048);
    sh3add(0x1000000000000000, 0x7FFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);

    sh3add_u_w(0x0000000000000001, 0x0000000000000001, 0x0000000000000009);
    sh3add_u_w(0x0000000000000002, 0x0000000000000002, 0x0000000000000012);
    sh3add_u_w(0x0000000000000004, 0x0000000000000004, 0x0000000000000024);
    sh3add_u_w(0x0000000000000008, 0x0000000000000008, 0x0000000000000048);
    sh3add_u_w(0x1000000000000000, 0x7FFFFFFFFFFFFFFF, 0x7FFFFFFFFFFFFFFF);

    slli_u_w();

    // CLMUL tests - lower 64 bits of carryless multiplication
    clmul(0x0000000000000000, 0x0000000000000000, 0x0000000000000000); // 0 * 0 = 0
    clmul(0x0000000000000001, 0x0000000000000001, 0x0000000000000001); // 1 * 1 = 1
    clmul(0x0000000000000002, 0x0000000000000002, 0x0000000000000004); // 2 * 2 = 4
    clmul(0x0000000000000003, 0x0000000000000003, 0x0000000000000005); // (x+1) * (x+1) = x^2 + 1
    clmul(0x0000000000000005, 0x0000000000000003, 0x000000000000000F); // (x^2+1) * (x+1) = x^3 + x^2 + x + 1
    clmul(0x000000000000000F, 0x000000000000000F, 0x0000000000000055); // Full nibble
    clmul(0x00000000000000FF, 0x00000000000000FF, 0x0000000000005555); // Full byte
    clmul(0x0000000000000101, 0x0000000000000101, 0x0000000000010001); // Sparse pattern
    clmul(0xFFFFFFFFFFFFFFFF, 0x0000000000000001, 0xFFFFFFFFFFFFFFFF); // All 1s * 1
    clmul(0xFFFFFFFFFFFFFFFF, 0x0000000000000002, 0xFFFFFFFFFFFFFFFE); // All 1s * 2
    clmul(0x0123456789ABCDEF, 0x0000000000000001, 0x0123456789ABCDEF); // Identity
    clmul(0x8000000000000000, 0x0000000000000002, 0x0000000000000000); // MSB overflow to upper

    // CLMULH tests - upper 64 bits of carryless multiplication
    clmul_h(0x0000000000000000, 0x0000000000000000, 0x0000000000000000); // 0 * 0
    clmul_h(0x0000000000000001, 0x0000000000000001, 0x0000000000000000); // Small values (result in lower)
    clmul_h(0x0000000000000002, 0x0000000000000002, 0x0000000000000000); // Result in lower bits
    clmul_h(0x0000000000000003, 0x0000000000000003, 0x0000000000000000); // Result in lower bits
    clmul_h(0x8000000000000000, 0x0000000000000002, 0x0000000000000001); // MSB * 2
    clmul_h(0x8000000000000000, 0x8000000000000000, 0x4000000000000000); // MSB * MSB
    clmul_h(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0x5555555555555555); // All 1s * all 1s
    clmul_h(0xFFFFFFFFFFFFFFFF, 0x8000000000000000, 0x7FFFFFFFFFFFFFFF); // All 1s * MSB
    clmul_h(0x0123456789ABCDEF, 0xFEDCBA9876543210, 0x00E038D8688850B0); // Complex pattern
    clmul_h(0xAAAAAAAAAAAAAAAA, 0x5555555555555555, 0x2222222222222222); // Alternating bits

    // CLMULR tests - reverse: bits [63:1] of upper 64 bits (shifted right by 1)
    clmul_r(0x0000000000000000, 0x0000000000000000, 0x0000000000000000); // 0 * 0
    clmul_r(0x0000000000000001, 0x0000000000000001, 0x0000000000000000); // Small values
    clmul_r(0x0000000000000002, 0x0000000000000002, 0x0000000000000000); // Result in lower
    clmul_r(0x0000000000000003, 0x0000000000000003, 0x0000000000000000); // Result in lower
    clmul_r(0x8000000000000000, 0x0000000000000002, 0x0000000000000002); // MSB * 2 >> 1
    clmul_r(0x8000000000000000, 0x8000000000000000, 0x8000000000000000); // MSB * MSB >> 1
    clmul_r(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xAAAAAAAAAAAAAAAA); // All 1s * all 1s >> 1
    clmul_r(0xFFFFFFFFFFFFFFFF, 0x8000000000000000, 0xFFFFFFFFFFFFFFFF); // All 1s * MSB >> 1
    clmul_r(0x0123456789ABCDEF, 0xFEDCBA9876543210, 0x01C071B0D110A160); // Complex >> 1
    clmul_r(0xAAAAAAAAAAAAAAAA, 0x5555555555555555, 0x4444444444444444); // Alternating >> 1

    xperm4(0x0000000000000000, 0x0000000000000000, 0x0000000000000000);
    xperm4(0x0123456789ABCDEF, 0x0123456789ABCDEF, 0xFEDCBA9876543210);
    xperm4(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF);

    xperm8(0x0000000000000000, 0x0000000000000000, 0x0000000000000000);
    xperm8(0x0102030405060708, 0x0001020304050607, 0x0807060504030201);
    xperm8(0xFFFFFFFFFFFFFFFF, 0xFFFFFFFFFFFFFFFF, 0x0000000000000000);

    println!("All RISC-V B extension diagnostics passed!");
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

fn min(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "min {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn minu(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "minu {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn max(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "max {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn maxu(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "maxu {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn sext_b(input_a: u64, expected_c: u64) {
    let mut a: u64 = input_a;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "sext.b {input1}, {input1}",
            input1 = inout(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(a, expected_c);
}

fn sext_h(input_a: u64, expected_c: u64) {
    let mut a: u64 = input_a;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "sext.h {input1}, {input1}",
            input1 = inout(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(a, expected_c);
}

fn zext_h(input_a: u64, expected_c: u64) {
    let mut a: u64 = input_a;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "zext.h {input1}, {input1}",
            input1 = inout(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(a, expected_c);
}

fn clz(input_a: u64, expected_c: u64) {
    let mut a: u64 = input_a;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "clz {input1}, {input1}",
            input1 = inout(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(a, expected_c);
}

fn clz_w(input_a: u64, expected_c: u64) {
    let mut a: u64 = input_a;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "clzw {input1}, {input1}",
            input1 = inout(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(a, expected_c);
}

fn ctz(input_a: u64, expected_c: u64) {
    let mut a: u64 = input_a;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "ctz {input1}, {input1}",
            input1 = inout(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(a, expected_c);
}

fn ctz_w(input_a: u64, expected_c: u64) {
    let mut a: u64 = input_a;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "ctzw {input1}, {input1}",
            input1 = inout(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(a, expected_c);
}

fn cpop(input_a: u64, expected_c: u64) {
    let mut a: u64 = input_a;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "cpop {input1}, {input1}",
            input1 = inout(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(a, expected_c);
}

fn cpop_w(input_a: u64, expected_c: u64) {
    let mut a: u64 = input_a;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "cpopw {input1}, {input1}",
            input1 = inout(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(a, expected_c);
}

fn orc_b(input_a: u64, expected_c: u64) {
    let mut a: u64 = input_a;

    // Use RISCV inline assembly to ensure RISC-V instruction is called
    unsafe {
        std::arch::asm!(
            "orc.b {input1}, {input1}",
            input1 = inout(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(a, expected_c);
}

fn bclr(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "bclr {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn bclri() {
    let a: u64 = 0xFFFFFFFFFFFFFFFF;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "bclri {result}, {input1}, 0",
            result = out(reg) c,
            input1 = in(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(c, 0xFFFFFFFFFFFFFFFE);
}

fn bext(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "bext {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn bexti() {
    let a: u64 = 0xFFFFFFFFFFFFFFFF;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "bexti {result}, {input1}, 0",
            result = out(reg) c,
            input1 = in(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(c, 1);
}

fn binv(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "binv {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn binvi() {
    let a: u64 = 0xFFFFFFFFFFFFFFFF;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "binvi {result}, {input1}, 0",
            result = out(reg) c,
            input1 = in(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(c, 0xFFFFFFFFFFFFFFFE);
}

fn bset(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "bset {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn bseti() {
    let a: u64 = 0;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "bseti {result}, {input1}, 1",
            result = out(reg) c,
            input1 = in(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(c, 0x2);
}

fn add_u_w(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "add.uw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn sh1add(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "sh1add {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn sh1add_u_w(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "sh1add.uw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn sh2add(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "sh2add {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn sh2add_u_w(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "sh2add.uw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn sh3add(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "sh3add {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn sh3add_u_w(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "sh3add.uw {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn slli_u_w() {
    let a: u64 = 0xFFFFFFFF00000001;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "slli.uw {result}, {input1}, 1",
            result = out(reg) c,
            input1 = in(reg) a,
        );
    }

    // Check result is as expected
    assert_eq!(c, 0x0000000000000002);
}

fn clmul(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "clmul {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn clmul_h(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "clmulh {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn clmul_r(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "clmulr {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn xperm4(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "xperm4 {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}

fn xperm8(input_a: u64, input_b: u64, expected_c: u64) {
    let a: u64 = input_a;
    let b: u64 = input_b;
    let c: u64;

    // Use RISCV inline assembly to ensure ZisK instruction is called
    unsafe {
        std::arch::asm!(
            "xperm8 {result}, {input1}, {input2}",
            result = out(reg) c,
            input1 = in(reg) a,
            input2 = in(reg) b,
        );
    }

    // Check result is as expected
    assert_eq!(c, expected_c);
}
