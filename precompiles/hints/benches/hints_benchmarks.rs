use anyhow::Result;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use precompiles_hints::HintsProcessor;
use std::hint::black_box;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use zisk_common::io::StreamSink;

struct BenchSink {
    received: Arc<Mutex<Vec<Vec<u64>>>>,
}

impl StreamSink for BenchSink {
    fn submit(&self, processed: Vec<u64>) -> Result<()> {
        self.received.lock().unwrap().push(processed);
        Ok(())
    }
}

fn make_header(hint_type: u32, length: u32) -> u64 {
    ((hint_type as u64) << 32) | (length as u64)
}

fn parallel_speedup_benchmark(c: &mut Criterion) {
    // Define custom hints with known processing times (use high values to avoid built-in conflicts)
    const FAST_HINT: u32 = 0x7FFF_0000; // 1ms
    const MEDIUM_HINT: u32 = 0x7FFF_0001; // 5ms
    const SLOW_HINT: u32 = 0x7FFF_0002; // 10ms

    // Test configuration
    const NUM_FAST: usize = 100;
    const NUM_MEDIUM: usize = 50;
    const NUM_SLOW: usize = 20;

    let mut group = c.benchmark_group("parallel_speedup");
    group.sample_size(10); // Reduce sample size for slower benchmarks

    let thread_counts = [1, 2, 4, 8, 16];

    for &num_threads in &thread_counts {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}_threads", num_threads)),
            &num_threads,
            |b, &threads| {
                b.iter(|| {
                    let received = Arc::new(Mutex::new(Vec::new()));
                    let received_clone = received.clone();
                    let sink = BenchSink { received: received_clone };

                    let p = HintsProcessor::builder(sink)
                        .num_threads(threads)
                        .custom_hint(FAST_HINT, |data: &[u64]| -> Result<Vec<u64>> {
                            thread::sleep(Duration::from_millis(1));
                            Ok(vec![data[0] + 1])
                        })
                        .custom_hint(MEDIUM_HINT, |data: &[u64]| -> Result<Vec<u64>> {
                            thread::sleep(Duration::from_millis(5));
                            Ok(vec![data[0] + 2])
                        })
                        .custom_hint(SLOW_HINT, |data: &[u64]| -> Result<Vec<u64>> {
                            thread::sleep(Duration::from_millis(10));
                            Ok(vec![data[0] + 3])
                        })
                        .build()
                        .unwrap();

                    let mut data = Vec::new();
                    let mut hint_idx = 0;

                    for _ in 0..NUM_FAST {
                        data.push(make_header(FAST_HINT, 1));
                        data.push(hint_idx);
                        hint_idx += 1;
                    }

                    for _ in 0..NUM_MEDIUM {
                        data.push(make_header(MEDIUM_HINT, 1));
                        data.push(hint_idx);
                        hint_idx += 1;
                    }

                    for _ in 0..NUM_SLOW {
                        data.push(make_header(SLOW_HINT, 1));
                        data.push(hint_idx);
                        hint_idx += 1;
                    }

                    p.process_hints(black_box(&data), false).unwrap();
                    p.wait_for_completion().unwrap();

                    let results = received.lock().unwrap();
                    assert_eq!(results.len(), NUM_FAST + NUM_MEDIUM + NUM_SLOW);
                });
            },
        );
    }

    group.finish();
}

fn microsecond_hints_benchmark(c: &mut Criterion) {
    const ULTRA_FAST: u32 = 0x7FFF_0010; // 10µs
    const VERY_FAST: u32 = 0x7FFF_0011; // 50µs
    const FAST: u32 = 0x7FFF_0012; // 100µs
    const NUM_HINTS: usize = 1000;

    let mut group = c.benchmark_group("microsecond_hints");
    group.sample_size(50);

    let test_cases = vec![
        ("ultra_fast_10us", ULTRA_FAST, 10),
        ("very_fast_50us", VERY_FAST, 50),
        ("fast_100us", FAST, 100),
    ];

    for (name, hint_code, micros) in test_cases {
        group.bench_function(name, |b| {
            b.iter(|| {
                let received = Arc::new(Mutex::new(Vec::new()));
                let received_clone = received.clone();
                let sink = BenchSink { received: received_clone };

                let p = HintsProcessor::builder(sink)
                    .num_threads(16)
                    .custom_hint(hint_code, move |data: &[u64]| -> Result<Vec<u64>> {
                        thread::sleep(Duration::from_micros(micros as u64));
                        Ok(vec![data[0] + 1])
                    })
                    .build()
                    .unwrap();

                let mut data = Vec::new();
                for i in 0..NUM_HINTS {
                    data.push(make_header(hint_code, 1));
                    data.push(i as u64);
                }

                p.process_hints(black_box(&data), false).unwrap();
                p.wait_for_completion().unwrap();

                let results = received.lock().unwrap();
                assert_eq!(results.len(), NUM_HINTS);
            });
        });
    }

    group.finish();
}

