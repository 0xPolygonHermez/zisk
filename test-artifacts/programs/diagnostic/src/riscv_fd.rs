#![cfg(all(target_os = "zkvm", target_vendor = "zisk"))]

//use std::arch::asm;

pub fn diagnostic_riscv_fd() {
    {
        let a = 1.1;
        let b = 2.2;
        let c = a + b;
        assert!(c > 3.2 && c < 3.4);
    }
    //fadd_d(1.1, 2.2, 3.3);
    println!("diagnostic_riscv_fd() success");
}

// fadd.d fadd.s fclass.d fclass.s fcvt.d.l fcvt.d.lu fcvt.d.s fcvt.d.w fcvt.d.wu fcvt.l.d fcvt.l.s fcvt.lu.d fcvt.lu.s fcvt.s.d fcvt.s.l fcvt.s.lu fcvt.s.w fcvt.s.wu fcvt.w.d fcvt.w.s fcvt.wu.d fcvt.wu.s fdiv.d fdiv.s feq.d feq.s fld fle.d fle.s flt.d flt.s flw fmadd.d fmadd.s fmax.d fmax.s fmin.d fmin.s fmsub.d fmsub.s fmul.d fmul.s fmv.w.x fmv.x.w fnmadd.d fnmadd.s fnmsub.d fnmsub.s fsd fsgnj.d fsgnj.s fsgnjn.d fsgnjn.s fsgnjx.d fsgnjx.s fsqrt.d fsqrt.s fsub.d fsub.s fsw

// fn fadd_d(input_a: f64, input_b: f64, expected_c: f64) {
//     let a: u64 = input_a.to_bits() as u64;
//     let b: u64 = input_b.to_bits() as u64;
//     let expected_c: u64 = expected_c.to_bits() as u64;
//     let c: u64;
//     //c = a + b;
//     unsafe {
//         asm!(
//             "fld ft2, double_val, {1}",
//             "fld ft3, double_val, {2}",
//             "fadd.d ft4, ft2, ft3",
//             "fsd {0}, ft4",
//             out(reg) c,
//             in(reg) a,
//             in(reg) b,
//         );
//     }

//     // Use RISCV inline assembly to ensure ZisK instruction is called
//     // unsafe {
//     //     std::arch::asm!(
//     //         "xor {result}, {input1}, {input2}",
//     //         result = out(reg) c,
//     //         input1 = in(reg) a,
//     //         input2 = in(reg) b,
//     //     );
//     // }

//     assert_eq!(c, expected_c); // Check we branched correctly
// }
