//! Generates the expected states for every smoke case and the matching input/publics files.

use std::{env, fs, path::PathBuf};
use zisk_koala_poseidon2_foundation::{Parameters, State, MODULUS};

fn packed(state: State) -> [u64; 8] {
    core::array::from_fn(|i| u64::from(state[2 * i]) | (u64::from(state[2 * i + 1]) << 32))
}

fn framed_input(case: u8) -> [u8; 16] {
    let mut input = [0; 16];
    input[..8].copy_from_slice(&1_u64.to_le_bytes());
    input[8] = case;
    input
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let parameters = Parameters::pinned();
    let initial = core::array::from_fn(|i| match i % 4 {
        0 => 0,
        1 => 1,
        2 => MODULUS - 1,
        _ => i as u32,
    });
    let mut outputs = vec![packed(initial); 1026];
    let mut state = initial;
    for output in outputs.iter_mut().skip(1) {
        state = parameters.permute(state).unwrap();
        *output = packed(state);
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(
        out.join("expected.rs"),
        format!("const EXPECTED: [[u64; 8]; 1026] = {outputs:?};\n"),
    )
    .unwrap();
    for (case, calls) in [(0, 0_u32), (1, 1), (8, 8), (9, 9), (0xfc, 1024), (0xfd, 1025)] {
        fs::write(out.join(format!("input-{calls}.bin")), framed_input(case)).unwrap();
        let mut publics = calls.to_le_bytes().to_vec();
        for word in outputs[calls as usize] {
            publics.extend_from_slice(&word.to_le_bytes());
        }
        fs::write(out.join(format!("expected-{calls}.bin")), publics).unwrap();
    }
    for lane in 0..16_u8 {
        fs::write(out.join(format!("input-invalid-lane-{lane}.bin")), framed_input(0x80 + lane))
            .unwrap();
    }
    for offset in 1..8_u8 {
        fs::write(out.join(format!("input-misaligned-{offset}.bin")), framed_input(0xa0 + offset))
            .unwrap();
    }
}
