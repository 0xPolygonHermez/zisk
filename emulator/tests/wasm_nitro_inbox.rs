//! Inbox accumulator controls. Build runtime/build.py before running ignored tests.
use std::io::Write;
use std::process::{Command, Stdio};
use zisk_common::EmuTrace;
use zisk_transpiler_wasm::nitro;
use ziskemu::{EmuOptions, ZiskEmulator};

struct Witness {
    seq_start: u64,
    delayed_start: u64,
    sequencer: Vec<(u64, Vec<u8>)>,
    delayed: Vec<(u64, Vec<u8>)>,
}

fn sequencer(number: u64, checkpoint: u64) -> (u64, Vec<u8>) {
    let mut bytes = vec![number as u8; 32];
    bytes.extend_from_slice(&checkpoint.to_be_bytes());
    bytes.extend_from_slice(b"batch");
    (number, bytes)
}

fn delayed(number: u64) -> (u64, Vec<u8>) {
    let mut bytes = vec![number as u8; 113];
    bytes.extend_from_slice(b"delayed");
    (number, bytes)
}

fn window(delayed_start: u64, checkpoints: &[u64], numbers: &[u64]) -> Witness {
    Witness {
        seq_start: 10,
        delayed_start,
        sequencer: checkpoints
            .iter()
            .enumerate()
            .map(|(i, &count)| sequencer(10 + i as u64, count))
            .collect(),
        delayed: numbers.iter().map(|&number| delayed(number)).collect(),
    }
}

fn state(batch: u64, position: u64) -> Vec<u8> {
    let mut bytes = vec![0x11; 32];
    bytes.extend_from_slice(&[0x22; 32]);
    bytes.extend_from_slice(&batch.to_le_bytes());
    bytes.extend_from_slice(&position.to_le_bytes());
    bytes
}

impl Witness {
    fn encode(&self) -> Vec<u8> {
        let mut payload = b"NZKWIT01".to_vec();
        payload.extend(state(10, 3));
        payload.extend_from_slice(&[0x55; 32]);
        payload.extend_from_slice(&[0x66; 32]);
        payload.extend_from_slice(&self.seq_start.to_le_bytes());
        payload.extend_from_slice(&self.delayed_start.to_le_bytes());
        payload.extend_from_slice(&0u32.to_le_bytes());
        payload.extend_from_slice(&(self.sequencer.len() as u32).to_le_bytes());
        payload.extend_from_slice(&(self.delayed.len() as u32).to_le_bytes());
        for (number, bytes) in self.sequencer.iter().chain(&self.delayed) {
            payload.extend_from_slice(&number.to_le_bytes());
            payload.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            payload.extend_from_slice(bytes);
        }
        let mut framed = (payload.len() as u64).to_le_bytes().to_vec();
        framed.extend(payload);
        framed.resize(framed.len().next_multiple_of(8), 0);
        framed
    }

    fn reads(&self) -> Vec<(bool, u64, u32, u32)> {
        self.sequencer
            .iter()
            .map(|(number, _)| (false, *number, 0, 32))
            .chain(self.delayed.iter().map(|(number, _)| (true, *number, 0, 32)))
            .collect()
    }
}

fn module(reads: &[(bool, u64, u32, u32)]) -> Vec<u8> {
    let calls: String = reads.iter().map(|&(is_delayed, number, offset, copied)| {
        let name = if is_delayed { "$delayed" } else { "$seq" };
        format!("(call $assert (i32.eq (call {name} (i64.const {}) (i32.const {offset}) (i32.const 64)) (i32.const {copied})))\n", number as i64)
    }).collect();
    wat::parse_str(format!(
        r#"(module
        (import "wavmio" "readInboxMessage" (func $seq (param i64 i32 i32) (result i32)))
        (import "wavmio" "readDelayedInboxMessage" (func $delayed (param i64 i32 i32) (result i32)))
        (import "wavmio" "setGlobalStateU64" (func $set64 (param i32 i64)))
        (import "wasi_snapshot_preview1" "proc_exit" (func $exit (param i32)))
        (memory 1)
        (func $assert (param $value i32) (if (i32.eqz (local.get $value)) (then unreachable)))
        (func (export "_start")
            {calls}
            (call $set64 (i32.const 0) (i64.const 14))
            (call $set64 (i32.const 1) (i64.const 0))
            (call $exit (i32.const 0))
            unreachable))"#
    ))
    .unwrap()
}

fn execute(wasm: &[u8], witness: &Witness) -> Result<Vec<u8>, String> {
    let elf = std::fs::read("../target/nitro-runtime/nitro-runtime.elf")
        .expect("run transpilers/wasm/runtime/build.py first");
    let rom = nitro::wasm2rom(wasm, &elf).map_err(|error| error.to_string())?;
    let options = EmuOptions { max_steps: 30_000_000, ..Default::default() };
    ZiskEmulator::process_rom(&rom, &witness.encode(), &options, None::<fn(EmuTrace)>)
        .map_err(|error| error.to_string())
}

