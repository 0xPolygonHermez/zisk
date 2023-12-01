// TRACE COLUMN LAYOUT
// ================================================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct TraceColumnLayout {
    column_name: String,
    column_bytes: usize,
}

#[allow(dead_code)]
impl TraceColumnLayout {
    pub fn new(column_name: &str, column_bytes: usize) -> TraceColumnLayout {
        TraceColumnLayout {
            column_name: String::from(column_name),
            column_bytes,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------
    pub fn column_name(&self) -> &str {
        &self.column_name
    }

    pub fn column_bytes(&self) -> usize {
        self.column_bytes
    }
}

// TRACE LAYOUT
// ================================================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct TraceLayout {
    trace_columns: Vec<TraceColumnLayout>,
    num_rows: usize,
    row_bytes: usize,
}

#[allow(dead_code)]
impl TraceLayout {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------
    pub fn new(num_rows: usize) -> TraceLayout {
        assert!(num_rows.is_power_of_two());

        let trace_columns = Vec::<TraceColumnLayout>::new();
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

    pub fn add_column(&mut self, column_name: String, column_bytes: usize) {
        let trace_column = TraceColumnLayout::new(&column_name, column_bytes);
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

    pub fn row_bytes(&self) -> usize {
        self.row_bytes
    }

    pub fn trace_columns(&self) -> &Vec<TraceColumnLayout> {
        &self.trace_columns
    }

    pub fn find_column_idx_by_name(&self, column_name: &str) -> Option<usize> {
        self.trace_columns.iter().position(|c| c.column_name() == column_name)
    }

    pub fn find_column_by_name(&self, column_name: &str) -> Option<&TraceColumnLayout> {
        self.trace_columns.iter().find(|c| c.column_name() == column_name)
    }

    pub fn exists_column(&self, column_name: &str) -> bool {
        self.trace_columns.iter().any(|c| c.column_name() == column_name)
    }

}