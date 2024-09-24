use proofman_starks_lib_c::{
    get_hint_field_c, get_hint_ids_by_name_c, print_by_name_c, print_expression_c, print_row_c, set_hint_field_c,
    StepsParams,
};

use p3_field::Field;
use proofman_common::{AirInstance, Challenges, ExtensionField, ProofCtx, PublicInputs, SetupCtx};

use std::os::raw::c_void;

use std::ops::{Add, Div, Mul, Sub, AddAssign, DivAssign, MulAssign, SubAssign};

use std::fmt::Debug;
use std::sync::Arc;

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

#[derive(Default)]
pub struct HintFieldOptions {
    pub dest: bool,
    pub inverse: bool,
    pub print_expression: bool,
}

impl HintFieldOptions {
    pub fn dest() -> Self {
        Self { dest: true, ..Default::default() }
    }

    pub fn inverse() -> Self {
        Self { inverse: true, ..Default::default() }
    }

    pub fn print_expression() -> Self {
        Self { print_expression: true, ..Default::default() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HintFieldValue<F: Clone + Copy> {
    Field(F),
    FieldExtended(ExtensionField<F>),
    Column(Vec<F>),
    ColumnExtended(Vec<ExtensionField<F>>),
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
// Define an enum to represent the possible return types
pub enum HintFieldOutput<F: Clone + Copy> {
    Field(F),
    FieldExtended(ExtensionField<F>),
}

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

impl<F: Field> Add<F> for HintFieldOutput<F> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: F) -> Self {
        match self {
            HintFieldOutput::Field(a) => HintFieldOutput::Field(a + rhs),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a + rhs),
        }
    }
}

impl<F: Field> Add<ExtensionField<F>> for HintFieldOutput<F> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: ExtensionField<F>) -> Self {
        match self {
            HintFieldOutput::Field(a) => HintFieldOutput::FieldExtended(rhs + a),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a + rhs),
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

impl<F: Field> AddAssign<F> for HintFieldOutput<F> {
    #[inline]
    fn add_assign(&mut self, rhs: F) {
        *self = match *self {
            HintFieldOutput::Field(a) => HintFieldOutput::Field(a + rhs),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a + rhs),
        }
    }
}

impl<F: Field> AddAssign<ExtensionField<F>> for HintFieldOutput<F> {
    #[inline]
    fn add_assign(&mut self, rhs: ExtensionField<F>) {
        *self = match *self {
            HintFieldOutput::Field(a) => HintFieldOutput::FieldExtended(rhs + a),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a + rhs),
        }
    }
}

impl<F: Field> AddAssign<HintFieldOutput<F>> for HintFieldOutput<F> {
    #[inline]
    fn add_assign(&mut self, rhs: HintFieldOutput<F>) {
        match rhs {
            HintFieldOutput::Field(b) => match self {
                HintFieldOutput::Field(a) => *self = HintFieldOutput::Field(*a + b),
                HintFieldOutput::FieldExtended(a) => *self = HintFieldOutput::FieldExtended(*a + b),
            },
            HintFieldOutput::FieldExtended(b) => match self {
                HintFieldOutput::Field(a) => *self = HintFieldOutput::FieldExtended(b + *a),
                HintFieldOutput::FieldExtended(a) => *self = HintFieldOutput::FieldExtended(*a + b),
            },
        }
    }
}

impl<F: Field> Sub<F> for HintFieldOutput<F> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: F) -> Self {
        match self {
            HintFieldOutput::Field(a) => HintFieldOutput::Field(a - rhs),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a - rhs),
        }
    }
}

impl<F: Field> Sub<ExtensionField<F>> for HintFieldOutput<F> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: ExtensionField<F>) -> Self {
        match self {
            HintFieldOutput::Field(a) => {
                HintFieldOutput::FieldExtended(ExtensionField { value: [a, F::zero(), F::zero()] } - rhs)
            }
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a - rhs),
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

impl<F: Field> SubAssign<F> for HintFieldOutput<F> {
    #[inline]
    fn sub_assign(&mut self, rhs: F) {
        *self = match *self {
            HintFieldOutput::Field(a) => HintFieldOutput::Field(a - rhs),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a - rhs),
        }
    }
}

