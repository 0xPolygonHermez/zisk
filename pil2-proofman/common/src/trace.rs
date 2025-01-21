pub trait Trace<F>: Send {
    fn num_rows(&self) -> usize;
    fn n_cols(&self) -> usize;
    fn airgroup_id(&self) -> usize;
    fn air_id(&self) -> usize;
    fn commit_id(&self) -> Option<usize>;
    fn get_buffer(&mut self) -> Vec<F>;
}

pub trait Values<F>: Send {
    fn get_buffer(&mut self) -> Vec<F>;
}

pub use proofman_macros::trace;

pub use proofman_macros::values;

#[cfg(test)]
use crate as common;

#[test]
fn check() {
    trace!(TraceRow, MyTrace<F> { a: F, b:F}, 0, 0, 8, 0);

    assert_eq!(TraceRow::<usize>::ROW_SIZE, 2);

    let mut trace = MyTrace::new();
    let num_rows = trace.num_rows();

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
    trace!(TraceRow, MyTrace<F> { a: F, b: [F; 3], c: F }, 0, 0, 8, 0);

    assert_eq!(TraceRow::<usize>::ROW_SIZE, 5);
    let mut trace = MyTrace::new();
    let num_rows = trace.num_rows();

    // Set values
    for i in 0..num_rows {
        trace[i].a = i;
        trace[i].b[0] = i * 10;
        trace[i].b[1] = i * 20;
        trace[i].b[2] = i * 30;
        trace[i].c = i * 40;
    }

    let buffer = trace.get_buffer();

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
    trace!(TraceRow, MyTrace<F> { a: [[F;3]; 2], b: F }, 0, 0, 8, 0);

    assert_eq!(TraceRow::<usize>::ROW_SIZE, 7);

    let mut trace = MyTrace::new();
    let num_rows = trace.num_rows();

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

    let buffer = trace.get_buffer();

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
    trace!(TraceRow, MyTrace<F> { a: [[F;3]; 2], b: F, c: [F; 2] }, 0, 0, 8, 0);

    assert_eq!(TraceRow::<usize>::ROW_SIZE, 9);

    let mut trace = MyTrace::new();
    let num_rows = trace.num_rows();

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

    let buffer = trace.get_buffer();

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
