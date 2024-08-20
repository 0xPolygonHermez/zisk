use p3_field::AbstractField;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintFieldType {
    Field = 0,
    FieldExtended = 1,
    Column = 2,
    ColumnExtended = 3,
}

#[repr(C)]
pub struct HintFieldInfo<F: AbstractField> {
    size: u64,
    type_: HintFieldType, 
    dest: *mut F,
}

impl<F: AbstractField> HintFieldInfo<F> {
    pub fn get_size(&self) -> u64 {
        self.size
    }

    pub fn get_dest(&self) -> *mut F {
        self.dest
    }

    pub fn get_type(&self) -> HintFieldType {
        self.type_
    }
}