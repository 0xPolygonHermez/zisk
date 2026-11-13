//! Fork-join regions on the assembly executor: its services run each region's tasks in
//! processes of their own, and the minimal traces must still be exactly those of the Rust
//! emulator, which runs the tasks one after another.
//!
//! Three guests: `fork_join` (Rust, `ziskos::par`), with two regions of eight tasks of very
//! different lengths and a `par::join` of two closures with results of different types, which
//! read the regions' results; and two C guests of `elf-regressions/zkvm_parallel` (`zkvm_parallel.h`) with
//! one region of four tasks each: SAXPY, and tasks that change the FCSR, which lives in memory no
//! task owns. Each runs once on each executor, with the minimal traces dumped
//! (`ZISK_DUMP_MIN_TRACES`); the dumps, one line per chunk, must be identical, and so must the
//! public values. The execution's count of regions run apart checks that the regions did run
//! apart, not one task after another; a guest whose regions have one task each checks that they
//! run one task after another, and are counted so.
//!
//! A guest that panics inside a task fails on both executors; on the assembly one, a region run
//! apart cannot follow it, and the error says that the guest ended inside a task, and which.
//!
//! Linux-only, for the same reason as `asm_job_boundary`. Ignored by default: it starts the ASM
//! microservices. It needs no proving key. Run it with:
//!
//!     cargo test -p integration-tests --test asm_fork_join -- --ignored --nocapture

#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};

use zisk_asm_runner::TaskAt;
use zisk_common::io::ZiskStdin;
use zisk_prover_backend::{AsmExecClient, EmuExecClient, ExecuteClient, GuestProgram, VerboseMode};
use zisk_test_artifacts::ELF_FORK_JOIN;

/// The fork_join guest's task sizes: very different lengths, an empty task among them
const FORK_JOIN_SIZES: [u32; 8] = [3000, 50, 0, 6000, 700, 2000, 1, 3500];

fn sizes_input(sizes: &[u32]) -> ZiskStdin {
    let stdin = ZiskStdin::new();
    let bytes: Vec<u8> = sizes.iter().flat_map(|size| size.to_le_bytes()).collect();
    stdin.write_slice(&bytes);
    stdin
}

fn fork_join_input() -> ZiskStdin {
    sizes_input(&FORK_JOIN_SIZES)
}

/// The third task of the first region panics (`fork_join` fails a task of size `u32::MAX`)
fn failing_task_input() -> ZiskStdin {
    sizes_input(&[3000, 50, u32::MAX, 700])
}

/// `fork_join`'s tasks of size `u32::MAX - 2` recurse through most of their stack, and
/// those of size `u32::MAX - 1` overflow it
const DEEP: u32 = u32::MAX - 2;
const DEEPER: u32 = u32::MAX - 1;
/// Its tasks of size `u32::MAX - 3` open a region of their own, which inside a task runs inline;
/// those of size `u32::MAX - 4` read the input, which a task must not
const NESTED: u32 = u32::MAX - 3;
const READS_INPUT: u32 = u32::MAX - 4;

fn deep_tasks_input() -> ZiskStdin {
    sizes_input(&[DEEP, 50, DEEP, 3000])
}

fn stack_overflow_input() -> ZiskStdin {
    sizes_input(&[50, DEEPER, 700])
}

/// Two nested regions in each of fork_join's two regions run inline; outside a task, where the
/// guest computes what it expects, each opens a region of its own: 2 + 4 regions run apart, and
/// the join
fn nested_input() -> ZiskStdin {
    sizes_input(&[NESTED, 50, NESTED, 3000])
}

fn reads_input_input() -> ZiskStdin {
    sizes_input(&[3000, READS_INPUT, 50])
}

/// One task: both of fork_join's regions have a single task, which the service runs as any other
/// code; its join still has two
fn one_task_input() -> ZiskStdin {
    sizes_input(&[3000])
}

/// A committed guest of `elf-regressions/zkvm_parallel`
fn c_guest(name: &str) -> GuestProgram {
    let elf = format!("{}/../elf-regressions/zkvm_parallel/{name}.elf", env!("CARGO_MANIFEST_DIR"));
    GuestProgram::from_bytes(name, std::fs::read(elf).expect("committed ELF"))
}

/// Fork-join regions an execution ran apart and one task after another
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Regions {
    apart: u64,
    sequential: u64,
}

/// One execution of `program` with its minimal traces dumped to `dump`, returning the public
/// values it committed, the dump, and the regions the assembly emulator reports (none on the
/// Rust emulator).
fn run_once(
    client: &dyn ExecuteClient,
    program: &GuestProgram,
    stdin: ZiskStdin,
    dump: &Path,
) -> (Vec<u64>, String, Regions) {
    std::env::set_var("ZISK_DUMP_MIN_TRACES", dump);
    let output = client
        .execute(program, stdin, None, None)
        .unwrap_or_else(|e| panic!("execute failed: {e:#}"));
    std::env::remove_var("ZISK_DUMP_MIN_TRACES");
    let regions = output.get_executor_time().asm_execution_duration.as_ref().map_or(
        Regions { apart: 0, sequential: 0 },
        |info| Regions {
            apart: info.fork_join_regions_apart,
            sequential: info.fork_join_regions_sequential,
        },
    );
    let traces = std::fs::read_to_string(dump).expect("the execution dumps its minimal traces");
    let _ = std::fs::remove_file(dump);
    (output.get_publics().public_u64(), traces, regions)
}

