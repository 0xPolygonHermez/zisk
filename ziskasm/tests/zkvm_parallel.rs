//! End-to-end tests of the fork-join driver (`zisklib/zkvm/parallel.zisk`) on the Rust emulator,
//! which runs the tasks one after another and checks, at every pass through the region's task
//! loop, the invariant a parallel emulation relies on: each pass starts on a chunk boundary and in
//! the state of the first pass but for the task index. A region that breaks it panics.

use zisk_common::EmuTrace;
use zisk_core::FCSR;
use ziskasm::{assemble, parser::parse_program};
use ziskemu::{EmuOptions, ZiskEmulator};

/// The library sources the driver needs (its fcall ids and FREE_INPUT), the driver, and the
/// test's tasks, in one program
const LIBRARY: [&str; 3] = [
    include_str!("../zisklib/fcall.zisk"),
    include_str!("../zisklib/mem.zisk"),
    include_str!("../zisklib/zkvm/parallel.zisk"),
];

/// Tasks, following the RISC-V calling convention: `entry(a0 = arg, a1 = out)`. Like compiled
/// code, they leave junk in caller-saved and virtual registers, which the driver must clear.
/// They return to `par_ret`, which stands for the guest's `zkvm_parallel_ret` thunk: a static
/// jump back into the driver.
const TASKS: &str = "
par_ret:
\tjump(ziskasm_zkvm_parallel_ret)

; *out = 1 + 2 + .. + arg (arg >= 1), 3 steps per term
task_sum:
\tcopyb(0, 0) -> r5
tsum_loop:
\tadd(r5, r10) -> r5
\tsub(r10, 1) -> r10
\tltu(0, r10), j(tsum_loop)
\tcopyb(r11, r5) -> 8[a + 0]
\tcopyb(0, 0x1111) -> r12
\tcopyb(0, 0x2222) -> r29
\tcopyb(0, 0x3333) -> r33
\tret

; *out = arg + 7, through the task's own stack
task_stack:
\tsub(r2, 16) -> r2
\tcopyb(r2, r10) -> 8[a + 0]
\tcopyb(r2, 8[a + 0]) -> r5
\tadd(r5, 7) -> r5
\tcopyb(r11, r5) -> 8[a + 0]
\tadd(r2, 16) -> r2
\tret

; writes nothing
task_empty:
\tret

; *out = the FCSR the task starts with; then it leaves arg in the FCSR, as a floating-point
; operation leaves its flags there
task_fcsr:
\tcopyb(0, [0xa0408018]) -> r5
\tcopyb(r11, r5) -> 8[a + 0]
\tcopyb(0, r10) -> [0xa0408018]
\tret

; *out = arg, and changes a callee-saved register without restoring it, which the calling
; convention forbids: the next pass starts in another state
task_bad:
\tcopyb(r11, r10) -> 8[a + 0]
\tcopyb(0, 0x5555) -> r19
\tret
";

