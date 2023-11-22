use super::trace_layout::TraceLayout;
use super::trace_mem::TraceMem;

// TRACE
// ================================================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct Trace {
    trace_layout: TraceLayout,
    trace_store_type: String,
    trace_mem_type: String,
    trace_mem: TraceMem
}

#[allow(dead_code)]
impl Trace {
    pub fn new(trace_layout: TraceLayout, trace_store_type: String, trace_mem_type: String) -> Trace {
        let trace_mem = TraceMem::new(&trace_layout, &trace_store_type);

        Trace {
            trace_layout,
            trace_store_type,
            trace_mem_type,
            trace_mem,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------
    pub fn trace_layout(&self) -> &TraceLayout {
        &self.trace_layout
    }

    pub fn trace_store_type(&self) -> &String {
        &self.trace_store_type
    }

    pub fn trace_mem_type(&self) -> &String {
        &self.trace_mem_type
    }

    // pub fn set(&mut self, column: usize, step: usize, value: ) {
    //     self.trace.set(column, step, value)
    // }

    pub fn print(&self) {
        println!("Trace Layout Info");
        println!("    Columns: {}", self.trace_layout.num_cols());
        println!("    Rows: {}", self.trace_layout.num_rows());
        println!("    Trace Store Type: {}", self.trace_store_type);
        println!("    Trace Mem   Type: {}", self.trace_mem_type);
        println!("    Bytes per Row: {}", self.trace_layout.row_bytes());
        println!("    Mem: {:?}", self.trace_mem);
    }
}