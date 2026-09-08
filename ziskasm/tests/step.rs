//! End-to-end test for the `step` a-source: reads the current step number on two
//! consecutive instructions and checks the emulator supplies a running counter
//! (nonzero, and incrementing by exactly one per instruction).

use zisk_common::EmuTrace;
use ziskasm::{assemble, parser::parse_program};
use ziskemu::{EmuOptions, ZiskEmulator};

#[test]
fn step_source_reads_running_counter() {
    // `add(step, 0)` puts the current step into `c` (copyb only copies `b`, so we
    // use `add`). Two consecutive reads are one instruction apart. We write both
    // to the output and check the difference is exactly 1.
    let src = "\
define OUTPUT_ADDR 0xa0410000
main:
\tadd(step, 0) -> r5             ; r5 = step at this instruction (A)
\tadd(step, 0) -> r6             ; r6 = step at the next instruction (A + 1)
\tcopyb(0, OUTPUT_ADDR) -> r8    ; r8 = output pointer
\tcopyb(r8, r5) -> 8[a + 0]      ; output[0] = step A
\tadd(r8, 8) -> r8              ; advance output pointer
\tcopyb(r8, r6) -> 8[a + 0]      ; output[1] = step B
\tret
";

    let program = parse_program(src, "step_test").expect("parse should succeed");
    let rom = assemble(&program).expect("assembly should succeed");

    let input: Vec<u8> = Vec::new();
    let out = ZiskEmulator::process_rom(&rom, &input, &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation should succeed");

    let step_a = u64::from_le_bytes(out[0..8].try_into().unwrap());
    let step_b = u64::from_le_bytes(out[8..16].try_into().unwrap());

    // The step counter is running (BIOS/launcher instructions executed first)...
    assert!(step_a > 0, "step should be nonzero, got {step_a}");
    // ...and advances by exactly one instruction between the two reads.
    assert_eq!(step_b, step_a + 1, "consecutive `step` reads should differ by 1");
}
