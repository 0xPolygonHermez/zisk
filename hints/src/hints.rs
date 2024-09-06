use proofman_starks_lib_c::{get_hint_field_c, get_hint_ids_by_name_c, print_expression_c, print_by_name_c, set_hint_field_c};

use p3_field::Field;
use proofman_common::{ExtensionField, AirInstanceCtx, SetupCtx};

use std::os::raw::c_void;

use std::ops::{Mul, Add, Sub, Div};

use std::fmt::Debug;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
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
    field_type: HintFieldType,
    pub values: *mut F,
}

#[repr(C)]
pub struct HintIdsResult {
    n_hints: u64,
    pub hint_ids: *mut u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HintFieldValue<F: Clone + Copy> {
    Field(F),
    FieldExtended(ExtensionField<F>),
    Column(Vec<F>),
    ColumnExtended(Vec<ExtensionField<F>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
// Define an enum to represent the possible return types
pub enum HintFieldOutput<F: Clone + Copy> {
    Field(F),
    FieldExtended(ExtensionField<F>),
}

// impl<F: Copy> Index<usize> for HintFieldValue<F> {
//     type Output = HintFieldOutput<F>;

//     fn index(&self, index: usize) -> &Self::Output {
//         match self {
//             HintFieldValue::Field(value) => &HintFieldOutput::Field(value.clone()),
//             HintFieldValue::FieldExtended(value) => &HintFieldOutput::FieldExtended(value.clone()),
//             HintFieldValue::Column(vec) => &HintFieldOutput::Field(vec[index].clone()),
//             HintFieldValue::ColumnExtended(vec) => &HintFieldOutput::FieldExtended(vec[index].clone())
//         }
//     }
// }

impl<F: Clone + Copy> HintFieldValue<F> {
    pub fn get(&self, index: usize) -> HintFieldOutput<F> {
        match self {
            HintFieldValue::Field(value) => HintFieldOutput::Field(*value),
            HintFieldValue::FieldExtended(value) => HintFieldOutput::FieldExtended(*value),
            HintFieldValue::Column(vec) => HintFieldOutput::Field(vec[index]),
            HintFieldValue::ColumnExtended(vec) => HintFieldOutput::FieldExtended(vec[index]),
        }
    }

    pub fn set(&mut self, index: usize, output: HintFieldOutput<F>) {
        match (self, output) {
            (HintFieldValue::Field(val), HintFieldOutput::Field(new_val)) => {
                *val = new_val;
            }
            (HintFieldValue::FieldExtended(val), HintFieldOutput::FieldExtended(new_val)) => {
                *val = new_val;
            }
            (HintFieldValue::Column(vec), HintFieldOutput::Field(new_val)) => {
                vec[index] = new_val;
            }
            (HintFieldValue::ColumnExtended(vec), HintFieldOutput::FieldExtended(new_val)) => {
                vec[index] = new_val;
            }
            _ => panic!("Mismatched types in set method"),
        }
    }
}

impl<F: Field> Add for HintFieldOutput<F> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        match (self, rhs) {
            // Field * Field
            (HintFieldOutput::Field(a), HintFieldOutput::Field(b)) => HintFieldOutput::Field(a + b),

            // Field * FieldExtended
            (HintFieldOutput::Field(a), HintFieldOutput::FieldExtended(b)) => HintFieldOutput::FieldExtended(b + a),

            // FieldExtended * Field
            (HintFieldOutput::FieldExtended(a), HintFieldOutput::Field(b)) => HintFieldOutput::FieldExtended(a + b),

            // FieldExtended * FieldExtended
            (HintFieldOutput::FieldExtended(a), HintFieldOutput::FieldExtended(b)) => {
                HintFieldOutput::FieldExtended(a + b)
            }
        }
    }
}

impl<F: Field> Sub for HintFieldOutput<F> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        match (self, rhs) {
            // Field * Field
            (HintFieldOutput::Field(a), HintFieldOutput::Field(b)) => HintFieldOutput::Field(a - b),

            // Field * FieldExtended
            (HintFieldOutput::Field(a), HintFieldOutput::FieldExtended(b)) => {
                HintFieldOutput::FieldExtended(ExtensionField { value: [a, F::zero(), F::zero()] } - b)
            }

            // FieldExtended * Field
            (HintFieldOutput::FieldExtended(a), HintFieldOutput::Field(b)) => HintFieldOutput::FieldExtended(a - b),

            // FieldExtended * FieldExtended
            (HintFieldOutput::FieldExtended(a), HintFieldOutput::FieldExtended(b)) => {
                HintFieldOutput::FieldExtended(a - b)
            }
        }
    }
}

impl<F: Field> Mul for HintFieldOutput<F> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        match (self, rhs) {
            // Field * Field
            (HintFieldOutput::Field(a), HintFieldOutput::Field(b)) => HintFieldOutput::Field(a * b),