fn expected(wasm: &[u8], seq_claim: &str, delayed_claim: &str) -> Vec<u8> {
    // Python hashlib is independent of compiler hashing and VM crypto. Claim vectors
    // below use prepare-witness.py's independent Keccak oracle and the OSP byte layout.
    let mut child = Command::new("python3")
        .args(["-c", "import hashlib,sys;sys.stdout.buffer.write(hashlib.sha256(sys.stdin.buffer.read()).digest())"])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().expect("python3 for independent SHA-256");
    child.stdin.take().unwrap().write_all(wasm).unwrap();
    let hashed = child.wait_with_output().unwrap();
    assert!(hashed.status.success());
    assert_eq!(hashed.stdout.len(), 32);
    let mut output = hashed.stdout;
    output.extend(state(10, 3));
    output.extend(state(14, 0));
    for claim in [seq_claim, delayed_claim] {
        output.extend(
            claim
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()),
        );
    }
    assert_eq!(output.len(), 256);
    output
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn multiple_delayed_checkpoints_finalize_all_256_claim_bytes() {
    let mut witness = window(7, &[7, 8, 8, 9], &[7, 8]);
    let ordered = witness.reads();
    // Optional messages outside the consumed window must not affect either claim.
    witness.sequencer.insert(0, sequencer(9, 100));
    witness.sequencer.push(sequencer(14, 100));
    witness.delayed.insert(0, delayed(6));
    witness.delayed.push(delayed(9));
    let reordered = vec![
        (false, 13, 32, 13),
        (true, 8, 96, 24),
        (false, 10, 0, 32),
        (true, 7, 0, 32),
        (false, 12, 0, 32),
        (false, 11, 0, 32),
        (true, 8, 0, 32),
        (false, 13, 0, 32),
        (true, 7, 32, 32),
    ];
    for reads in [ordered, reordered] {
        let wasm = module(&reads);
        assert_eq!(
            execute(&wasm, &witness).unwrap(),
            expected(
                &wasm,
                "4b2844598ca14e3045e4f9831051ea86eae89d37cea5740b3ae74bb1f329319a",
                "ae88c07bb601f53c3b9d256d777f461804fc623ffdb2d121d32f14450aef84e3"
            )
        );
    }
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn zero_checkpoint_uses_zero_accumulator() {
    let witness = window(0, &[0], &[]);
    let wasm = module(&witness.reads());
    assert_eq!(
        execute(&wasm, &witness).unwrap(),
        expected(
            &wasm,
            "dc035b929d41478e29aa3d0b09105043e389edbad7ce3910b79e6faeb5d14661",
            "40911a7783cffc42d4e2767db488e04de986fa096e1476e134233df75fc5a76b"
        )
    );
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn maximum_delayed_count_without_overflow_is_accepted() {
    let witness = window(u64::MAX - 1, &[u64::MAX], &[u64::MAX - 1]);
    let wasm = module(&witness.reads());
    assert_eq!(
        execute(&wasm, &witness).unwrap(),
        expected(
            &wasm,
            "6acd3d614ae4895acc8a3d37c95636c8cbef71b67eca813d94abc7dc65ffab11",
            "ae1dc8df6030bfbda191f2f3963d689906605b12f61224ec3630da54a938ce69"
        )
    );
}

fn assert_traps(witness: &Witness, reads: &[(bool, u64, u32, u32)]) {
    let error = execute(&module(reads), witness).unwrap_err();
    assert!(error.to_lowercase().contains("trap"), "{error}");
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn inbox_gaps_trap_inside_vm() {
    let mut seq_gap = window(7, &[7, 7], &[]);
    seq_gap.sequencer[1].0 = 12;
    let delayed_gap = window(7, &[10], &[7, 9]);
    for witness in [seq_gap, delayed_gap] {
        assert_traps(&witness, &witness.reads());
    }
    // Supplied but unread delayed bytes cannot fill a hole in the consumed range.
    let witness = window(7, &[10], &[7, 8, 9]);
    assert_traps(&witness, &[(false, 10, 0, 32), (true, 7, 0, 32), (true, 9, 0, 32)]);
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn out_of_range_or_decreasing_checkpoints_trap_inside_vm() {
    for witness in [
        window(7, &[6], &[]),
        window(7, &[0], &[]),
        window(7, &[8], &[]),
        window(7, &[8, 7], &[7]),
        window(0, &[1, 0], &[0]),
    ] {
        assert_traps(&witness, &witness.reads());
    }
    let witness = window(7, &[8], &[7]);
    assert_traps(&witness, &[(false, 10, 0, 32)]);
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn consumed_delayed_end_requires_a_sequencer_checkpoint() {
    for witness in [window(7, &[8], &[7, 8]), window(7, &[], &[7])] {
        assert_traps(&witness, &witness.reads());
    }
}

#[test]
#[ignore = "requires built native runtime ELF and float feature"]
fn inbox_count_overflow_traps_inside_vm() {
    let mut seq_overflow = window(7, &[7], &[]);
    seq_overflow.seq_start = u64::MAX;
    seq_overflow.sequencer[0].0 = u64::MAX;
    for witness in [seq_overflow, window(u64::MAX, &[u64::MAX], &[u64::MAX])] {
        assert_traps(&witness, &witness.reads());
    }
}
