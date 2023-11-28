use crate::air::trace::trace_layout::TraceLayout;

use math::fields::f64::BaseElement;
use math::fields::CubeExtension;
use math::FieldElement;

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
}

#[allow(dead_code)]
impl TraceBuffer {
    /// Creates a new trace buffer of the specified size.
    pub fn new(size_bytes: usize) -> TraceBuffer {
        let buffer = vec![u8::default(); size_bytes];
        TraceBuffer { buffer }
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
}

/// Trace air context is a container for trace column segments and trace buffers. Each air instance has a single trace air context.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct TraceAirContext {
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
impl TraceAirContext {
    pub fn new(layout: &TraceLayout, store_type: StoreType) -> TraceAirContext {
        // TODO! Check if layout.clone() is a good idea, better to pass a reference and a lifetime?
        TraceAirContext { layout: layout.clone(), store_type, segments: Vec::<TraceColSegment>::new(), buffers: Vec::<TraceBuffer>::new() }
    }

    pub fn new_trace(&mut self, trace_rows: usize) {
        // Check trace_rows is less than or equal to self num_rows
        assert!(trace_rows <= self.layout.num_rows());

        // Create a new buffer
        let trace_buffer = TraceBuffer::new(self.layout.row_bytes() as usize * self.layout.num_rows());
        self.buffers.push(trace_buffer);

        // Add all trace col segements to the context
        let mut offset = 0;
        for trace_column in self.layout.trace_columns() {
            let segment = TraceColSegment {
                column: trace_column.column_name().to_string(),
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

    pub fn set_column(&mut self, column_name: &str, elements: &Vec<BaseElement>) {
        assert!(elements.len() <= self.layout.num_rows());

        let trace_column = self.segments.iter().find(|c| c.column == column_name).unwrap();
        let layout_column = self.layout.trace_columns().iter().find(|c| c.column_name() == column_name).unwrap();

        assert!(trace_column.row_to - trace_column.row_from + 1 == elements.len());

        let buffer = self.buffers[0].buffer_mut();

        let elements_u8 = FieldElement::elements_as_bytes(&elements);
        
        let element_bytes = layout_column.column_bytes();
        let mut offset = trace_column.offset;
        for i in 0..elements.len() {
            buffer[offset..offset + element_bytes].copy_from_slice(&elements_u8[i * element_bytes..(i + 1) * element_bytes]);
            offset += trace_column.next;
        }
    }

    pub fn set_ext_column(&mut self, column_name: &str, elements: &Vec<CubeExtension<BaseElement>>) {
        assert!(elements.len() <= self.layout.num_rows());

        let trace_column = self.segments.iter().find(|c| c.column == column_name).unwrap();
        let layout_column = self.layout.trace_columns().iter().find(|c| c.column_name() == column_name).unwrap();

        assert!(trace_column.row_to - trace_column.row_from + 1 == elements.len());

        let buffer = self.buffers[0].buffer_mut();

        let elements_u8 = FieldElement::elements_as_bytes(&elements);
        
        let element_bytes = layout_column.column_bytes();
        let mut offset = trace_column.offset;
        for i in 0..elements.len() {
            buffer[offset..offset + element_bytes].copy_from_slice(&elements_u8[i * element_bytes..(i + 1) * element_bytes]);
            offset += trace_column.next;
        }
    }

    pub fn add_trace_u8(&mut self, trace_rows: usize, values: &[u8]) {
        // Check that the row_bytes * num_rows is equal to the size of the values buffer and that
        // the row_bytes is equal to the size of the current context layout
        assert_eq!(self.layout.row_bytes() as usize * trace_rows, values.len());

        // NOTE: At the moment we only support row major layout
        // NOTE: Each time we add a trace we create a new buffer
        // TODO! Add support for column major layout

        // Check trace_rows is less than or equal to self num_rows
        assert!(trace_rows <= self.layout.num_rows());

        // Create a new buffer
        let mut trace_buffer = TraceBuffer::new(self.layout.row_bytes() as usize * self.layout.num_rows());
        let buffer = trace_buffer.buffer_mut();
        buffer[0..values.len()].copy_from_slice(values);

        self.buffers.push(trace_buffer);

        // Add all trace col segements to the context
        let mut offset = 0;
        for trace_column in self.layout.trace_columns() {
            let segment = TraceColSegment {
                column: trace_column.column_name().to_string(),
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