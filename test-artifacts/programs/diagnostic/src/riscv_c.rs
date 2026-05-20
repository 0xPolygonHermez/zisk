#![cfg(all(target_os = "zkvm", target_vendor = "zisk"))]

//use std::arch::asm;

pub fn diagnostic_riscv_c() {
    // riscv_c_add(1, 2, 3);
    // println!("diagnostic_riscv_c() success");
}
// c.add c.addi c.addi16sp c.addi4spn c.and c.andi c.beqz c.bnez c.ebreak c.fld c.fldsp c.flw c.flwsp c.fsd c.fsdsp c.fsw c.fswsp c.j c.jal c.jalr c.jr c.ld c.ldsp c.li c.lui c.lw c.lwsp c.mv c.or c.sd c.sdsp c.slli c.srai c.srli c.sub c.sw c.swsp c.xor

// fn riscv_c_add(input_a: u64, input_b: u64, expected_c: u64) {
//     let mut a: u64 = input_a;
//     let b: u64 = input_b;

//     // Use RISCV inline assembly to ensure RISC-V instruction is called
//     unsafe {
//         std::arch::asm!(
//             "c.add {input1}, {input2}",
//             input1 = inout(reg) a,
//             input2 = in(reg) b,
//         );
//     }

//     assert_eq!(a, expected_c); // Check we branched correctly
// }
