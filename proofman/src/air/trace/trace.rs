use crate::air::trace::trace_layout::TraceLayout;

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum StoreType {
    RowMajor,
    ColMajor,
}

// TRACE COLUMN SEGMENT
// ================================================================================================
/// A segment of a trace column. A trace column can be split into multiple segments if it is
/// stored in on or multiple buffers.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceColSegment {
    /// Name of the trace column.
    column: String,
    /// Size in bytes of each element
    column_bytes: usize,
    /// Row index of the first row in the segment.
    row_from: usize,
    /// Row index of the last row in the segment.
    row_to: usize,
    /// Index of the buffer in the TraceBuffersTable where the segment is stored.
    buffer_idx: usize,
    /// Offset in bytes of the first element within the buffer.
    offset: usize,
    /// Offset in bytes of the next row element within the buffer.
    next: usize,
    /// Flag indicating whether this is the last segment of the column.
    last: bool
}

/// Trace buffer to store trace column segments.
#[derive(Debug, Clone, PartialEq)]
pub struct TraceBuffer {
    /// Buffer data.
    buffer: Vec<u8>,
    row_bytes: usize,
    num_rows: usize,
}

#[allow(dead_code)]
impl TraceBuffer {
    /// Creates a new trace buffer of the specified size.
    pub fn new(row_bytes: usize, num_rows: usize) -> TraceBuffer {
        let buffer = vec![u8::default(); row_bytes * num_rows];
        TraceBuffer { buffer, row_bytes, num_rows }
    }

    /// Returns a reference to the buffer data.
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Returns a mutable reference to the buffer data.
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    /// Returns the size of the buffer.
    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    pub fn set_element(&mut self, offset: usize, value: &[u8]) {
        assert!(offset + value.len() <= self.buffer.len());

        self.buffer[offset..offset + value.len()].copy_from_slice(value);
    }

}

/// Trace air context is a container for trace column segments and trace buffers. Each air instance has a single trace air context.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct Trace {
    /// Trace layout. TODO, this should be a reference to the layout in the air. ADD metadata as filled?
    layout: TraceLayout,
    /// Trace store type.
    store_type: StoreType,
    /// Trace column segments.
    segments: Vec<TraceColSegment>,
    /// Trace buffers.
    buffers: Vec<TraceBuffer>,
}

#[allow(dead_code)]
impl Trace {
    pub fn new(layout: &TraceLayout, store_type: StoreType) -> Trace {
        // TODO! Check if layout.clone() is a good idea, better to pass a reference and a lifetime?
        Trace { layout: layout.clone(), store_type, segments: Vec::<TraceColSegment>::new(), buffers: Vec::<TraceBuffer>::new() }
    }

    pub fn new_trace(&mut self, trace_rows: usize) {
        // Check trace_rows is less than or equal to self num_rows
        assert!(trace_rows <= self.layout.num_rows());

        // Create a new buffer
        let trace_buffer = TraceBuffer::new(self.layout.row_bytes(), self.layout.num_rows());
        self.buffers.push(trace_buffer);

        // Add all trace col segements to the context
        let mut offset = 0;
        for trace_column in self.layout.trace_columns() {
            let segment = TraceColSegment {
                column: trace_column.column_name().to_string(),
                column_bytes: trace_column.column_bytes(),
                row_from: 0,
                row_to: trace_rows - 1,
                buffer_idx: self.buffers.len() - 1,
                offset,
                // At the moment we only support row major layout so next is equal to the row_bytes
                next: self.layout.row_bytes() as usize,
                last: true
            };
            self.segments.push(segment);

            offset += trace_column.column_bytes() as usize;
        }
    }

    pub fn set_buffer_u8(&mut self, trace_rows: usize, values: &[u8]) {
        // Check that the row_bytes * num_rows is equal to the size of the values buffer and that
        // the row_bytes is equal to the size of the current context layout
        assert_eq!(self.layout.row_bytes() as usize * trace_rows, values.len());

        // NOTE: At the moment we only support row major layout
        // TODO! Add support for column major layout

        // Check trace_rows is less than or equal to self num_rows
        assert!(trace_rows <= self.layout.num_rows());

        // Create a new buffer
        let buffer = self.buffers[0].buffer_mut();
        buffer[0..values.len()].copy_from_slice(values);
    }

    pub fn set_column_u8(&mut self, column_name: &str, num_rows: usize, values: &[u8]) {
        // Check that the values buffer size is multiple of len
        assert!(values.len() % num_rows == 0);
        // Check that the values fits in the trace buffer
        assert!(num_rows <= self.layout.num_rows());

        // NOTE: At the moment we only support row major layout
        // TODO! Add support for column major layout

        let trace_column = self.segments.iter().find(|c| c.column == column_name).unwrap();

        assert_eq!(trace_column.column_bytes, values.len() / num_rows);
        assert!(trace_column.row_to - trace_column.row_from + 1 == num_rows);

        let element_bytes = trace_column.column_bytes;
        let mut offset = trace_column.offset;

        for i in 0..num_rows {
            self.buffers[0].set_element(offset, &values[i * element_bytes..(i + 1) * element_bytes]);
            offset += trace_column.next;
        }
    }
}


#[cfg(test)]
mod tests {
    // Import necessary items
    use super::*;

    // TODO! Add tests
    #[test]
    fn test_addition() {
        //let trace_air_context = TraceAirContext::new(TraceLayout::new(8), StoreType::RowMajor);
    }

    // More test functions can be added here
}