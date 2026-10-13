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

/// A `u64` initializer may be a symbol, i.e. its address: here a pointer table
/// (in ROM and in RAM, including a forward reference to a later declaration) is
/// followed to its targets, which sum to 7.
#[test]
fn symbol_initializers_hold_addresses() {
    let src = "\
const u64 PTRS = ONE, TWO          ; ROM pointers, forward references
u64 RPTR = FOUR                    ; RAM pointer
const u64 ONE = 1
u64 TWO = 2
u64 FOUR = 4
define OUTPUT_ADDR 0xa0410000

main:
\tcopyb(0, PTRS) -> r10
\tcopyb(r10, 8[a + 0]) -> r5       ; r5 = &ONE
\tcopyb(r5, 8[a + 0]) -> r7        ; r7 = 1
\tcopyb(r10, 8[a + 8]) -> r5       ; r5 = &TWO
\tcopyb(r5, 8[a + 0]) -> r6
\tadd(r7, r6) -> r7                ; + 2
\tcopyb(0, [RPTR]) -> r5           ; r5 = &FOUR
\tcopyb(r5, 8[a + 0]) -> r6
\tadd(r7, r6) -> r7                ; + 4
\tcopyb(0, OUTPUT_ADDR) -> r11
\tcopyb(r11, r7) -> 8[a + 0]
\tret
";

    let program = parse_program(src, "data_sym_test").expect("parse should succeed");
    let rom = assemble(&program).expect("assembly should succeed");

    let out = ZiskEmulator::process_rom(&rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation should succeed");

    let result = u64::from_le_bytes(out[0..8].try_into().unwrap());
    assert_eq!(result, 7, "the pointers should reach 1, 2 and 4");
}

/// A symbol initializer needs `u64` (an address does not fit a narrower type), and
/// its symbol must exist.
#[test]
fn symbol_initializers_are_checked() {
    let narrow = "u32 P = Q\nu64 Q = 0\nmain:\n\tret\n";
    let err = parse_program(narrow, "narrow").expect_err("u32 symbol initializer");
    assert!(err.contains("needs u64"), "{err}");

    let undefined = "u64 P = NOWHERE\nmain:\n\tret\n";
    let program = parse_program(undefined, "undefined").expect("parse should succeed");
    let err = assemble(&program).expect_err("undefined symbol");
    assert!(err.contains("undefined symbol `NOWHERE`"), "{err}");
}
