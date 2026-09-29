//! End-to-end test for data declarations: sums a `const` array living in ROM into
//! a RAM accumulator, exercising const/RAM data, symbolic operands (`NAME` as an
//! address, `[NAME]` as a value), array access via a register pointer, and
//! `jump(label)`.

use zisk_common::EmuTrace;
use ziskasm::{assemble, parser::parse_program};
use ziskemu::{EmuOptions, ZiskEmulator};

#[test]
fn sum_const_array_into_ram() {
    let src = "\
const u64 TABLE = 10, 20, 30, 40   ; ROM array (sum = 100)
u64 count = 4                      ; RAM scalar
u64 acc = 0                        ; RAM accumulator
define OUTPUT_ADDR 0xa0410000

main:
\tcopyb(0, TABLE) -> r10           ; r10 = ADDRESS of TABLE (a pointer)
\tcopyb(0, [count]) -> r6          ; r6 = count value (4)
\tcopyb(0, 0) -> r5                ; i = 0
\tcopyb(0, [acc]) -> r7            ; acc = 0
loop:
\teq(r5, r6), j(done)              ; if i == count -> done
\tcopyb(r10, 8[a + 0]) -> r8       ; r8 = TABLE[i]  (indirect load via pointer)
\tadd(r7, r8) -> r7                ; acc += TABLE[i]
\tadd(r10, 8) -> r10               ; pointer += 8
\tadd(r5, 1) -> r5                 ; i += 1
\tjump(loop)                       ; unconditional back-edge
done:
\tcopyb(0, r7) -> [acc]            ; store acc to RAM (symbolic STORE_MEM)
\tcopyb(0, OUTPUT_ADDR) -> r11
\tcopyb(r11, [acc]) -> 8[a + 0]    ; output[0] = [acc]  (symbolic SRC_MEM)
\tret
";

    let program = parse_program(src, "data_test").expect("parse should succeed");
    let rom = assemble(&program).expect("assembly should succeed");

    let out = ZiskEmulator::process_rom(&rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation should succeed");

    let result = u64::from_le_bytes(out[0..8].try_into().unwrap());
    assert_eq!(result, 100, "sum of the const ROM array should be 100");
}
