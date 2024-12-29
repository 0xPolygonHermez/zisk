use std::arch::global_asm;

global_asm!(include_str!("emu_asm.s"));

extern "C" {
    fn emulator_execute() -> usize;
}

fn main() {
    println!("EMULATOR ASM");
    println!("Calling emulator_execute()");
    let result = unsafe { emulator_execute() };
    println!("Called emulator_execute() result={}", result);
}