fn dump_path(name: &str, executor: &str) -> PathBuf {
    std::env::temp_dir().join(format!("asm_fork_join_{}_{name}_{executor}.txt", std::process::id()))
}

/// Sets `program` up on both executors, once: the assembly client starts its services anew for a
/// program set up again
fn setup(asm: &AsmExecClient, emulator: &EmuExecClient, program: &GuestProgram) {
    asm.setup(program, false).expect("assembly setup failed");
    ExecuteClient::setup(emulator, program, false).expect("emulator setup failed");
}

/// Runs `program`, set up on both executors, on `input` on each and checks they agree, and that
/// the assembly executor ran `apart` regions apart and `sequential` one task after another
fn check(
    asm: &AsmExecClient,
    emulator: &EmuExecClient,
    program: &GuestProgram,
    name: &str,
    input: fn() -> ZiskStdin,
    (apart, sequential): (u64, u64),
) {
    let (asm_publics, asm_traces, regions) =
        run_once(asm, program, input(), &dump_path(name, "asm"));
    let (emu_publics, emu_traces, _) =
        run_once(emulator, program, input(), &dump_path(name, "emu"));

    assert_eq!(regions, Regions { apart, sequential }, "{name}: regions on the assembly executor");
    assert!(!emu_traces.is_empty(), "{name}: no chunks dumped");
    let (asm_chunks, emu_chunks) = (asm_traces.lines().count(), emu_traces.lines().count());
    assert_eq!(asm_chunks, emu_chunks, "{name}: chunks on assembly vs the emulator");
    if let Some((i, (a, e))) =
        asm_traces.lines().zip(emu_traces.lines()).enumerate().find(|(_, (a, e))| a != e)
    {
        panic!("{name}: chunk {i} differs:\n  assembly {a}\n  emulator {e}");
    }
    assert_eq!(asm_publics, emu_publics, "{name}: public values on assembly vs the emulator");
}

#[test]
#[ignore = "starts the ASM microservices; run with --ignored"]
fn regions_run_apart_give_the_emulator_traces() {
    // Regions run apart whatever the host's CPU count, which the default number of workers is.
    // Set before the services start: they read it.
    std::env::set_var("ZISK_ASM_PAR_WORKERS", "4");

    let asm = AsmExecClient::new(VerboseMode::Info, None, false)
        .expect("failed to build the assembly client");
    let emulator = EmuExecClient::new(VerboseMode::Info).expect("failed to build the emulator");

    setup(&asm, &emulator, &ELF_FORK_JOIN);
    // Its two regions and its join
    check(&asm, &emulator, &ELF_FORK_JOIN, "fork_join", fork_join_input, (3, 0));
    check(&asm, &emulator, &ELF_FORK_JOIN, "fork_join_deep", deep_tasks_input, (3, 0));
    check(&asm, &emulator, &ELF_FORK_JOIN, "fork_join_nested", nested_input, (7, 0));
    check(&asm, &emulator, &ELF_FORK_JOIN, "fork_join_one_task", one_task_input, (1, 2));
    for name in ["zkvm_parallel", "zkvm_parallel_fcsr"] {
        let program = c_guest(name);
        setup(&asm, &emulator, &program);
        check(&asm, &emulator, &program, name, ZiskStdin::new, (1, 0));
    }
}

#[test]
#[ignore = "starts the ASM microservices; run with --ignored"]
fn a_guest_that_ends_inside_a_task_fails_on_both_executors() {
    std::env::set_var("ZISK_ASM_PAR_WORKERS", "4");

    let asm = AsmExecClient::new(VerboseMode::Info, None, false)
        .expect("failed to build the assembly client");
    asm.setup(&ELF_FORK_JOIN, false).expect("assembly setup failed");
    let emulator = EmuExecClient::new(VerboseMode::Info).expect("failed to build the emulator");
    ExecuteClient::setup(&emulator, &ELF_FORK_JOIN, false).expect("emulator setup failed");

    // A task that panics, one that overflows its stack, which the stack canary turns into a panic
    // instead of a heap written over, and one that reads the input, which panics inside a task
    // The failing task is the first region's (region 0): the third, the second, the second
    for (case, input, task) in [
        ("a panic", failing_task_input as fn() -> ZiskStdin, 2),
        ("a stack overflow", stack_overflow_input, 1),
        ("reading the input", reads_input_input, 1),
    ] {
        let error = ExecuteClient::execute(&asm, &ELF_FORK_JOIN, input(), None, None)
            .map(|_| ())
            .expect_err(&format!("{case} inside a task must fail on the assembly executor"));
        let error = format!("{error:#}");
        let named = format!("inside a fork-join task ({}): ", TaskAt { region: 0, task });
        assert!(error.contains(&named), "{case}: assembly executor: {error}");

        ExecuteClient::execute(&emulator, &ELF_FORK_JOIN, input(), None, None)
            .map(|_| ())
            .expect_err(&format!("{case} inside a task must fail on the emulator too"));
    }
}