            // Field * FieldExtended
            (HintFieldOutput::Field(a), HintFieldOutput::FieldExtended(b)) => HintFieldOutput::FieldExtended(b * a),

            // FieldExtended * Field
            (HintFieldOutput::FieldExtended(a), HintFieldOutput::Field(b)) => HintFieldOutput::FieldExtended(a * b),

            // FieldExtended * FieldExtended
            (HintFieldOutput::FieldExtended(a), HintFieldOutput::FieldExtended(b)) => {
                HintFieldOutput::FieldExtended(a * b)
            }
        }
    }
}

impl<F: Field> Div for HintFieldOutput<F> {
    type Output = Self;

    #[inline]
    fn div(self, rhs: Self) -> Self {
        match (self, rhs) {
            // Field * Field
            (HintFieldOutput::Field(a), HintFieldOutput::Field(b)) => HintFieldOutput::Field(a / b),

            // Field * FieldExtended
            (HintFieldOutput::Field(a), HintFieldOutput::FieldExtended(b)) => {
                HintFieldOutput::FieldExtended(b.inverse() * a)
            }

            // FieldExtended * Field
            (HintFieldOutput::FieldExtended(a), HintFieldOutput::Field(b)) => {
                HintFieldOutput::FieldExtended(a * b.inverse())
            }

            // FieldExtended * FieldExtended
            (HintFieldOutput::FieldExtended(a), HintFieldOutput::FieldExtended(b)) => {
                HintFieldOutput::FieldExtended(a / b)
            }
        }
    }
}

pub struct HintCol;

impl HintCol {
    pub fn from_hint_field<F: Clone + Copy>(hint_field: &HintFieldInfo<F>) -> HintFieldValue<F> {
        unsafe {
            match hint_field.field_type {
                HintFieldType::Field => HintFieldValue::Field(*hint_field.values),
                HintFieldType::FieldExtended => {
                    let array: [F; 3] =
                        [*hint_field.values, *hint_field.values.wrapping_add(1), *hint_field.values.wrapping_add(2)];
                    HintFieldValue::FieldExtended(ExtensionField { value: array })
                }
                HintFieldType::Column => {
                    let vec =
                        Vec::from_raw_parts(hint_field.values, hint_field.size as usize, hint_field.size as usize);
                    HintFieldValue::Column(vec)
                }
                HintFieldType::ColumnExtended => {
                    let mut extended_vec: Vec<ExtensionField<F>> = Vec::with_capacity(hint_field.size as usize / 3);
                    for i in 0..(hint_field.size as usize / 3) {
                        let base_ptr = hint_field.values.wrapping_add(i * 3);
                        extended_vec.push(ExtensionField {
                            value: [*base_ptr, *base_ptr.wrapping_add(1), *base_ptr.wrapping_add(2)],
                        });
                    }
                    HintFieldValue::ColumnExtended(extended_vec)
                }
            }
        }
    }
}

pub fn get_hint_ids_by_name(p_expressions: *mut c_void, name: &str) -> Vec<u64> {
    let raw_ptr = get_hint_ids_by_name_c(p_expressions, name);

    let hint_ids_result = unsafe { Box::from_raw(raw_ptr as *mut HintIdsResult) };

    let slice = unsafe { std::slice::from_raw_parts(hint_ids_result.hint_ids, hint_ids_result.n_hints as usize) };

    // Copy the contents of the slice into a Vec<u64>

    slice.to_vec()
}

pub fn get_hint_field<F: Clone + Copy>(
    setup_ctx: &SetupCtx,
    air_instance_ctx: &mut AirInstanceCtx<F>,
    hint_id: usize,
    hint_field_name: &str,
    dest: bool,
    inverse: bool,
    print_expression: bool,
) -> HintFieldValue<F> {
    
    let params = air_instance_ctx.params.unwrap();

    let setup = setup_ctx.get_setup(air_instance_ctx.air_group_id, air_instance_ctx.air_id).expect("REASON");

    let raw_ptr = get_hint_field_c(setup.p_expressions, params, hint_id as u64, hint_field_name, dest, inverse, print_expression);
    
    let hint_field = unsafe { Box::from_raw(raw_ptr as *mut HintFieldInfo<F>) };

    HintCol::from_hint_field(hint_field.as_ref())
}

pub fn get_hint_field_constant<F: Clone + Copy>(
    setup_ctx: &SetupCtx,
    air_group_id: usize,
    air_id: usize,
    hint_id: usize,
    hint_field_name: &str,
    dest: bool,
    print_expression: bool,
) -> HintFieldValue<F> {
    
    let setup = setup_ctx.get_setup(air_group_id, air_id).expect("REASON");

    let raw_ptr = get_hint_field_c(setup.p_expressions, std::ptr::null_mut(), hint_id as u64, hint_field_name, dest, false, print_expression);
    
    let hint_field = unsafe { Box::from_raw(raw_ptr as *mut HintFieldInfo<F>) };

    HintCol::from_hint_field(hint_field.as_ref())
}

