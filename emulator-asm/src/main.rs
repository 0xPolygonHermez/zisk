use std::arch::global_asm;
use std::ptr;

global_asm!(include_str!("emu_asm.s"));

extern "C" {
    fn emulator_execute() -> usize;
}
use zisk_core::OUTPUT_ADDR;

fn main() {
    println!("EMULATOR ASM");
    println!("Calling emulator_execute()");
    let result = unsafe { emulator_execute() };
    println!("Called emulator_execute() result={}", result);

    // Get output
    let mut output_address = OUTPUT_ADDR as *const u32;
    let output_length = unsafe { ptr::read(output_address) };
    output_address = output_address.wrapping_add(1);
    let mut output: Vec<u32> = Vec::with_capacity(output_length as usize);
    for _i in 0..output_length {
        let data = unsafe { ptr::read(output_address) };
        output.push(data);
        output_address = output_address.wrapping_add(1);
    }

    // Log the output to console
    for o in &output {
        println!("{:08x}", o);
    }
}
