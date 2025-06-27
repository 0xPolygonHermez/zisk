#[macro_use]
extern crate criterion;
use criterion::Criterion;
//use std::{fs::File /* , time::Duration */};
use zisk_common::EmuTrace;
use zisk_core::{Riscv2zisk, ZiskRom};
use ziskemu::{EmuOptions, Emulator, ZiskEmulator};

// Thanks to the example provided by @jebbow in his article
// https://www.jibbow.com/posts/criterion-flamegraphs/

/*
fn fibonacci(n: u64) -> u64 {
    match n {
        0 | 1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn bench(c: &mut Criterion) {
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    c.bench_function("Fibonacci", |b| b.iter(|| fibonacci(black_box(20))));

    if let Ok(report) = guard.report().build() {
        let file = File::create("flamegraph.svg").unwrap();
        report.flamegraph(file).unwrap();
    };
}

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fibonacci Sizes");

    for s in &[1, 10 /* , 100, 1000 */] {
        group.bench_with_input(BenchmarkId::from_parameter(s), s, |b, s| {
            b.iter(|| fibonacci(black_box(*s)))
        });
    }
}
*/
fn dummy_callback(_v: EmuTrace) {}

fn bench_emulate(c: &mut Criterion) {
    /*let guard = pprof::ProfilerGuardBuilder::default()
    .frequency(1000)
    .blocklist(&["libc", "libgcc", "pthread", "vdso"])
    .build()
    .unwrap();*/

    c.bench_function("Emulate", |b| {
        b.iter(|| {
            let options = EmuOptions {
                elf: Some("./benches/data/my.elf".to_string()),
                inputs: Some("./benches/data/input.bin".to_string()),
                log_metrics: true,
                ..Default::default()
            };
            let emulator = ZiskEmulator;
            emulator.emulate(&options, Some(Box::new(dummy_callback))).unwrap();
        })
    });

    /*if let Ok(report) = guard.report().build() {
        let file = File::create("emulate.svg").unwrap();
        report.flamegraph(file).unwrap();
    };*/
}

fn bench_riscv2zisk(c: &mut Criterion) {
    /*let guard = pprof::ProfilerGuardBuilder::default()
    .frequency(1000)
    .blocklist(&["libc", "libgcc", "pthread", "vdso"])
    .build()
    .unwrap();*/

    c.bench_function("Riscv2zisk", |b| {
        b.iter(|| {
            // Convert the ELF file to ZisK ROM
            let elf_file = "./benches/data/my.elf".to_string();
            let _rom: ZiskRom = {
                // Create an instance of the RISCV -> ZisK program converter
                let rv2zk = Riscv2zisk::new(elf_file.clone());

                // Convert program to rom
                let result = rv2zk.run();
                if result.is_err() {
                    panic!("Application error: {}", result.err().unwrap());
                }

                // Get the result
                result.unwrap()
            };
        });
    });

    /*if let Ok(report) = guard.report().build() {
        let file = File::create("riscv2zisk.svg").unwrap();
        report.flamegraph(file).unwrap();
    };*/
}

fn bench_process_rom(c: &mut Criterion) {
    /*let guard = pprof::ProfilerGuardBuilder::default()
    .frequency(1000)
    .blocklist(&["libc", "libgcc", "pthread", "vdso"])
    .build()
    .unwrap();*/

    c.bench_function("Process ROM", |b| {
        // Convert the ELF file to ZisK ROM
        let elf_file = "./benches/data/my.elf".to_string();
        let rom: ZiskRom = {
            // Create an instance of the RISCV -> ZisK program converter
            let rv2zk = Riscv2zisk::new(elf_file.clone());

            // Convert program to rom
            let result = rv2zk.run();
            if result.is_err() {
                panic!("Application error: {}", result.err().unwrap());
            }

            // Get the result
            result.unwrap()
        };

        let options = EmuOptions {
            elf: Some("./benches/data/my.elf".to_string()),
            inputs: Some("./benches/data/input.bin".to_string()),
            log_metrics: true,
            ..Default::default()
        };

        //let input: Vec<u8> = Vec::new();
        let input_file: String = "./benches/data/input.bin".to_string();
        let input: Vec<u8> = {
            // Read input data from the provided input path
            let path = std::path::PathBuf::from(input_file);
            std::fs::read(path).expect("Could not read input file ")
        };

        b.iter(|| {
            let _ =
                ZiskEmulator::process_rom(&rom, &input, &options, None::<Box<dyn Fn(EmuTrace)>>);
        });
    });

    /*if let Ok(report) = guard.report().build() {
        let file = File::create("process_rom.svg").unwrap();
        report.flamegraph(file).unwrap();
    };*/
}

fn bench_process_rom_callback(c: &mut Criterion) {
    /*let guard = pprof::ProfilerGuardBuilder::default()
    .frequency(1000)
    .blocklist(&["libc", "libgcc", "pthread", "vdso"])
    .build()
    .unwrap();*/

    c.bench_function("Process ROM with callback", |b| {
        // Convert the ELF file to ZisK ROM
        //let elf_file =
        // "../riscof/riscof_work/rv64i_m/A/src/amoxor.w-01.S/dut/my.elf".to_string();
        let elf_file = "./benches/data/my.elf".to_string();
        let zisk_rom: ZiskRom = {
            // Create an instance of the RISCV -> ZisK program converter
            let rv2zk = Riscv2zisk::new(elf_file.clone());

            // Convert program to rom
            let result = rv2zk.run();
            if result.is_err() {
                panic!("Application error: {}", result.err().unwrap());
            }

            // Get the result
            result.unwrap()
        };

        let options = EmuOptions {
            elf: Some("./benches/data/my.elf".to_string()),
            inputs: Some("./benches/data/input.bin".to_string()),
            log_metrics: true,
            chunk_size: Some(1000000),
            ..Default::default()
        };

        //let input: Vec<u8> = Vec::new();
        let input_file: String = "./benches/data/input.bin".to_string();
        let input: Vec<u8> = {
            // Read input data from the provided input path
            let path = std::path::PathBuf::from(input_file);
            std::fs::read(path).expect("Could not read input file ")
        };

        b.iter(|| {
            let _ = ZiskEmulator::process_rom(
                &zisk_rom,
                &input,
                &options,
                Some(Box::new(dummy_callback)),
            );
        });
    });

    /*if let Ok(report) = guard.report().build() {
        let file = File::create("process_rom_callback.svg").unwrap();
        report.flamegraph(file).unwrap();
    };*/
}

criterion_group! {
    name = benches;
    config = Criterion::default().significance_level(0.1).sample_size(10);//.with_profiler(PProfProfiler::new(/*100*/1, Output::Flamegraph(None)));
    //targets = bench_emulate, bench, bench_group
    targets = bench_emulate, bench_riscv2zisk, bench_process_rom, bench_process_rom_callback
}
criterion_main!(benches);
