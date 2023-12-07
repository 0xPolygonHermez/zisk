// use super::trace_column::TraceColumnLayout;
// use super::trace_layout::TraceLayout;
// use super::trace::{Trace, StoreType};
// use crate::air::mock_base_field::mock_base_field::{MockBaseField, BaseFieldType};

// #[test]
// fn new_trace_table() {
//     let trace_column = TraceColumnLayout::new("colA", 8);

//     assert_eq!(trace_column.column_name(), "colA");
//     assert_eq!(trace_column.column_bytes(), 8);
    
//     assert_eq!(1, 1);
// }

// fn create_mock_vector_a() -> Vec<MockBaseField> {
//     vec![
//         MockBaseField::new(BaseFieldType::NoExtended, &[1]),
//         MockBaseField::new(BaseFieldType::NoExtended, &[2]),
//         MockBaseField::new(BaseFieldType::NoExtended, &[3]),
//         MockBaseField::new(BaseFieldType::NoExtended, &[4]),
//     ]
// }

// fn create_mock_vector_b() -> Vec<MockBaseField> {
//     vec![
//         MockBaseField::new(BaseFieldType::Extended, &[10, 11, 12]),
//         MockBaseField::new(BaseFieldType::Extended, &[13, 14, 15]),
//         MockBaseField::new(BaseFieldType::Extended, &[16, 17, 18]),
//         MockBaseField::new(BaseFieldType::Extended, &[19, 20, 21]),
//     ]
// }

// fn create_mock_vector_c() -> Vec<MockBaseField>{
//     vec![
//         MockBaseField::new(BaseFieldType::NoExtended, &[5]),
//         MockBaseField::new(BaseFieldType::NoExtended, &[6]),
//         MockBaseField::new(BaseFieldType::NoExtended, &[7]),
//         MockBaseField::new(BaseFieldType::NoExtended, &[8]),
//     ]
// }

// //TODO reorganize tests, change names, ...
// #[test]
// fn test_trace() {
//     let trace_col_a =  TraceColumnLayout::new("colA", MockBaseField::SIZE);
//     let trace_col_b =  TraceColumnLayout::new("colB", MockBaseField::EXTENDED_SIZE);
//     let trace_col_c =  TraceColumnLayout::new("colC", MockBaseField::SIZE);

//     let num_rows = 2usize.pow(2);
//     let mut trace_layout = TraceLayout::new(num_rows);
//     trace_layout.add_column(trace_col_a);
//     trace_layout.add_column(trace_col_b);
//     trace_layout.add_column(trace_col_c);

//     let mut trace: Trace = Trace::new(trace_layout, StoreType::RowMajor);

//     let values_col_a = create_mock_vector_a();
//     let values_col_b = create_mock_vector_b();
//     let values_col_c = create_mock_vector_c();

//     trace.set_column("colA", &values_col_a);
//     trace.set_column("colB", &values_col_b);
//     trace.set_column("colC", &values_col_c);

//     println!("trace: {:?}", trace);
//     trace.set("colB", 1, &MockBaseField::new(BaseFieldType::Extended, &[113, 114, 115]));
    
//     let temp = trace.get("colB", 1);
//     println!("temp: {:?}", temp);

// }

// #[test]
// fn test_trace_fibonacci() {
//     let num_rows = 2usize.pow(3);

//     // Create Trace Layout
//     let mut trace_layout = TraceLayout::new(num_rows);

//     trace_layout.add_column(TraceColumnLayout::new("witness.a", MockBaseField::SIZE));
//     trace_layout.add_column(TraceColumnLayout::new("witness.b", MockBaseField::SIZE));
//     trace_layout.add_column(TraceColumnLayout::new("fixed.L1", MockBaseField::SIZE));
//     trace_layout.add_column(TraceColumnLayout::new("fixed.LLAST", MockBaseField::SIZE));

//     // Create Mock Data values for witness and fixed columns
//     let mut witness_a = Vec::<MockBaseField>::new();
//     let mut witness_b = Vec::<MockBaseField>::new();
//     let mut fixed_l1 = Vec::<MockBaseField>::new();
//     let mut fixed_llast = Vec::<MockBaseField>::new();

//     let mut a = 1;
//     let mut b = 1;

//     for i in 0..num_rows {
//         witness_a.push(MockBaseField::new(BaseFieldType::NoExtended, &[a]));
//         witness_b.push(MockBaseField::new(BaseFieldType::NoExtended, &[b]));
//         fixed_l1.push(MockBaseField::new(BaseFieldType::NoExtended, &[if i == 0 { 1 } else { 0 }]));
//         fixed_llast.push(MockBaseField::new(BaseFieldType::NoExtended, &[if i == num_rows - 1 { 1 } else { 0 }]));

//         let temp = a;
//         a = b;
//         b = temp + b;
//     }

//     // Create Trace
//     let mut trace: Trace = Trace::new(trace_layout, StoreType::RowMajor);

//     trace.set_column("witness.a", &witness_a);
//     trace.set_column("witness.b", &witness_b);
//     trace.set_column("fixed.L1", &fixed_l1);
//     trace.set_column("fixed.LLAST", &fixed_llast);

//     println!("trace: {:?}", trace);

//     println!("col_a: {:?}", trace.get_column("witness.a"));
//     println!("col_b: {:?}", trace.get_column("witness.b"));
//     println!("col_l1: {:?}", trace.get_column("fixed.L1"));
//     println!("col_llast: {:?}", trace.get_column("fixed.LLAST"));
// }