impl<F: Field> SubAssign<ExtensionField<F>> for HintFieldOutput<F> {
    #[inline]
    fn sub_assign(&mut self, rhs: ExtensionField<F>) {
        *self = match *self {
            HintFieldOutput::Field(a) => {
                HintFieldOutput::FieldExtended(ExtensionField { value: [a, F::zero(), F::zero()] } - rhs)
            }
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a - rhs),
        }
    }
}

impl<F: Field> SubAssign<HintFieldOutput<F>> for HintFieldOutput<F> {
    #[inline]
    fn sub_assign(&mut self, rhs: HintFieldOutput<F>) {
        match rhs {
            HintFieldOutput::Field(b) => match self {
                HintFieldOutput::Field(a) => *self = HintFieldOutput::Field(*a - b),
                HintFieldOutput::FieldExtended(a) => *self = HintFieldOutput::FieldExtended(*a - b),
            },
            HintFieldOutput::FieldExtended(b) => match self {
                HintFieldOutput::Field(a) => {
                    *self = HintFieldOutput::FieldExtended(ExtensionField { value: [*a, F::zero(), F::zero()] } - b)
                }
                HintFieldOutput::FieldExtended(a) => *self = HintFieldOutput::FieldExtended(*a - b),
            },
        }
    }
}

impl<F: Field> Mul<F> for HintFieldOutput<F> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: F) -> Self {
        match self {
            HintFieldOutput::Field(a) => HintFieldOutput::Field(a * rhs),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a * rhs),
        }
    }
}

impl<F: Field> Mul<ExtensionField<F>> for HintFieldOutput<F> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: ExtensionField<F>) -> Self {
        match self {
            HintFieldOutput::Field(a) => HintFieldOutput::FieldExtended(rhs * a),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a * rhs),
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

impl<F: Field> MulAssign<F> for HintFieldOutput<F> {
    #[inline]
    fn mul_assign(&mut self, rhs: F) {
        *self = match *self {
            HintFieldOutput::Field(a) => HintFieldOutput::Field(a * rhs),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a * rhs),
        }
    }
}

impl<F: Field> MulAssign<ExtensionField<F>> for HintFieldOutput<F> {
    #[inline]
    fn mul_assign(&mut self, rhs: ExtensionField<F>) {
        *self = match *self {
            HintFieldOutput::Field(a) => HintFieldOutput::FieldExtended(rhs * a),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a * rhs),
        }
    }
}

impl<F: Field> MulAssign<HintFieldOutput<F>> for HintFieldOutput<F> {
    #[inline]
    fn mul_assign(&mut self, rhs: HintFieldOutput<F>) {
        match rhs {
            HintFieldOutput::Field(b) => match self {
                HintFieldOutput::Field(a) => *self = HintFieldOutput::Field(*a * b),
                HintFieldOutput::FieldExtended(a) => *self = HintFieldOutput::FieldExtended(*a * b),
            },
            HintFieldOutput::FieldExtended(b) => match self {
                HintFieldOutput::Field(a) => *self = HintFieldOutput::FieldExtended(b * *a),
                HintFieldOutput::FieldExtended(a) => *self = HintFieldOutput::FieldExtended(*a * b),
            },
        }
    }
}

impl<F: Field> Div<F> for HintFieldOutput<F> {
    type Output = Self;

    #[inline]
    fn div(self, rhs: F) -> Self {
        match self {
            HintFieldOutput::Field(a) => HintFieldOutput::Field(a / rhs),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a * rhs.inverse()),
        }
    }
}

impl<F: Field> Div<ExtensionField<F>> for HintFieldOutput<F> {
    type Output = Self;

