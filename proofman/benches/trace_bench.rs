use proofman::trace;

use criterion::{criterion_group, criterion_main, Criterion};

fn with_trace(c: &mut Criterion) {
    const STRIDE: usize = 32;

    trace!(BinaryTrace { a: [u64; STRIDE] });

    let mut binary_trace = BinaryTrace::new(1 << 23);

    for i in 0..(1 << 23) {
        for j in 0..(1 << 3) {
            binary_trace.a[j][i] = (i * STRIDE + j) as u64;
        }
    }

    let mut a = 0u64;
    c.bench_function("access_using_trace", |b| {
        b.iter(|| {
            for i in 0..(1 << 23) {
                for j in 0..(1 << 3) {
                    a += binary_trace.a[j][i];
                }
            }
        })
    });

    println!("a: {}", a);
}

fn with_raw(c: &mut Criterion) {
    let mut binary_trace = vec![0; (1 << 23) * 32];
    let stride = 32;

    // Initialize the buffer
    for i in 0..(1 << 23) {
        for j in 0..(1 << 3) {
            let index = i * stride + j;
            binary_trace[index] = index as u64;
        }
    }

    let mut a = 0u64;
    c.bench_function("access_using_raw", |b| {
        b.iter(|| {
            for i in 0..(1 << 23) {
                for j in 0..(1 << 3) {
                    a += binary_trace[i * stride + j];
                }
            }
        })
    });

    println!("a: {}", a);
}

criterion_group!(benches, with_trace, with_raw);
criterion_main!(benches);

// The folllowing code is not used, is the original code from the benchmark
// The results are more accurate

// use proofman::trace::trace_seq::TraceSeq;
// use std::time::Instant;

// fn bench() {
//     let iter = 300;

//     println!("With trace");
//     with_trace2(iter);

//     println!("With raw");
//     with_raw2(iter);
// }

// fn with_trace2(iter: usize) {
//     const STRIDE: usize = 32;

//     trace!(BinaryTrace { a: [u64; STRIDE] });

//     let mut binary_trace = BinaryTrace::new(1 << 23);

//     for i in 0..(1 << 23) {
//         for j in 0..(1 << 3) {
//             binary_trace.a[j][i] = (i * STRIDE + j) as u64;
//         }
//     }

//     // Measure time using standard clock not with openmp
//     let start_time = Instant::now();

//     let mut a = 0u64;
//     for _k in 0..iter {
//         a = 0u64;
//         for i in 0..(1 << 23) {
//             for j in 0..(1 << 3) {
//                 a += binary_trace.a[j][i];
//             }
//         }
//     }

//     println!("a: {}", a);

//     // Stop measuring time
//     let end_time = Instant::now();

//     // Calculate the duration in microseconds
//     let duration = end_time.duration_since(start_time);

//     // Print the duration
//     println!(
//         "Execution time: {} microseconds",
//         duration.as_micros() / iter as u128
//     );
// }

// fn with_raw2(iter: usize) {
//     let mut binary_trace = vec![0; (1 << 23) * 32];
//     let stride = 32;

//     // Initialize the buffer
//     for i in 0..(1 << 23) {
//         for j in 0..(1 << 3) {
//             let index = i * stride + j;
//             binary_trace[index] = index as u64;
//         }
//     }

//     // Measure time using standard clock not with openmp
//     let start_time = Instant::now();

//     let mut a = 0u64;
//     for _k in 0..iter {
//         a = 0;
//         for i in 0..(1 << 23) {
//             for j in 0..(1 << 3) {
//                 a += binary_trace[i * stride + j];
//             }
//         }
//     }

//     println!("a: {}", a);

//     // Stop measuring time
//     let end_time = Instant::now();

//     // Calculate the duration in microseconds
//     let duration = end_time.duration_since(start_time);

//     // Print the duration
//     println!(
//         "Execution time: {} microseconds",
//         duration.as_micros() / iter as u128
//     );
// }
