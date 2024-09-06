pub trait Trace: Send {
    fn num_rows(&self) -> usize;
    fn get_buffer_ptr(&mut self) -> *mut u8;
}

pub use proofman_macros::trace;

#[cfg(test)]
use crate as common;

#[test]
fn check() {
    const OFFSET: usize = 1;
    let num_rows = 8;

    trace!(TraceRow, MyTrace<F> { a: F, b: F });

    let mut buffer = vec![0usize; num_rows * TraceRow::<usize>::ROW_SIZE + OFFSET];
    let trace = MyTrace::map_buffer(&mut buffer, num_rows, OFFSET);
    let mut trace = trace.unwrap();

    // Set values
    for i in 0..num_rows {
        trace[i].a = i;
        trace[i].b = i * 10;
    }

    // Check values
    for i in 0..num_rows {
        assert_eq!(trace[i].a, i);
        assert_eq!(trace[i].b, i * 10);
    }
}

#[test]
#[should_panic]
fn test_errors_are_launched_when_num_rows_is_invalid_1() {
    let mut buffer = vec![0u8; 3];
    trace!(SimpleRow, Simple<F> { a: F });
    let _ = Simple::map_buffer(&mut buffer, 1, 0);
}

#[test]
#[should_panic]
fn test_errors_are_launched_when_num_rows_is_invalid_2() {
    let mut buffer = vec![0u8; 3];
    trace!(SimpleRow, Simple<F> { a: F });
    let _ = Simple::map_buffer(&mut buffer, 3, 0);
}

#[test]
#[should_panic]
fn test_errors_are_launched_when_num_rows_is_invalid_3() {
    trace!(SimpleRow, Simple<F> { a: F });
    let _ = Simple::<u8>::new(1);
}

#[test]
#[should_panic]
fn test_errors_are_launched_when_num_rows_is_invalid_4() {
    trace!(SimpleRow, Simple<F> { a: F });
    let _ = Simple::<u8>::new(3);
}