pub fn set_hint_field<F: Copy + core::fmt::Debug>(
    setup_ctx: &SetupCtx,
    air_instance_ctx: &mut AirInstanceCtx<F>,
    hint_id: u64,
    hint_field_name: &str,
    values: &HintFieldValue<F>,
) {

    let params = air_instance_ctx.params.unwrap();

    let setup = setup_ctx.get_setup(air_instance_ctx.air_group_id, air_instance_ctx.air_id).expect("REASON");

    let values_ptr: *mut c_void = match values {
        HintFieldValue::Column(vec) => vec.as_ptr() as *mut c_void,
        HintFieldValue::ColumnExtended(vec) => vec.as_ptr() as *mut c_void,
        _ => panic!("Only column and column extended are accepted"),
    };

    let id = set_hint_field_c(setup.p_expressions, params, values_ptr, hint_id, hint_field_name);

    air_instance_ctx.set_commit_calculated(id as usize);
}

pub fn set_hint_field_val<F: Clone + Copy + std::fmt::Debug>(
    setup_ctx: &SetupCtx,
    air_instance_ctx: &mut AirInstanceCtx<F>,
    hint_id: u64,
    hint_field_name: &str,
    value: HintFieldOutput<F>,
) {
    let params = air_instance_ctx.params.unwrap();

    let setup = setup_ctx.get_setup(air_instance_ctx.air_group_id, air_instance_ctx.air_id).expect("REASON");

    let mut value_array: Vec<F> = Vec::new();

    match value {
        HintFieldOutput::Field(val) => {
            value_array.push(val);
        }
        HintFieldOutput::FieldExtended(val) => {
            value_array.push(val.value[0]);
            value_array.push(val.value[1]);
            value_array.push(val.value[2]);
        }
    };

    let values_ptr = value_array.as_mut_ptr() as *mut c_void;
     
    let id = set_hint_field_c(setup.p_expressions, params, values_ptr, hint_id, hint_field_name);

    air_instance_ctx.set_subproofvalue_calculated(id as usize);
}

pub fn print_expression<F: Clone + Copy + Debug>(
    setup_ctx: &SetupCtx,
    air_instance_ctx: &mut AirInstanceCtx<F>,
    expr: &HintFieldValue<F>,
    first_print_value: u64,
    last_print_value: u64,
) {    
    let setup = setup_ctx.get_setup(air_instance_ctx.air_group_id, air_instance_ctx.air_id).expect("REASON");
    
    match expr {
        HintFieldValue::Column(vec) => {
            print_expression_c(setup.p_expressions, vec.as_ptr() as *mut c_void, 1, first_print_value, last_print_value);
        } 
        HintFieldValue::ColumnExtended(vec) => {
            print_expression_c(setup.p_expressions, vec.as_ptr() as *mut c_void, 3, first_print_value, last_print_value);
        }
        HintFieldValue::Field(val) => {
            println!("Field value: {:?}", val);
        }
        HintFieldValue::FieldExtended(val) => {
            println!("FieldExtended values: {:?}", val);
        }
    }
        
}

pub fn print_by_name<F: Clone + Copy>(
    setup_ctx: &SetupCtx,
    air_instance_ctx: &mut AirInstanceCtx<F>,
    name: &str,
    lengths: Option<Vec<u64>>,
    first_print_value: u64,
    last_print_value: u64,
) -> Option<HintFieldValue<F>> {
    let setup = setup_ctx.get_setup(air_instance_ctx.air_group_id, air_instance_ctx.air_id).expect("REASON");

    let params = air_instance_ctx.params.unwrap();

    let lengths_ptr = lengths.as_ref().map(|lengths| lengths.clone().as_mut_ptr()).unwrap_or(std::ptr::null_mut());

    // TODO: CHECK WHAT IS WRONG WITH RETURN VALUES
    let _raw_ptr = print_by_name_c(setup.p_expressions, params, name, lengths_ptr, first_print_value, last_print_value, false);

    // if return_values {
    //     let field = unsafe { Box::from_raw(raw_ptr as *mut HintFieldInfo<F>) };

    //     Some(HintCol::from_hint_field(field.as_ref()))
    // } else {
        None
    // }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_1() {
        let mut buffer = [0usize; 90];
        for (i, item) in buffer.iter_mut().enumerate() {
            *item = i + 144;
        }
        let hint_field: HintFieldInfo<usize> = HintFieldInfo::<usize> {
            size: 1,
            offset: 1,
            field_type: HintFieldType::Field,
            values: buffer.as_mut_ptr(),
        };

        match HintCol::from_hint_field(&hint_field) {
            HintFieldValue::Field(value) => {
                assert_eq!(value, 144);
            }
            _ => panic!("Expected a field value"),
        }
    }
}
