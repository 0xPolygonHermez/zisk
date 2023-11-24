use crate::air::mock_base_field::mock_base_field::MockBaseField;

use super::trace_layout::TraceLayout;
use super::trace_row_major_buffer::TraceRowMajorBuffer;
use super::trace::StoreType;

// TRACE MEMORY PTR
// ================================================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct TraceTablePtr {
    pub column_name: String,
    buffer_idx: u32,
    num_bytes: u32,
    offset: usize,
    next: usize,
}

#[allow(dead_code)]
impl TraceTablePtr {
    pub fn new(column_name: String, buffer_idx: u32, num_bytes: u32, offset: usize, next: usize) -> TraceTablePtr {
        TraceTablePtr {
            column_name,
            buffer_idx,
            num_bytes,
            offset,
            next,
        }
    }
}

// TRACE MEMORY BUFFER
// ================================================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct TraceMem {
    pub row_bytes: u32,
    pub num_rows: usize,
    traces_table: Vec::<TraceTablePtr>,
    buffers_table: Vec::<TraceRowMajorBuffer>,
}

#[allow(dead_code)]
impl TraceMem {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------
    pub fn new(trace_layout: &TraceLayout, trace_store_type: &StoreType) -> TraceMem {
        let mut traces_table = Vec::<TraceTablePtr>::new();

        let mut buffers_table = Vec::<TraceRowMajorBuffer>::new();
        buffers_table.push(TraceRowMajorBuffer::new(trace_layout));

        let mut offset = 0;

        for trace_column in trace_layout.trace_columns().iter() {
            let next = if trace_store_type == &StoreType::RowMajor { trace_layout.row_bytes() } else { trace_column.column_bytes() };
            let trace_table_ptr = TraceTablePtr::new(
                trace_column.column_name().to_string(),
                0,
                trace_column.column_bytes(),
                offset,
                next as usize,
            );
            traces_table.push(trace_table_ptr);
            offset += trace_column.column_bytes() as usize;
        }        

        TraceMem {
            row_bytes: trace_layout.row_bytes(),
            num_rows: trace_layout.num_rows(),
            traces_table,
            buffers_table,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------
    pub fn find_column_idx_by_name(&self, column_name: &str) -> Option<usize> {
        self.traces_table.iter().position(|c| c.column_name == column_name)
    }

    pub fn get(&self, column_name: &str, row_idx: usize) -> MockBaseField {
        let column_idx = self.find_column_idx_by_name(column_name).unwrap();

        let col_offset = self.traces_table[column_idx].offset;
        let buffer = self.buffers_table.get(0).unwrap();

        let value_u8 = buffer.get(col_offset, row_idx, self.traces_table[column_idx].num_bytes);

        MockBaseField::from_raw_parts(value_u8)
    }

    pub fn set(&mut self, column_name: &str, row_idx: usize, value: &MockBaseField) {
        let column_idx = self.find_column_idx_by_name(column_name).unwrap();

        let col_offset = self.traces_table[column_idx].offset;
        let buffer = self.buffers_table.get_mut(0).unwrap();
        buffer.set(col_offset, row_idx, value);
    }

    pub fn set_column(&mut self, column_name: &str, values: &[MockBaseField]) {
        assert!(values.len() == self.num_rows, "Number of values does not match number of rows in the trace");

        let column_idx = self.find_column_idx_by_name(column_name).unwrap();

        let col_offset = self.traces_table[column_idx].offset;
        let buffer = self.buffers_table.get_mut(0).unwrap();
        buffer.set_column(col_offset, values);
    }    
}