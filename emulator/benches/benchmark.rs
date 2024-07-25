#[macro_use]
extern crate criterion;
use criterion::{/* black_box, BenchmarkId, */ Criterion};
use pprof::criterion::{Output, PProfProfiler};
use std::{fs::File /* , time::Duration */};
use ziskemu::{emulate, EmuOptions};

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
fn bench_emulate(c: &mut Criterion) {
    let guard = pprof::ProfilerGuardBuilder::default()
        .frequency(1000)
        .blocklist(&["libc", "libgcc", "pthread", "vdso"])
        .build()
        .unwrap();

    c.bench_function("Emulate", |b| {
        b.iter(|| {
            let options = EmuOptions {
                elf: Some(
                    "../riscof/riscof_work/rv64i_m/A/src/amoxor.w-01.S/dut/my.elf".to_string(),
                ),
                ..Default::default()
            };
            emulate(&options)
        })
    });

    if let Ok(report) = guard.report().build() {
        let file = File::create("flamegraph.svg").unwrap();
        report.flamegraph(file).unwrap();
    };
}

criterion_group! {
    name = benches;
    config = Criterion::default().with_profiler(PProfProfiler::new(/*100*/1, Output::Flamegraph(None)));
    //targets = bench_emulate, bench, bench_group
    targets = bench_emulate
}
criterion_main!(benches);