    #[inline]
    fn div(self, rhs: ExtensionField<F>) -> Self {
        match self {
            HintFieldOutput::Field(a) => HintFieldOutput::FieldExtended(rhs.inverse() * a),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a / rhs),
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

impl<F: Field> DivAssign<F> for HintFieldOutput<F> {
    #[inline]
    fn div_assign(&mut self, rhs: F) {
        *self = match *self {
            HintFieldOutput::Field(a) => HintFieldOutput::Field(a / rhs),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a * rhs.inverse()),
        }
    }
}

impl<F: Field> DivAssign<ExtensionField<F>> for HintFieldOutput<F> {
    #[inline]
    fn div_assign(&mut self, rhs: ExtensionField<F>) {
        *self = match *self {
            HintFieldOutput::Field(a) => HintFieldOutput::FieldExtended(rhs.inverse() * a),
            HintFieldOutput::FieldExtended(a) => HintFieldOutput::FieldExtended(a / rhs),
        }
    }
}

impl<F: Field> DivAssign<HintFieldOutput<F>> for HintFieldOutput<F> {
    #[inline]
    fn div_assign(&mut self, rhs: HintFieldOutput<F>) {
        match rhs {
            HintFieldOutput::Field(b) => match self {
                HintFieldOutput::Field(a) => *self = HintFieldOutput::Field(*a / b),
                HintFieldOutput::FieldExtended(a) => *self = HintFieldOutput::FieldExtended(*a * b.inverse()),
            },
            HintFieldOutput::FieldExtended(b) => match self {
                HintFieldOutput::Field(a) => *self = HintFieldOutput::FieldExtended(b.inverse() * *a),
                HintFieldOutput::FieldExtended(a) => *self = HintFieldOutput::FieldExtended(*a / b),
            },
        }
    }
}

impl<F: Field> HintFieldValue<F> {
    pub fn add(&mut self, index: usize, value: F) {
        match self {
            HintFieldValue::Field(v) => *v += value,
            HintFieldValue::FieldExtended(v) => *v += value,
            HintFieldValue::Column(vec) => vec[index] += value,
            HintFieldValue::ColumnExtended(vec) => vec[index] += value,
        };
    }

    pub fn add_e(&mut self, index: usize, value: ExtensionField<F>) {
        match self {
            HintFieldValue::FieldExtended(v) => *v += value,
            HintFieldValue::ColumnExtended(vec) => vec[index] += value,
            _ => panic!(),
        };
    }
}

impl<F: Field> HintFieldValue<F> {
    pub fn sub(&mut self, index: usize, value: F) {
        match self {
            HintFieldValue::Field(v) => *v -= value,
            HintFieldValue::FieldExtended(v) => *v -= value,
            HintFieldValue::Column(vec) => vec[index] -= value,
            HintFieldValue::ColumnExtended(vec) => vec[index] -= value,
        };
    }

    pub fn sub_e(&mut self, index: usize, value: ExtensionField<F>) {
        match self {
            HintFieldValue::FieldExtended(v) => *v -= value,
            HintFieldValue::ColumnExtended(vec) => vec[index] -= value,
            _ => panic!(),
        };
    }
}

impl<F: Field> HintFieldValue<F> {
    pub fn mul(&mut self, index: usize, value: F) {
        match self {
            HintFieldValue::Field(v) => *v *= value,
            HintFieldValue::FieldExtended(v) => *v *= value,
            HintFieldValue::Column(vec) => vec[index] *= value,
            HintFieldValue::ColumnExtended(vec) => vec[index] *= value,
        };
    }

    pub fn mul_e(&mut self, index: usize, value: ExtensionField<F>) {
        match self {
            HintFieldValue::FieldExtended(v) => *v *= value,
            HintFieldValue::ColumnExtended(vec) => vec[index] *= value,
            _ => panic!(),
        };
    }
}

impl<F: Field> HintFieldValue<F> {
    pub fn div(&mut self, index: usize, value: F) {
        match self {
            HintFieldValue::Field(v) => *v *= value.inverse(),
            HintFieldValue::FieldExtended(v) => *v *= value.inverse(),
            HintFieldValue::Column(vec) => vec[index] *= value.inverse(),
            HintFieldValue::ColumnExtended(vec) => vec[index] *= value.inverse(),
        };
    }

