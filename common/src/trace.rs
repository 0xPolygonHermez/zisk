pub trait Trace: Send {
    fn num_rows(&self) -> usize;
    fn get_buffer_ptr(&mut self) -> *mut u8;
}

pub use proofman_macros::trace;

#[cfg(test)]
use crate as common;

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

#[test]
fn check() {
    const OFFSET: usize = 1;
    let num_rows = 8;

    trace!(TraceRow, MyTrace<F> { a: F, b:F});

    assert_eq!(TraceRow::<usize>::ROW_SIZE, 2);

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
fn check_array() {
    let num_rows = 8;

    trace!(TraceRow, MyTrace<F> { a: F, b: [F; 3], c: F });

    assert_eq!(TraceRow::<usize>::ROW_SIZE, 5);
    let mut buffer = vec![0usize; num_rows * TraceRow::<usize>::ROW_SIZE];
    let trace = MyTrace::map_buffer(&mut buffer, num_rows, 0);
    let mut trace = trace.unwrap();

    // Set values
    for i in 0..num_rows {
        trace[i].a = i;
        trace[i].b[0] = i * 10;
        trace[i].b[1] = i * 20;
        trace[i].b[2] = i * 30;
        trace[i].c = i * 40;
    }

    // Check values
    for i in 0..num_rows {
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE], i);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 1], i * 10);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 2], i * 20);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 3], i * 30);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 4], i * 40);
    }
}

#[test]
fn check_multi_array() {
    let num_rows = 8;

    trace!(TraceRow, MyTrace<F> { a: [[F;3]; 2], b: F });

    assert_eq!(TraceRow::<usize>::ROW_SIZE, 7);
    let mut buffer = vec![0usize; num_rows * TraceRow::<usize>::ROW_SIZE];
    let trace = MyTrace::map_buffer(&mut buffer, num_rows, 0);
    let mut trace = trace.unwrap();

    // Set values
    for i in 0..num_rows {
        trace[i].a[0][0] = i;
        trace[i].a[0][1] = i * 10;
        trace[i].a[0][2] = i * 20;
        trace[i].a[1][0] = i * 30;
        trace[i].a[1][1] = i * 40;
        trace[i].a[1][2] = i * 50;
        trace[i].b = i + 3;
    }

    // Check values
    for i in 0..num_rows {
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE], i);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 1], i * 10);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 2], i * 20);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 3], i * 30);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 4], i * 40);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 5], i * 50);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 6], i + 3);
    }
}

#[test]
fn check_multi_array_2() {
    let num_rows = 8;

    trace!(TraceRow, MyTrace<F> { a: [[F;3]; 2], b: F, c: [F; 2] });

    assert_eq!(TraceRow::<usize>::ROW_SIZE, 9);
    let mut buffer = vec![0usize; num_rows * TraceRow::<usize>::ROW_SIZE];
    let trace = MyTrace::map_buffer(&mut buffer, num_rows, 0);
    let mut trace = trace.unwrap();

    // Set values
    for i in 0..num_rows {
        trace[i].a[0][0] = i;
        trace[i].a[0][1] = i * 10;
        trace[i].a[0][2] = i * 20;
        trace[i].a[1][0] = i * 30;
        trace[i].a[1][1] = i * 40;
        trace[i].a[1][2] = i * 50;
        trace[i].b = i + 3;
        trace[i].c[0] = i + 9;
        trace[i].c[1] = i + 2;
    }

    // Check values
    for i in 0..num_rows {
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE], i);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 1], i * 10);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 2], i * 20);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 3], i * 30);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 4], i * 40);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 5], i * 50);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 6], i + 3);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 7], i + 9);
        assert_eq!(buffer[i * TraceRow::<usize>::ROW_SIZE + 8], i + 2);
    }
}
