use super::trace_column::TraceColumn;

#[test]
fn new_trace_table() {
    let trace_column = TraceColumn::new("colA", 8);

    assert_eq!(trace_column.column_name(), "colA");
    assert_eq!(trace_column.column_bytes(), 8);
    
    assert_eq!(1, 1);
}