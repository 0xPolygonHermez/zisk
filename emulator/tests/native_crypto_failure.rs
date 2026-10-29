//! Adversarial free-input witnesses must fail in the production secp256k1 library.
use std::{collections::HashMap, sync::OnceLock};

use zisk_core::{ZiskRom, GENERAL_RAM_ADDR, ROM_ADDR, STACK_ADDR, STACK_SIZE};
use ziskemu::{Emu, EmuOptions, FailureReason};

fn library() -> &'static (ZiskRom, HashMap<String, u64>) {
    static LIBRARY: OnceLock<(ZiskRom, HashMap<String, u64>)> = OnceLock::new();
    LIBRARY.get_or_init(|| {
        let library = ziskasm::assemble_zisk_library().unwrap();
        let symbols = library.symbols.clone();
        let mut rom = ZiskRom::default();
        zisk_riscv::merge_library(&mut rom, library).unwrap();
        rom.optimize_instruction_lookup().unwrap();
        (rom, symbols)
    })
}

// Step the real fcall normally, then replace its public context's result before
// the guest consumes it. This models an untrusted hint without a production hook.
fn run(
    entry: &str,
    input: [u64; 4],
    forge: Option<&dyn Fn(&mut [u64])>,
) -> (Option<FailureReason>, Vec<u64>) {
    let (rom, symbols) = library();
    let mut emu = Emu::new(rom);
    emu.ctx = emu.create_emu_context(vec![], &EmuOptions::default());
    let arg = GENERAL_RAM_ADDR + 0x1000;
    let out = arg + 32;
    for (i, word) in input.into_iter().enumerate() {
        emu.ctx.inst_ctx.mem.write(arg + i as u64 * 8, word, 8);
    }
    emu.ctx.inst_ctx.pc = symbols[entry];
    emu.set_reg(1, ROM_ADDR); // sentinel: a successful return is observed before stepping it
    emu.set_reg(2, STACK_ADDR + STACK_SIZE - 256);
    emu.set_reg(10, arg);
    emu.set_reg(11, if entry.contains("inv_") || entry.contains("glv_") { out } else { 0 });
    emu.set_reg(12, if entry.contains("glv_") { out + 32 } else { out });
    let mut fcalls = 0;
    for _ in 0..20_000 {
        if emu.terminated() || emu.ctx.inst_ctx.pc == ROM_ADDR {
            let result = (0..8).map(|i| emu.ctx.inst_ctx.mem.read(out + i * 8, 8)).collect();
            return (emu.failure().map(|(reason, _, _)| reason), result);
        }
        let fcall = rom.get_instruction(emu.ctx.inst_ctx.pc).op_str == "fcall";
        emu.step_fast();
        if fcall {
            fcalls += 1;
            if let Some(forge) = forge {
                let ctx = &mut emu.ctx.inst_ctx;
                forge(&mut ctx.fcall.result[..ctx.fcall.result_size as usize]);
                ctx.mem.free_input = ctx.fcall.result[0];
            }
        }
    }
    panic!("{entry} did not terminate ({fcalls} fcalls)");
}

const ONE: [u64; 4] = [1, 0, 0, 0];
const P: [u64; 4] = [0xfffffffefffffc2f, u64::MAX, u64::MAX, u64::MAX];
const N: [u64; 4] = [0xbfd25e8cd0364141, 0xbaaedce6af48a03b, 0xfffffffffffffffe, u64::MAX];

#[test]
fn forged_inverse_hints_fail_closed() {
    for (entry, modulus) in [("zisklib_inv_fp_secp256k1", P), ("zisklib_inv_fn_secp256k1", N)] {
        assert_eq!(run(entry, ONE, None).1[..4], ONE);
        for witness in [[0; 4], modulus] {
            assert_eq!(
                run(entry, ONE, Some(&|hint| hint.copy_from_slice(&witness))).0,
                Some(FailureReason::Trap),
                "{entry}: {witness:x?}"
            );
        }
    }
}

#[test]
fn forged_square_root_hints_fail_closed() {
    let entry = "zisklib_sqrt_fp_secp256k1";
    assert_eq!(run(entry, [4, 0, 0, 0], None).1[..4], [2, 0, 0, 0]);
    for witness in [[0; 4], P] {
        assert_eq!(
            run(entry, [4, 0, 0, 0], Some(&|hint| hint[1..].copy_from_slice(&witness))).0,
            Some(FailureReason::Trap)
        );
    }
}

#[test]
fn forged_glv_hints_fail_closed() {
    let entry = "zisklib_glv_decompose_fn_secp256k1";
    assert_eq!(run(entry, ONE, None).1, [1, 0, 0, 0, 0, 0, 0, 0]);
    for (word, value) in [(0, 2), (2, 1), (6, 1), (8, 2), (9, 2)] {
        assert_eq!(
            run(entry, ONE, Some(&|hint| hint[word] = value)).0,
            Some(FailureReason::Trap),
            "word {word}"
        );
    }
}