    pub fn div_e(&mut self, index: usize, value: ExtensionField<F>) {
        match self {
            HintFieldValue::FieldExtended(v) => *v *= value.inverse(),
            HintFieldValue::ColumnExtended(vec) => vec[index] *= value.inverse(),
            _ => panic!(),
        };
    }
}
pub struct HintCol;

impl HintCol {
    pub fn from_hint_field<F: Clone + Copy>(hint_field: &HintFieldInfo<F>) -> HintFieldValue<F> {
        let values_slice = unsafe { std::slice::from_raw_parts(hint_field.values, hint_field.size as usize) };

        match hint_field.field_type {
            HintFieldType::Field => HintFieldValue::Field(values_slice[0]),
            HintFieldType::FieldExtended => {
                let array = [values_slice[0], values_slice[1], values_slice[2]];
                HintFieldValue::FieldExtended(ExtensionField { value: array })
            }
            HintFieldType::Column => HintFieldValue::Column(values_slice.to_vec()),
            HintFieldType::ColumnExtended => {
                let mut extended_vec: Vec<ExtensionField<F>> = Vec::with_capacity(hint_field.size as usize / 3);
                for chunk in values_slice.chunks(3) {
                    extended_vec.push(ExtensionField { value: [chunk[0], chunk[1], chunk[2]] });
                }
                HintFieldValue::ColumnExtended(extended_vec)
            }
        }
    }
}

pub fn get_hint_ids_by_name(p_setup: *mut c_void, name: &str) -> Vec<u64> {
    let raw_ptr = get_hint_ids_by_name_c(p_setup, name);

    let hint_ids_result = unsafe { Box::from_raw(raw_ptr as *mut HintIdsResult) };

    let slice = unsafe { std::slice::from_raw_parts(hint_ids_result.hint_ids, hint_ids_result.n_hints as usize) };

    // Copy the contents of the slice into a Vec<u64>

    slice.to_vec()
}

pub fn get_hint_field<F: Clone + Copy + Debug>(
    setup_ctx: &SetupCtx,
    public_inputs: &PublicInputs,
    challenges: &Challenges<F>,
    air_instance: &mut AirInstance<F>,
    hint_id: usize,
    hint_field_name: &str,
    options: HintFieldOptions,
) -> HintFieldValue<F> {
    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id).expect("REASON");

    let public_inputs_ptr = (*public_inputs.inputs.read().unwrap()).as_ptr() as *mut c_void;
    let challenges_ptr = (*challenges.challenges.read().unwrap()).as_ptr() as *mut c_void;

    let steps_params = StepsParams {
        buffer: air_instance.get_buffer_ptr() as *mut c_void,
        public_inputs: public_inputs_ptr,
        challenges: challenges_ptr,
        subproof_values: air_instance.evals.as_ptr() as *mut c_void,
        evals: air_instance.subproof_values.as_ptr() as *mut c_void,
    };

    let raw_ptr = get_hint_field_c(
        setup.p_setup,
        steps_params,
        hint_id as u64,
        hint_field_name,
        options.dest,
        options.inverse,
        options.print_expression,
    );

    let hint_field = unsafe { Box::from_raw(raw_ptr as *mut HintFieldInfo<F>) };

    HintCol::from_hint_field(hint_field.as_ref())
}

pub fn get_hint_field_constant<F: Clone + Copy>(
    setup_ctx: &SetupCtx,
    airgroup_id: usize,
    air_id: usize,
    hint_id: usize,
    hint_field_name: &str,
    options: HintFieldOptions,
) -> HintFieldValue<F> {
    let setup = setup_ctx.get_setup(airgroup_id, air_id).expect("REASON");

    let steps_params = StepsParams {
        buffer: std::ptr::null_mut(),
        public_inputs: std::ptr::null_mut(),
        challenges: std::ptr::null_mut(),
        subproof_values: std::ptr::null_mut(),
        evals: std::ptr::null_mut(),
    };

    let raw_ptr = get_hint_field_c(
        setup.p_setup,
        steps_params,
        hint_id as u64,
        hint_field_name,
        options.dest,
        options.inverse,
        options.print_expression,
    );

    let hint_field = unsafe { Box::from_raw(raw_ptr as *mut HintFieldInfo<F>) };

    HintCol::from_hint_field(hint_field.as_ref())
}