fn workload_patterns_benchmark(c: &mut Criterion) {
    const VERY_FAST: u32 = 0x7FFF_0020; // 0.5ms
    const FAST: u32 = 0x7FFF_0021; // 2ms
    const MEDIUM: u32 = 0x7FFF_0022; // 5ms
    const SLOW: u32 = 0x7FFF_0023; // 10ms
    const VERY_SLOW: u32 = 0x7FFF_0024; // 20ms

    let mut group = c.benchmark_group("workload_patterns");
    group.sample_size(10);

    let patterns = vec![
        ("uniform_fast", vec![(FAST, 100)]),
        ("uniform_slow", vec![(SLOW, 50)]),
        ("mixed_balanced", vec![(FAST, 40), (MEDIUM, 20), (SLOW, 10)]),
        ("skewed_fast", vec![(VERY_FAST, 80), (SLOW, 10), (VERY_SLOW, 10)]),
        ("heavy_tail", vec![(FAST, 50), (VERY_SLOW, 5)]),
    ];

    for (name, hints) in patterns {
        group.bench_function(name, |b| {
            b.iter(|| {
                let received = Arc::new(Mutex::new(Vec::new()));
                let received_clone = received.clone();
                let sink = BenchSink { received: received_clone };

                let p = HintsProcessor::builder(sink)
                    .num_threads(8)
                    .custom_hint(VERY_FAST, |data: &[u64]| -> Result<Vec<u64>> {
                        thread::sleep(Duration::from_micros(500));
                        Ok(vec![data[0] + 1])
                    })
                    .custom_hint(FAST, |data: &[u64]| -> Result<Vec<u64>> {
                        thread::sleep(Duration::from_millis(2));
                        Ok(vec![data[0] + 1])
                    })
                    .custom_hint(MEDIUM, |data: &[u64]| -> Result<Vec<u64>> {
                        thread::sleep(Duration::from_millis(5));
                        Ok(vec![data[0] + 1])
                    })
                    .custom_hint(SLOW, |data: &[u64]| -> Result<Vec<u64>> {
                        thread::sleep(Duration::from_millis(10));
                        Ok(vec![data[0] + 1])
                    })
                    .custom_hint(VERY_SLOW, |data: &[u64]| -> Result<Vec<u64>> {
                        thread::sleep(Duration::from_millis(20));
                        Ok(vec![data[0] + 1])
                    })
                    .build()
                    .unwrap();

                let mut data = Vec::new();
                let mut idx = 0;
                for (hint_code, count) in &hints {
                    for _ in 0..*count {
                        data.push(make_header(*hint_code, 1));
                        data.push(idx);
                        idx += 1;
                    }
                }

                p.process_hints(black_box(&data), false).unwrap();
                p.wait_for_completion().unwrap();

                let total_hints: usize = hints.iter().map(|(_, count)| count).sum();
                let results = received.lock().unwrap();
                assert_eq!(results.len(), total_hints);
            });
        });
    }

    group.finish();
}

fn noop_throughput_benchmark(c: &mut Criterion) {
    struct NullSink;

    impl StreamSink for NullSink {
        fn submit(&self, _processed: Vec<u64>) -> Result<()> {
            Ok(())
        }
    }

    let mut group = c.benchmark_group("noop_throughput");
    group.sample_size(20);

    let hint_counts = [1000, 10000, 100000];

    // Pass-through hint code (bit 31 set = pass-through, no computation needed)
    const PASSTHROUGH_HINT: u32 = 0x8000_1000;

    for &count in &hint_counts {
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &num_hints| {
            b.iter(|| {
                let p = HintsProcessor::builder(NullSink).num_threads(32).build().unwrap();

                let mut data = Vec::with_capacity(num_hints * 2);
                for i in 0..num_hints {
                    data.push(make_header(PASSTHROUGH_HINT, 1));
                    data.push(i as u64);
                }

                p.process_hints(black_box(&data), false).unwrap();
                p.wait_for_completion().unwrap();
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    parallel_speedup_benchmark,
    microsecond_hints_benchmark,
    workload_patterns_benchmark,
    noop_throughput_benchmark
);
criterion_main!(benches);
