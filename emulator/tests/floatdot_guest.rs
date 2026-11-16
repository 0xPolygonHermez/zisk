use std::path::Path;
use zisk_common::EmuTrace;
use zisk_core::ZiskRom;
use ziskemu::{Emu, EmuOptions};

fn rom(name: &str) -> ZiskRom {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/fp32/artifacts").join(name);
    let elf = std::fs::read(&path)
        .expect("build the guest ELFs first: node precompiles/floatdot/tests/run_guests.cjs");
    zisk_riscv::elf2rom(&elf).expect("transpile guest")
}

#[test]
#[ignore = "requires both local FP32 guest ELF variants"]
fn actual_guests_agree_on_order_subnormals_fir_and_vector_dot() {
    let expected: Vec<u8> =
        [1u64 << 32, (3u64 << 32) | 1, 0xbec00000, 0x45800000, (8u64 << 32) | 0xff800001]
            .into_iter()
            .flat_map(u64::to_le_bytes)
            .collect();
    for name in ["precompile.elf", "software.elf"] {
        let rom = rom(name);
        let mut emu = Emu::new(&rom);
        emu.run(Vec::new(), &EmuOptions::default(), None::<fn(EmuTrace)>);
        assert!(emu.terminated());
        assert!(!emu.ctx.inst_ctx.error);
        let output = emu.get_output_8();
        assert_eq!(&output[..expected.len()], expected);
        assert!(output[expected.len()..].iter().all(|&byte| byte == 0));
        println!(
            "{name}: {} emulator guest steps; proof cost/time/RAM unmeasured",
            emu.number_of_steps()
        );
    }
}

#[test]
#[ignore = "requires both local mixed-input guest ELF variants"]
fn actual_mixed_input_guests_agree_with_software_widening() {
    let mut outputs = Vec::new();
    for name in ["floatdot-precompile.elf", "floatdot-software.elf"] {
        let rom = rom(name);
        let mut emu = Emu::new(&rom);
        emu.run(Vec::new(), &EmuOptions::default(), None::<fn(EmuTrace)>);
        assert!(emu.terminated());
        assert!(!emu.ctx.inst_ctx.error);
        let output = emu.get_output_8();
        println!(
            "{name}: {} emulator guest steps; proof cost/time/RAM unmeasured",
            emu.number_of_steps()
        );
        outputs.push(output);
    }
    assert_eq!(outputs[0], outputs[1]);
    let words: Vec<u64> = outputs[0][..56]
        .chunks_exact(8)
        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
        .collect();
    assert!(outputs[0][56..].iter().all(|&byte| byte == 0));
    // Exceptional FP16 call: inf - inf and the signaling NaN give NV; initial DZ survives.
    assert_eq!(words[2], (u64::from(8u8 | 16 | 1) << 32) | 0x7fc0_0000);
    assert_eq!(words[5], (24u64 << 32) | 0xff80_0001);
    assert_eq!(words[6], 1u64 << 32);
    println!(
        "mixed outputs: {}",
        words.iter().map(|w| format!("{w:016x}")).collect::<Vec<_>>().join(",")
    );
}