pub fn set_hint_field<F: Copy + core::fmt::Debug>(
    setup_ctx: &SetupCtx,
    air_instance: &mut AirInstance<F>,
    hint_id: u64,
    hint_field_name: &str,
    values: &HintFieldValue<F>,
) {
    let buffer = air_instance.get_buffer_ptr() as *mut c_void;

    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id).expect("REASON");

    let values_ptr: *mut c_void = match values {
        HintFieldValue::Column(vec) => vec.as_ptr() as *mut c_void,
        HintFieldValue::ColumnExtended(vec) => vec.as_ptr() as *mut c_void,
        _ => panic!("Only column and column extended are accepted"),
    };

    let id = set_hint_field_c(setup.p_setup, buffer, std::ptr::null_mut(), values_ptr, hint_id, hint_field_name);

    air_instance.set_commit_calculated(id as usize);
}

pub fn set_hint_field_val<F: Clone + Copy + std::fmt::Debug>(
    setup_ctx: &SetupCtx,
    air_instance: &mut AirInstance<F>,
    hint_id: u64,
    hint_field_name: &str,
    value: HintFieldOutput<F>,
) {
    let subproof_values = air_instance.subproof_values.as_mut_ptr() as *mut c_void;

    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id).expect("REASON");

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

    let values_ptr = value_array.as_ptr() as *mut c_void;

    let id =
        set_hint_field_c(setup.p_setup, std::ptr::null_mut(), subproof_values, values_ptr, hint_id, hint_field_name);

    air_instance.set_subproofvalue_calculated(id as usize);
}

pub fn print_expression<F: Clone + Copy + Debug>(
    setup_ctx: &SetupCtx,
    air_instance: &mut AirInstance<F>,
    expr: &HintFieldValue<F>,
    first_print_value: u64,
    last_print_value: u64,
) {
    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id).expect("REASON");

    match expr {
        HintFieldValue::Column(vec) => {
            print_expression_c(setup.p_setup, vec.as_ptr() as *mut c_void, 1, first_print_value, last_print_value);
        }
        HintFieldValue::ColumnExtended(vec) => {
            print_expression_c(setup.p_setup, vec.as_ptr() as *mut c_void, 3, first_print_value, last_print_value);
        }
        HintFieldValue::Field(val) => {
            println!("Field value: {:?}", val);
        }
        HintFieldValue::FieldExtended(val) => {
            println!("FieldExtended values: {:?}", val);
        }
    }
}

pub fn print_row<F: Clone + Copy + Debug>(setup_ctx: &SetupCtx, air_instance: &AirInstance<F>, stage: u64, row: u64) {
    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id).expect("REASON");

    let buffer = air_instance.get_buffer_ptr() as *mut c_void;

    print_row_c(setup.p_setup, buffer, stage, row);
}

pub fn print_by_name<F: Clone + Copy>(
    setup_ctx: &SetupCtx,
    proof_ctx: Arc<ProofCtx<F>>,
    air_instance: &AirInstance<F>,
    name: &str,
    lengths: Option<Vec<u64>>,
    first_print_value: u64,
    last_print_value: u64,
) -> Option<HintFieldValue<F>> {
    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id).expect("REASON");
    let public_inputs_ptr = (*proof_ctx.public_inputs.inputs.read().unwrap()).as_ptr() as *mut c_void;
    let challenges_ptr = (*proof_ctx.challenges.challenges.read().unwrap()).as_ptr() as *mut c_void;

    let steps_params = StepsParams {
        buffer: air_instance.get_buffer_ptr() as *mut c_void,
        public_inputs: public_inputs_ptr,
        challenges: challenges_ptr,
        subproof_values: air_instance.evals.as_ptr() as *mut c_void,
        evals: std::ptr::null_mut(),
    };

    let mut lengths_vec = lengths.unwrap_or_default();
    let lengths_ptr = lengths_vec.as_mut_ptr();

    let _raw_ptr =
        print_by_name_c(setup.p_setup, steps_params, name, lengths_ptr, first_print_value, last_print_value, false);

    // TODO: CHECK WHAT IS WRONG WITH RETURN VALUES
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
