// use super::trace_layout::TraceLayout;
// use super::trace_mem::TraceMem;

// use crate::air::mock_base_field::mock_base_field::MockBaseField;

// // TRACE
// // ================================================================================================
// #[derive(Debug, Clone, PartialEq)]
// #[allow(dead_code)]
// pub enum StoreType {
//     RowMajor,
//     ColMajor,
// }

// // TODO: declare Trace as Trace<T> where T: BaseField
// #[derive(Debug, Clone, PartialEq)]
// pub struct Trace {
//     trace_layout: TraceLayout,
//     trace_store_type: StoreType,
//     trace_mem: TraceMem
// }

// #[allow(dead_code)]
// impl Trace {
//     // CONSTRUCTORS
//     // --------------------------------------------------------------------------------------------
//     pub fn new(trace_layout: TraceLayout, trace_store_type: StoreType) -> Trace {
//         let trace_mem = TraceMem::new(&trace_layout, &trace_store_type);

//         Trace {
//             trace_layout,
//             trace_store_type,
//             trace_mem,
//         }
//     }

//     // PUBLIC ACCESSORS
//     // --------------------------------------------------------------------------------------------
//     pub fn trace_layout(&self) -> &TraceLayout {
//         &self.trace_layout
//     }

//     pub fn trace_store_type(&self) -> &StoreType {
//         &self.trace_store_type
//     }

//     pub fn get(&self, col_name: &str, row_idx: usize) -> MockBaseField {
//         self.trace_mem.get(col_name, row_idx)
//     }

//     pub fn get_column(&self, col_name: &str) -> Vec<MockBaseField> {
//         self.trace_mem.get_column(col_name)
//     }

//     pub fn set(&mut self, col_name: &str, row_idx: usize, value: &MockBaseField) {
//         self.trace_mem.set(col_name, row_idx, value);
//     }

//     pub fn set_column(&mut self, col_name: &str, values: &[MockBaseField]) {
//         self.trace_mem.set_column(col_name, values);
//     }

//     pub fn print(&self) {
//         println!("Trace Layout Info");
//         println!("    Columns: {}", self.trace_layout.num_cols());
//         println!("    Rows: {}", self.trace_layout.num_rows());
//         println!("    Bytes per Row: {}", self.trace_layout.row_bytes());
//         println!("    Mem: {:?}", self.trace_mem);
//     }
// }