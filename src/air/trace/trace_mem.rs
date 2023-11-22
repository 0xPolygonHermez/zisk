use super::trace_layout::TraceLayout;

// TRACE MEMORY PTR
// ================================================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct TraceMemPtr {
    pub column_name: String,
    pub ptr: *const u8,
    pub offset: u32,
}

#[allow(dead_code)]
impl TraceMemPtr {
    pub fn new(column_name: String, ptr: *const u8, offset: u32) -> TraceMemPtr {
        TraceMemPtr {
            column_name,
            ptr,
            offset,
        }
    }
}

// TRACE MEMORY BUFFER
// ================================================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct TraceMem {
    pub row_bytes: u32,
    pub num_rows: usize,
    pub buffer: Vec::<u8>,
    pub trace_table: Vec::<TraceMemPtr>,
}

#[allow(dead_code)]
impl TraceMem {
    pub fn new(trace_layout: &TraceLayout, trace_store_type: &String) -> TraceMem {
        let buffer = vec![0_u8; trace_layout.num_rows() * trace_layout.row_bytes() as usize];

        let mut trace_table = Vec::<TraceMemPtr>::new();
        
        unsafe {
            let mut col_offset = 0;
            for trace_column in trace_layout.trace_columns().iter() {
                let ptr: *const u8 = buffer.as_ptr().offset(col_offset) as *const u8;
                col_offset += trace_column.column_bytes() as isize;

                let offset = if trace_store_type == "row_major" {
                    trace_layout.row_bytes()
                } else { // TODO control if there is a column with unknown type
                    trace_column.column_bytes()
                };

                let trace_table_item = TraceMemPtr::new(
                    trace_column.column_name().to_string(),
                    ptr,
                    offset
                );

                trace_table.push(trace_table_item);
            }
        }

        TraceMem {
            row_bytes: trace_layout.row_bytes(),
            num_rows: trace_layout.num_rows(),
            buffer,
            trace_table
        }
    }
}