/// A task: its entry label and argument
type Task = (&'static str, u64);

/// The `main` that runs `regions` one after another through the driver and outputs every task's
/// out, in order. Each task owns a 512-byte block: its out at the bottom, its stack above it.
fn main_source(regions: &[&[Task]]) -> String {
    let total: usize = regions.iter().map(|r| r.len()).sum();
    let mut src = format!(
        "define OUTPUT_ADDR 0xa0410000\n\
         u64 PAR_TASKS[{}]\n\
         u64 PAR_MEM[{}]\n\
         main:\n\
         \tcopyb(0, r1) -> r18\n",
        6 * total.max(1),
        64 * total.max(1)
    );
    let mut k = 0;
    for region in regions {
        src += "\tcopyb(0, PAR_TASKS) -> r20\n";
        for (i, (entry, arg)) in region.iter().enumerate() {
            let word = 48 * i;
            let (start, end) = (512 * (k + i), 512 * (k + i + 1));
            src += &format!(
                "\tcopyb(r20, {entry}) -> 8[a + {}]\n\
                 \tcopyb(r20, {arg}) -> 8[a + {}]\n\
                 \tcopyb(r20, PAR_MEM + {start}) -> 8[a + {}]\n\
                 \tcopyb(r20, PAR_MEM + {end}) -> 8[a + {}]\n\
                 \tcopyb(r20, PAR_MEM + {start}) -> 8[a + {}]\n\
                 \tcopyb(r20, PAR_MEM + {end}) -> 8[a + {}]\n",
                word,
                word + 8,
                word + 16,
                word + 24,
                word + 32,
                word + 40
            );
        }
        src += &format!(
            "\tcopyb(0, PAR_TASKS) -> r10\n\tcopyb(0, {}) -> r11\n\tcopyb(0, par_ret) -> r12\n\
             \tcall ziskasm_zkvm_parallel\n",
            region.len()
        );
        k += region.len();
    }
    src += "\tcopyb(0, OUTPUT_ADDR) -> r20\n\tcopyb(0, PAR_MEM) -> r21\n";
    for i in 0..total {
        src += &format!(
            "\tcopyb(r21, 8[a + {}]) -> r5\n\tcopyb(r20, r5) -> 8[a + {}]\n",
            512 * i,
            8 * i
        );
    }
    src += "\tcopyb(0, r18) -> r1\n\tret\n";
    src
}

/// Runs `regions` through the driver and returns every task's out, in order
fn run(regions: &[&[Task]]) -> Vec<u64> {
    let total: usize = regions.iter().map(|r| r.len()).sum();
    run_main(&main_source(regions), total)
}

/// Runs the program of `main` with the tasks and the library, and returns its first `words`
/// output words
fn run_main(main: &str, words: usize) -> Vec<u64> {
    let src = [main, TASKS].into_iter().chain(LIBRARY).collect::<String>();
    let program = parse_program(&src, "zkvm_parallel").expect("parse should succeed");
    let rom = assemble(&program).expect("assembly should succeed");
    let out = ZiskEmulator::process_rom(&rom, &[], &EmuOptions::default(), None::<fn(EmuTrace)>)
        .expect("emulation should succeed");
    out[..words * 8].chunks(8).map(|w| u64::from_le_bytes(w.try_into().unwrap())).collect()
}

/// 1 + 2 + .. + n
fn sum(n: u64) -> u64 {
    n * (n + 1) / 2
}

#[test]
fn runs_the_tasks_in_order_each_on_its_own_stack() {
    let out = run(&[&[("task_sum", 10), ("task_stack", 35), ("task_empty", 0), ("task_sum", 3)]]);
    assert_eq!(out, [sum(10), 42, 0, sum(3)]);
}

#[test]
fn a_task_longer_than_a_chunk() {
    // 3 steps per term: 300,000 steps, more than one 2^18-step chunk
    let out = run(&[&[("task_sum", 100_000), ("task_stack", 1)]]);
    assert_eq!(out, [sum(100_000), 8]);
}

#[test]
fn a_region_without_tasks() {
    assert!(run(&[&[]]).is_empty());
}

#[test]
fn regions_one_after_another() {
    let out = run(&[&[("task_sum", 4), ("task_stack", 1)], &[("task_stack", 2)], &[]]);
    assert_eq!(out, [sum(4), 8, 9]);
}

#[test]
fn many_short_tasks_pad_from_any_point_of_a_chunk() {
    // Tasks of 0 to ~3,000 steps leave the padding a different remainder each time
    let tasks: Vec<Task> = (1..=12).map(|i| ("task_sum", i * i * 7)).collect();
    let out = run(&[&tasks]);
    assert_eq!(out, (1..=12).map(|i| sum(i * i * 7)).collect::<Vec<_>>());
}

#[test]
#[should_panic(expected = "in another state than the first pass: r19")]
fn a_task_that_breaks_the_calling_convention_is_caught() {
    run(&[&[("task_bad", 1), ("task_stack", 1)]]);
}

/// The FCSR the caller leaves before the region in the FCSR test
const REGION_FCSR: u64 = 0x60;

#[test]
fn every_task_starts_with_the_regions_fcsr_and_the_caller_gets_it_back() {
    // Each task reports the FCSR it starts with and leaves another one, as floating-point code
    // does; the caller sets it before the region and outputs it after, after the tasks' outs
    let tasks: &[Task] = &[("task_fcsr", 0x1f), ("task_fcsr", 0x80), ("task_fcsr", 0x01)];
    let main = main_source(&[tasks])
        .replacen(
            "main:\n",
            &format!("main:\n\tcopyb(0, {REGION_FCSR}) -> r5\n\tcopyb(0, r5) -> [{FCSR:#x}]\n"),
            1,
        )
        .replacen(
            "\tcopyb(0, r18) -> r1\n\tret\n",
            &format!(
                "\tcopyb(0, [{FCSR:#x}]) -> r5\n\tcopyb(r20, r5) -> 8[a + {}]\n\tcopyb(0, r18) -> r1\n\tret\n",
                8 * tasks.len()
            ),
            1,
        );
    assert_eq!(run_main(&main, tasks.len() + 1), [REGION_FCSR; 4]);
}

#[test]
fn the_driver_restores_the_fcsr_at_its_address() {
    let driver = include_str!("../zisklib/zkvm/parallel.zisk");
    let line = driver.lines().find(|l| l.starts_with("define ZPAR_FCSR ")).expect("ZPAR_FCSR");
    let value = line["define ZPAR_FCSR ".len()..].split(';').next().unwrap().trim();
    assert_eq!(u64::from_str_radix(value.trim_start_matches("0x"), 16).unwrap(), FCSR);
    // ... which the test task reads and writes too
    assert!(TASKS.contains(&format!("[{FCSR:#x}]")));
}
