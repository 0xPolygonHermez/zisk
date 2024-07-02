use crate::TracePolEnum;

pub struct RomLink<T> {
    pub col: TracePolEnum<T>,
    pub binary: bool,
}

impl<T> RomLink<T> {
    pub fn new(col: TracePolEnum<T>, binary: bool) -> Self {
        RomLink { col, binary }
    }
}
