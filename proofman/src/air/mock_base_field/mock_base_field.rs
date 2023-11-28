use std::fmt;

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum BaseFieldType {
    NoExtended,
    Extended,
}

// MOCK BASE FIELD TRAIT
// ================================================================================================
#[derive(Clone, PartialEq)]
pub struct MockBaseField {
    value: Vec<u8>,
    field_type: BaseFieldType,
}

impl fmt::Debug for MockBaseField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value[0])
    }
}

#[allow(dead_code)]
impl MockBaseField {
    pub const SIZE: u32 = 1;
    pub const EXTENDED_SIZE: u32 = 3;

    pub fn new(field_type: BaseFieldType, value: &[u8]) -> MockBaseField {
        if field_type == BaseFieldType::Extended {
            assert!(value.len() == 3);
        }
        else {
            assert!(value.len() <= 1);
        }

        MockBaseField {
            value: value.to_vec(),
            field_type,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------
    pub fn value(&self) -> &[u8] {
        &self.value
    }

    pub fn field_type(&self) -> &BaseFieldType {
        &self.field_type
    }

    pub fn from_raw_parts(value: &[u8]) -> MockBaseField {
        let field_type = if value.len() == 3 {
            BaseFieldType::Extended
        } else {
            BaseFieldType::NoExtended
        };

        MockBaseField {
            value: value.to_vec(),
            field_type,
        }
    }
}
