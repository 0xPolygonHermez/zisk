use super::trace_column::TraceColumn;

// TRACE LAYOUT
// ================================================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct TraceLayout {
    trace_columns: Vec<TraceColumn>,
    num_rows: usize,
    row_bytes: u32,
}

#[allow(dead_code)]
impl TraceLayout {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------
    pub fn new(num_rows: usize) -> TraceLayout {
        let trace_columns = Vec::<TraceColumn>::new();
        let mut row_bytes = 0;
        for trace_column in trace_columns.iter() {
            row_bytes += trace_column.column_bytes();
        }

        TraceLayout {
            trace_columns,
            num_rows,
            row_bytes,
        }
    }

    pub fn add_column(&mut self, trace_column: TraceColumn) {
        self.row_bytes += trace_column.column_bytes();
        self.trace_columns.push(trace_column);
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------
    pub fn num_cols(&self) -> usize {
        self.trace_columns.len()
    }

    pub fn num_rows(&self) -> usize {
        self.num_rows
    }

    pub fn row_bytes(&self) -> u32 {
        self.row_bytes
    }

    pub fn trace_columns(&self) -> &Vec<TraceColumn> {
        &self.trace_columns
    }

    pub fn find_column_idx_by_name(&self, column_name: &str) -> Option<usize> {
        self.trace_columns.iter().position(|c| c.column_name() == column_name)
    }

    pub fn find_column_by_name(&self, column_name: &str) -> Option<&TraceColumn> {
        self.trace_columns.iter().find(|c| c.column_name() == column_name)
    }

    pub fn exists_column(&self, column_name: &str) -> bool {
        self.trace_columns.iter().any(|c| c.column_name() == column_name)
    }

}