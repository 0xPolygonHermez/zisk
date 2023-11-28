// use crate::air::mock_base_field::mock_base_field::MockBaseField;

// use super::trace_layout::TraceLayout;
// use core::slice;

// // TRACE ROW MAJOR BUFFER
// // ================================================================================================
// #[derive(Debug, Clone, PartialEq)]
// pub struct TraceRowMajorBuffer {
//     buffer: Vec<u8>,
//     num_rows: usize,
//     row_bytes: usize,
//     total_bytes: usize,
// }

// #[allow(dead_code)]
// impl TraceRowMajorBuffer {
//     // CONSTRUCTORS
//     // --------------------------------------------------------------------------------------------
//     pub fn new(trace_layout: &TraceLayout) -> TraceRowMajorBuffer {
//         let buffer = vec![u8::default(); trace_layout.num_rows() * trace_layout.row_bytes() as usize];
//         let num_rows = trace_layout.num_rows();
//         let row_bytes = trace_layout.row_bytes();

//         let total_bytes = row_bytes as usize * num_rows;

//         TraceRowMajorBuffer {
//             buffer,
//             num_rows,
//             row_bytes,
//             total_bytes,
//         }
//     }

//     // PUBLIC ACCESSORS
//     // --------------------------------------------------------------------------------------------
//     pub fn num_rows(&self) -> usize { self.num_rows }

//     pub fn row_bytes(&self) -> usize { self.row_bytes }

//     pub fn total_bytes(&self) -> usize { self.total_bytes }

//     pub fn row(&self, row_idx: usize) -> &[u8] {
//         assert!(row_idx < self.num_rows, "Row index out of bounds");
//         let start = row_idx * self.row_bytes as usize;

//         let elements = &self.buffer[start..start + start + self.row_bytes as usize];

//         let ptr = elements.as_ptr();
//         let len = elements.len();
//         unsafe { slice::from_raw_parts(ptr as *const u8, len) }
//     }

//     pub fn get(&self, col_idx: usize, row_idx: usize, num_bytes: usize) -> &[u8] {
//         assert!(row_idx < self.num_rows, "Row index out of bounds");
//         assert!(col_idx < self.row_bytes as usize, "Column index out of bounds");

//         let position = row_idx * self.row_bytes as usize + col_idx;
//         &self.buffer[position..(position + num_bytes as usize)]
//     }

//     pub fn set(&mut self, col_offset: usize, row_idx: usize, value: &MockBaseField) {
//         assert!(row_idx < self.num_rows, "Row index out of bounds");
//         assert!(col_offset < self.row_bytes as usize, "Column offset out of bounds");

//         let value = value.value();
//         let position = row_idx * self.row_bytes as usize + col_offset;
//         self.buffer.splice(position..(position + value.len()), value.into_iter().cloned());
//     }

//     pub fn set_column(&mut self, col_offset: usize, values: &[MockBaseField]) {
//         assert!(values.len() == self.num_rows, "Number of values does not match number of rows in the trace");

//         for row_idx in 0..self.num_rows {
//             self.set(col_offset, row_idx, &values[row_idx]);
//         }
//     }
// }