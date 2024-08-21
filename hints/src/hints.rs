use starks_lib_c::get_hint_field_c;

use std::ops::Index;
use ::std::os::raw::c_void;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HintFieldType {
    Field = 0,          // F
    FieldExtended = 1,  // [F; 3]
    Column = 2,         // Vec<F>
    ColumnExtended = 3, // Vec<[F;3]>
}

#[repr(C)]
#[allow(dead_code)]
pub struct HintFieldInfo<F> {
    size: u64,
    offset: u8, // 1 or 3
    type_: HintFieldType,
    pub values: *mut F,
}

pub enum HintFieldValue<F> {
    Field(F),
    FieldExtended([F; 3]),
    Column(Vec<F>),
    ColumnExtended(Vec<[F; 3]>),
}

// pub enum HintFieldReference<'a, F> {
//     Field(&'a F),
//     FieldArray(&'a [F; 3]),
// }

// impl<'a, F> Index<usize> for HintFieldValue<F> {
//     type Output = HintFieldReference<'a, F>;

//     fn index(&self, index: usize) -> &Self::Output {
//         match self {
//             HintFieldValue::Field(value) => self,
//             HintFieldValue::FieldExtended(array) => self,
//             HintFieldValue::Column(vec) => &self[index],
//             HintFieldValue::ColumnExtended(vec) => &self[index],
//         }
//     }
// }

impl<F> Index<usize> for HintFieldValue<F> {
    type Output = HintFieldValue<F>;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            HintFieldValue::Field(_value) => self,
            HintFieldValue::FieldExtended(_array) => self,
            HintFieldValue::Column(_vec) => &self[index],
            HintFieldValue::ColumnExtended(_vec) => &self[index],
        }
    }
}

pub struct HintCol;

impl HintCol {
    pub fn from_hint_field<F: Clone>(hint_field: &HintFieldInfo<F>) -> HintFieldValue<F> {
        unsafe {
            match hint_field.type_ {
                HintFieldType::Field => {
                    // Dereference the first element in the raw pointer
                    HintFieldValue::Field((*hint_field.values).clone())
                }
                HintFieldType::FieldExtended => {
                    // Create an array [F; 3] from the first three elements in the raw pointer
                    let array: [F; 3] = [
                        (*hint_field.values).clone(),
                        (*hint_field.values.wrapping_add(1)).clone(),
                        (*hint_field.values.wrapping_add(2)).clone(),
                    ];
                    HintFieldValue::FieldExtended(array)
                }
                HintFieldType::Column => {
                    let vec =
                        Vec::from_raw_parts(hint_field.values, hint_field.size as usize, hint_field.size as usize);
                    HintFieldValue::Column(vec)
                }
                HintFieldType::ColumnExtended => {
                    let mut extended_vec: Vec<[F; 3]> = Vec::with_capacity(hint_field.size as usize / 3);
                    for i in 0..(hint_field.size as usize / 3) {
                        let base_ptr = hint_field.values.wrapping_add(i * 3);
                        extended_vec.push([
                            (*base_ptr).clone(),
                            (*base_ptr.wrapping_add(1)).clone(),
                            (*base_ptr.wrapping_add(2)).clone(),
                        ]);
                    }
                    HintFieldValue::ColumnExtended(extended_vec)
                }
            }
        }
    }
}

pub fn get_hint_field<F: Clone>(
    p_chelpers_steps: *mut c_void,
    hint_id: u64,
    hint_field_name: &str,
    dest: bool,
) -> HintFieldValue<F> {
    let raw_ptr = get_hint_field_c(p_chelpers_steps, hint_id, hint_field_name, dest);

    let hint_field = unsafe { Box::from_raw(raw_ptr as *mut HintFieldInfo<F>) };

    HintCol::from_hint_field(hint_field.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_1() {
        let mut buffer = [0usize; 90];
        for i in 0..buffer.len() {
            buffer[i] = i + 144;
        }

        let hint_field: HintFieldInfo<usize> =
            HintFieldInfo::<usize> { size: 1, offset: 1, type_: HintFieldType::Field, values: buffer.as_mut_ptr() };

        match HintCol::from_hint_field(&hint_field) {
            HintFieldValue::Field(value) => {
                assert_eq!(value, 144);
            }
            _ => panic!("Expected a field value"),
        }
    }
}
