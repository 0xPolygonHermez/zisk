use super::trace_column::TraceColumn;
use super::trace_layout::TraceLayout;
use super::trace::{Trace, StoreType};
use crate::air::mock_base_field::mock_base_field::{MockBaseField, BaseFieldType};

#[test]
fn new_trace_table() {
    let trace_column = TraceColumn::new("colA", 8);

    assert_eq!(trace_column.column_name(), "colA");
    assert_eq!(trace_column.column_bytes(), 8);
    
    assert_eq!(1, 1);
}

fn create_mock_vector_a() -> Vec<MockBaseField> {
    vec![
        MockBaseField::new(BaseFieldType::NoExtended, &[1]),
        MockBaseField::new(BaseFieldType::NoExtended, &[2]),
        MockBaseField::new(BaseFieldType::NoExtended, &[3]),
        MockBaseField::new(BaseFieldType::NoExtended, &[4]),
    ]
}

fn create_mock_vector_b() -> Vec<MockBaseField> {
    vec![
        MockBaseField::new(BaseFieldType::Extended, &[10, 11, 12]),
        MockBaseField::new(BaseFieldType::Extended, &[13, 14, 15]),
        MockBaseField::new(BaseFieldType::Extended, &[16, 17, 18]),
        MockBaseField::new(BaseFieldType::Extended, &[19, 20, 21]),
    ]
}

fn create_mock_vector_c() -> Vec<MockBaseField>{
    vec![
        MockBaseField::new(BaseFieldType::NoExtended, &[5]),
        MockBaseField::new(BaseFieldType::NoExtended, &[6]),
        MockBaseField::new(BaseFieldType::NoExtended, &[7]),
        MockBaseField::new(BaseFieldType::NoExtended, &[8]),
    ]
}

//TODO reorganize tests, change names, ...
#[test]
fn test_trace() {
    let trace_col_a =  TraceColumn::new("colA", 1);
    let trace_col_b =  TraceColumn::new("colB", 3);
    let trace_col_c =  TraceColumn::new("colC", 1);

    let num_rows = 2usize.pow(2);
    let mut trace_layout = TraceLayout::new(num_rows);
    trace_layout.add_column(trace_col_a);
    trace_layout.add_column(trace_col_b);
    trace_layout.add_column(trace_col_c);

    let mut trace: Trace = Trace::new(trace_layout, StoreType::RowMajor);

    let values_col_a = create_mock_vector_a();
    let values_col_b = create_mock_vector_b();
    let values_col_c = create_mock_vector_c();

    trace.set_column("colA", &values_col_a);
    trace.set_column("colB", &values_col_b);
    trace.set_column("colC", &values_col_c);

    println!("trace: {:?}", trace);
    trace.set("colB", 1, &MockBaseField::new(BaseFieldType::Extended, &[113, 114, 115]));
    
    let temp = trace.get("colB", 1);
    println!("temp: {:?}", temp);

}