#[test]
fn wrong_lift_parity_fails_closed() {
    let entry = "zisklib_lift_x_secp256k1";
    let gx = [0x59f2815b16f81798, 0x029bfcdb2dce28d9, 0x55a06295ce870b07, 0x79be667ef9dcbbac];
    let gy = [0x9c47d08ffb10d4b8, 0xfd17b448a6855419, 0x5da4fbfc0e1108a8, 0x483ada7726a3c465];
    let (failure, result) = run(entry, gx, None);
    assert_eq!(failure, None);
    assert_eq!(result[..4], gx);
    assert_eq!(result[4..], gy);
    // -G.y has the same square and canonical encoding, but the wrong parity.
    let negative_gy =
        [0x63b82f6f04ef2777, 0x02e84bb7597aabe6, 0xa25b0403f1eef757, 0xb7c52588d95c3b9a];
    assert_eq!(
        run(entry, gx, Some(&|hint| hint[1..].copy_from_slice(&negative_gy))).0,
        Some(FailureReason::Trap)
    );
}

/// Reproducible input for Assembly/prover validation on a Linux x86-64 host.
#[test]
#[ignore = "writes the proof-route control and Assembly sources under target/nitro-proof-route"]
fn export_native_crypto_proof_control() {
    use zisk_common::EmuTrace;
    use zisk_core::{ziskbin, AsmGenerationMethod};
    use zisk_transpiler_common::{program2rom, program2romfile};
    use zisk_transpiler_wasm::{nitro::crypto_imports, wasm2rom_with_imports};
    use ziskemu::ZiskEmulator;

    let escaped = |hex: &str| {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|p| format!("\\{}", std::str::from_utf8(p).unwrap()))
            .collect::<String>()
    };
    let hash = "1111111122222222333333334444444455555555666666667777777788888888";
    let sig = "5424df55c64936e1ea26fe1bafc9f7becb09b4e08badb05f3d5a399b11af56ce621c59ef710fb0221066d9af00bf482dd3d49991f2afa8f745a88a5c9f58755201";
    let wat = format!(
        r#"(module
      (import "arbcrypto" "keccak256" (func $hash (param i32 i32 i32)))
      (import "arbcrypto" "ecrecovery" (func $recover (param i32 i32 i32 i32 i32) (result i32)))
      (import "wasi_snapshot_preview1" "fd_write" (func $write (param i32 i32 i32 i32) (result i32)))
      (memory 1)
      (data (i32.const 17) "{}") (data (i32.const 65) "{}")
      (func (export "_start")
        (call $hash (i32.const 65536) (i32.const 0) (i32.const 320))
        (if (call $recover (i32.const 17) (i32.const 32) (i32.const 65) (i32.const 65) (i32.const 193)) (then unreachable))
        (i32.store (i32.const 0) (i32.const 193))
        (i32.store (i32.const 4) (i32.const 65))
        (drop (call $write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 8)))))"#,
        escaped(hash),
        escaped(sig)
    );
    let wasm = wat::parse_str(&wat).unwrap();
    let rom = wasm2rom_with_imports(&wasm, &crypto_imports()).unwrap();
    let elf = ziskbin::rom_to_elf(&rom);
    let roundtrip = program2rom(&elf).unwrap();
    assert_eq!(rom.zkvmcalls, roundtrip.zkvmcalls);
    let expected = "0475f2165273d36c4603da9f4c3af30fa4f798d3d880d5338acccf1cfb8399017c3c006c2f5c884a4c42f10609551b55b214dbcfa2f407a6d13cb53f7f8c09fc85";
    for rom in [&rom, &roundtrip] {
        let result =
            ZiskEmulator::process_rom(rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>)
                .unwrap();
        assert_eq!(result[..65].iter().map(|b| format!("{b:02x}")).collect::<String>(), expected);
    }
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/nitro-proof-route");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("control.wat"), wat).unwrap();
    std::fs::write(dir.join("control.wasm"), wasm).unwrap();
    std::fs::write(dir.join("control.elf"), &elf).unwrap();
    for hints in [false, true] {
        for (name, method) in [
            ("mt", AsmGenerationMethod::AsmMinimalTraces),
            ("rh", AsmGenerationMethod::AsmRomHistogram),
            ("moh", AsmGenerationMethod::AsmMemOp),
            ("mol", AsmGenerationMethod::AsmMemOpLight),
        ] {
            let path = dir.join(format!("control-{name}-hints-{hints}.asm"));
            program2romfile(&elf, &path, method, false, false, hints).unwrap();
        }
    }
    eprintln!("Proof control: {}", dir.display());
}
