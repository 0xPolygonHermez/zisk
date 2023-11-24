// TRACE TRAIT
// ================================================================================================
#[derive(Debug, Clone, PartialEq)]
pub struct TraceColumn {
    column_name: String,
    column_bytes: u32,
    pos_bytes: u32,
}

#[allow(dead_code)]
impl TraceColumn {
    pub fn new(column_name: &str, column_bytes: u32) -> TraceColumn {
        TraceColumn {
            column_name: String::from(column_name),
            column_bytes,
            pos_bytes: 0_u32,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------
    pub fn column_name(&self) -> &str {
        &self.column_name
    }

    pub fn column_bytes(&self) -> u32 {
        self.column_bytes
    }

    pub fn pos_bytes(&self) -> u32 {
        self.pos_bytes
    }
}