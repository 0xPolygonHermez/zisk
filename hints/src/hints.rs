use proofman_starks_lib_c::{
    acc_hint_field_c, acc_mul_hint_fields_c, get_hint_field_c, get_hint_ids_by_name_c, mul_hint_fields_c,
    print_expression_c, print_row_c, set_hint_field_c, VecU64Result,
};

use std::collections::HashMap;

use p3_field::Field;
use proofman_common::{AirInstance, ExtensionField, ProofCtx, SetupCtx, StepsParams};

use std::os::raw::c_void;

use std::ops::{Add, Div, Mul, Sub, AddAssign, DivAssign, MulAssign, SubAssign};

use std::fmt::{Display, Debug, Formatter, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub enum HintFieldType {
    Field = 0,          // F
    FieldExtended = 1,  // [F; 3]
    Column = 2,         // Vec<F>
    ColumnExtended = 3, // Vec<[F;3]>
    String = 4,
}

#[repr(C)]
#[allow(dead_code)]
pub struct HintFieldInfo<F: Field> {
    size: u64,
    offset: u8, // 1 or 3cd
    field_type: HintFieldType,
    values: *mut F,
    string_value: *mut u8,
    pub matrix_size: u64,
    pub pos: *mut u64,
}

#[repr(C)]
pub struct HintFieldInfoValues<F: Field> {
    pub n_values: u64,
    pub hint_field_values: *mut HintFieldInfo<F>,
}

#[repr(C)]
#[derive(Default)]
pub struct HintFieldOptions {
    pub dest: bool,
    pub inverse: bool,
    pub print_expression: bool,
    pub initialize_zeros: bool,
    pub compilation_time: bool,
}

impl From<&HintFieldOptions> for *mut c_void {
    fn from(options: &HintFieldOptions) -> *mut c_void {
        options as *const HintFieldOptions as *mut c_void
    }
}

impl HintFieldOptions {
    pub fn dest() -> Self {
        Self { dest: true, ..Default::default() }
    }

    pub fn dest_with_zeros() -> Self {
        Self { dest: true, initialize_zeros: true, ..Default::default() }
    }

    pub fn inverse() -> Self {
        Self { inverse: true, ..Default::default() }
    }

    pub fn compilation_time() -> Self {
        Self { compilation_time: true, ..Default::default() }
    }

    pub fn inverse_and_print_expression() -> Self {
        Self { inverse: true, print_expression: true, ..Default::default() }
    }

    pub fn print_expression() -> Self {
        Self { print_expression: true, ..Default::default() }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HintFieldValue<F: Field> {
    Field(F),
    FieldExtended(ExtensionField<F>),
    Column(Vec<F>),
    ColumnExtended(Vec<ExtensionField<F>>),
    String(String),
}

impl<F: Field> Display for HintFieldValue<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            HintFieldValue::Field(value) => write!(f, "{}", value),
            HintFieldValue::FieldExtended(ext_field) => write!(f, "{}", ext_field),
            HintFieldValue::Column(column) => {
                let formatted: Vec<String> = column.iter().map(|v| format!("{}", v)).collect();
                write!(f, "[{}]", formatted.join(", "))
            }
            HintFieldValue::ColumnExtended(ext_column) => {
                let formatted: Vec<String> = ext_column.iter().map(|v| format!("{}", v)).collect();
                write!(f, "[{}]", formatted.join(", "))
            }
            HintFieldValue::String(s) => write!(f, "{}", s),
        }
    }
}

pub struct HintFieldValues<F: Field> {
    pub values: HashMap<Vec<u64>, HintFieldValue<F>>,
}

impl<F: Field> HintFieldValues<F> {
    pub fn get(&self, index: usize) -> HashMap<Vec<u64>, HintFieldOutput<F>> {
        self.values.iter().map(|(key, value)| (key.clone(), value.get(index))).collect()
    }
}

#[derive(Clone, Debug)]
pub struct HintFieldValuesVec<F: Field> {
    pub values: Vec<HintFieldValue<F>>,
}

impl<F: Field> HintFieldValuesVec<F> {
    pub fn get(&self, index: usize) -> Vec<HintFieldOutput<F>> {
        self.values.iter().map(|value| value.get(index)).collect()
    }
}

impl<F: Field> Display for HintFieldValuesVec<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "[")?;
        for (i, value) in self.values.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", value)?;
        }
        write!(f, "]")
    }
}

#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
// Define an enum to represent the possible return types
pub enum HintFieldOutput<F: Clone + Copy + Display> {
    Field(F),
    FieldExtended(ExtensionField<F>),
}

impl<F: Clone + Copy + Display> Display for HintFieldOutput<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            HintFieldOutput::Field(value) => write!(f, "{}", value),
            HintFieldOutput::FieldExtended(ext_field) => write!(f, "{}", ext_field),
        }
    }
}

pub fn format_vec<T: Copy + Clone + Debug + Display>(vec: &[T]) -> String {
    format!("[{}]", vec.iter().map(|item| item.to_string()).collect::<Vec<String>>().join(", "))
}

impl<F: Field> HintFieldValue<F> {
    pub fn get(&self, index: usize) -> HintFieldOutput<F> {
        match self {
            HintFieldValue::Field(value) => HintFieldOutput::Field(*value),
            HintFieldValue::FieldExtended(value) => HintFieldOutput::FieldExtended(*value),
            HintFieldValue::Column(vec) => HintFieldOutput::Field(vec[index]),
            HintFieldValue::ColumnExtended(vec) => HintFieldOutput::FieldExtended(vec[index]),
            HintFieldValue::String(_str) => panic!(),
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
            HintFieldValue::String(_str) => panic!(),
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
            HintFieldValue::String(_str) => panic!(),
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
            HintFieldValue::String(_str) => panic!(),
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
            HintFieldValue::String(_str) => panic!(),
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
    pub fn from_hint_field<F: Field>(hint_field: &HintFieldInfo<F>) -> HintFieldValue<F> {
        let values_slice = match hint_field.field_type {
            HintFieldType::String => &[],
            _ => unsafe { std::slice::from_raw_parts(hint_field.values, hint_field.size as usize) },
        };

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
            HintFieldType::String => {
                let str_slice =
                    unsafe { std::slice::from_raw_parts(hint_field.string_value, hint_field.size as usize) };

                match std::str::from_utf8(str_slice) {
                    Ok(value) => HintFieldValue::String(value.to_string()),
                    Err(_) => HintFieldValue::String(String::new()),
                }
            }
        }
    }
}

pub fn get_hint_ids_by_name(p_expressions_bin: *mut c_void, name: &str) -> Vec<u64> {
    let raw_ptr = get_hint_ids_by_name_c(p_expressions_bin, name);

    let hint_ids_result = unsafe { Box::from_raw(raw_ptr as *mut VecU64Result) };

    let slice = unsafe { std::slice::from_raw_parts(hint_ids_result.values, hint_ids_result.n_values as usize) };

    // Copy the contents of the slice into a Vec<u64>

    slice.to_vec()
}

#[allow(clippy::too_many_arguments)]
pub fn mul_hint_fields<F: Field + Field>(
    setup_ctx: &SetupCtx,
    proof_ctx: &ProofCtx<F>,
    air_instance: &mut AirInstance<F>,
    hint_id: usize,
    hint_field_dest: &str,
    hint_field_name1: &str,
    options1: HintFieldOptions,
    hint_field_name2: &str,
    options2: HintFieldOptions,
) -> u64 {
    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id);

    let public_inputs_ptr = (*proof_ctx.public_inputs.inputs.read().unwrap()).as_ptr() as *mut c_void;
    let challenges_ptr = (*proof_ctx.challenges.challenges.read().unwrap()).as_ptr() as *mut c_void;

    let const_pols_ptr = (*setup.const_pols.values.read().unwrap()).as_ptr() as *mut c_void;
    let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;

    let steps_params = StepsParams {
        trace: air_instance.get_trace_ptr() as *mut c_void,
        pols: air_instance.get_buffer_ptr() as *mut c_void,
        public_inputs: public_inputs_ptr,
        challenges: challenges_ptr,
        airgroup_values: air_instance.airgroup_values.as_ptr() as *mut c_void,
        airvalues: air_instance.airvalues.as_ptr() as *mut c_void,
        evals: air_instance.evals.as_ptr() as *mut c_void,
        xdivxsub: std::ptr::null_mut(),
        p_const_pols: const_pols_ptr,
        p_const_tree: const_tree_ptr,
        custom_commits: air_instance.get_custom_commits_ptr(),
    };

    mul_hint_fields_c(
        (&setup.p_setup).into(),
        (&steps_params).into(),
        hint_id as u64,
        hint_field_dest,
        hint_field_name1,
        hint_field_name2,
        (&options1).into(),
        (&options2).into(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn acc_hint_field<F: Field>(
    setup_ctx: &SetupCtx,
    proof_ctx: &ProofCtx<F>,
    air_instance: &mut AirInstance<F>,
    hint_id: usize,
    hint_field_dest: &str,
    hint_field_airgroupvalue: &str,
    hint_field_name: &str,
    add: bool,
) -> (u64, u64) {
    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id);

    let public_inputs_ptr = (*proof_ctx.public_inputs.inputs.read().unwrap()).as_ptr() as *mut c_void;
    let challenges_ptr = (*proof_ctx.challenges.challenges.read().unwrap()).as_ptr() as *mut c_void;

    let const_pols_ptr = (*setup.const_pols.values.read().unwrap()).as_ptr() as *mut c_void;
    let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;

    let steps_params = StepsParams {
        trace: air_instance.get_trace_ptr() as *mut c_void,
        pols: air_instance.get_buffer_ptr() as *mut c_void,
        public_inputs: public_inputs_ptr,
        challenges: challenges_ptr,
        airgroup_values: air_instance.airgroup_values.as_ptr() as *mut c_void,
        airvalues: air_instance.airvalues.as_ptr() as *mut c_void,
        evals: air_instance.evals.as_ptr() as *mut c_void,
        xdivxsub: std::ptr::null_mut(),
        p_const_pols: const_pols_ptr,
        p_const_tree: const_tree_ptr,
        custom_commits: air_instance.get_custom_commits_ptr(),
    };

    let raw_ptr = acc_hint_field_c(
        (&setup.p_setup).into(),
        (&steps_params).into(),
        hint_id as u64,
        hint_field_dest,
        hint_field_airgroupvalue,
        hint_field_name,
        add,
    );

    let hint_ids_result = unsafe { Box::from_raw(raw_ptr as *mut VecU64Result) };

    let slice = unsafe { std::slice::from_raw_parts(hint_ids_result.values, hint_ids_result.n_values as usize) };

    (slice[0], slice[1])
}

#[allow(clippy::too_many_arguments)]
pub fn acc_mul_hint_fields<F: Field>(
    setup_ctx: &SetupCtx,
    proof_ctx: &ProofCtx<F>,
    air_instance: &mut AirInstance<F>,
    hint_id: usize,
    hint_field_dest: &str,
    hint_field_airgroupvalue: &str,
    hint_field_name1: &str,
    hint_field_name2: &str,
    options1: HintFieldOptions,
    options2: HintFieldOptions,
    add: bool,
) -> (u64, u64) {
    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id);

    let public_inputs_ptr = (*proof_ctx.public_inputs.inputs.read().unwrap()).as_ptr() as *mut c_void;
    let challenges_ptr = (*proof_ctx.challenges.challenges.read().unwrap()).as_ptr() as *mut c_void;

    let const_pols_ptr = (*setup.const_pols.values.read().unwrap()).as_ptr() as *mut c_void;
    let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;

    let steps_params = StepsParams {
        trace: air_instance.get_trace_ptr() as *mut c_void,
        pols: air_instance.get_buffer_ptr() as *mut c_void,
        public_inputs: public_inputs_ptr,
        challenges: challenges_ptr,
        airgroup_values: air_instance.airgroup_values.as_ptr() as *mut c_void,
        airvalues: air_instance.airvalues.as_ptr() as *mut c_void,
        evals: air_instance.evals.as_ptr() as *mut c_void,
        xdivxsub: std::ptr::null_mut(),
        p_const_pols: const_pols_ptr,
        p_const_tree: const_tree_ptr,
        custom_commits: air_instance.get_custom_commits_ptr(),
    };

    let raw_ptr = acc_mul_hint_fields_c(
        (&setup.p_setup).into(),
        (&steps_params).into(),
        hint_id as u64,
        hint_field_dest,
        hint_field_airgroupvalue,
        hint_field_name1,
        hint_field_name2,
        (&options1).into(),
        (&options2).into(),
        add,
    );

    let hint_ids_result = unsafe { Box::from_raw(raw_ptr as *mut VecU64Result) };

    let slice = unsafe { std::slice::from_raw_parts(hint_ids_result.values, hint_ids_result.n_values as usize) };

    (slice[0], slice[1])
}

pub fn get_hint_field<F: Field>(
    setup_ctx: &SetupCtx,
    proof_ctx: &ProofCtx<F>,
    air_instance: &mut AirInstance<F>,
    hint_id: usize,
    hint_field_name: &str,
    options: HintFieldOptions,
) -> HintFieldValue<F> {
    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id);

    let public_inputs_ptr = (*proof_ctx.public_inputs.inputs.read().unwrap()).as_ptr() as *mut c_void;
    let challenges_ptr = (*proof_ctx.challenges.challenges.read().unwrap()).as_ptr() as *mut c_void;

    let const_pols_ptr = (*setup.const_pols.values.read().unwrap()).as_ptr() as *mut c_void;
    let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;

    let steps_params = StepsParams {
        trace: air_instance.get_trace_ptr() as *mut c_void,
        pols: air_instance.get_buffer_ptr() as *mut c_void,
        public_inputs: public_inputs_ptr,
        challenges: challenges_ptr,
        airgroup_values: air_instance.airgroup_values.as_ptr() as *mut c_void,
        airvalues: air_instance.airvalues.as_ptr() as *mut c_void,
        evals: air_instance.evals.as_ptr() as *mut c_void,
        xdivxsub: std::ptr::null_mut(),
        p_const_pols: const_pols_ptr,
        p_const_tree: const_tree_ptr,
        custom_commits: air_instance.get_custom_commits_ptr(),
    };

    let raw_ptr = get_hint_field_c(
        (&setup.p_setup).into(),
        (&steps_params).into(),
        hint_id as u64,
        hint_field_name,
        (&options).into(),
    );

    unsafe {
        let hint_field_values = &*(raw_ptr as *mut HintFieldInfoValues<F>);
        let value = &*(hint_field_values.hint_field_values.add(0));
        if value.matrix_size != 0 {
            panic!("get_hint_field can only be called with single expressions, but {} is an array", hint_field_name);
        }
        HintCol::from_hint_field(value)
    }
}

pub fn get_hint_field_a<F: Field>(
    setup_ctx: &SetupCtx,
    proof_ctx: &ProofCtx<F>,
    air_instance: &mut AirInstance<F>,
    hint_id: usize,
    hint_field_name: &str,
    options: HintFieldOptions,
) -> HintFieldValuesVec<F> {
    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id);

    let public_inputs_ptr = (*proof_ctx.public_inputs.inputs.read().unwrap()).as_ptr() as *mut c_void;
    let challenges_ptr = (*proof_ctx.challenges.challenges.read().unwrap()).as_ptr() as *mut c_void;

    let const_pols_ptr = (*setup.const_pols.values.read().unwrap()).as_ptr() as *mut c_void;
    let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;

    let steps_params = StepsParams {
        trace: air_instance.get_trace_ptr() as *mut c_void,
        pols: air_instance.get_buffer_ptr() as *mut c_void,
        public_inputs: public_inputs_ptr,
        challenges: challenges_ptr,
        airgroup_values: air_instance.airgroup_values.as_ptr() as *mut c_void,
        airvalues: air_instance.airvalues.as_ptr() as *mut c_void,
        evals: air_instance.evals.as_ptr() as *mut c_void,
        xdivxsub: std::ptr::null_mut(),
        p_const_pols: const_pols_ptr,
        p_const_tree: const_tree_ptr,
        custom_commits: air_instance.get_custom_commits_ptr(),
    };

    let raw_ptr = get_hint_field_c(
        (&setup.p_setup).into(),
        (&steps_params).into(),
        hint_id as u64,
        hint_field_name,
        (&options).into(),
    );

    unsafe {
        let mut hint_field_values = Vec::new();
        let hint_field = &*(raw_ptr as *mut HintFieldInfoValues<F>);
        for v in 0..hint_field.n_values {
            let h = &*(hint_field.hint_field_values.add(v as usize));
            if v == 0 && h.matrix_size != 1 {
                panic!("get_hint_field_m can only be called with an array of expressions!");
            }
            let hint_value = HintCol::from_hint_field(h);
            hint_field_values.push(hint_value);
        }

        HintFieldValuesVec { values: hint_field_values }
    }
}

pub fn get_hint_field_m<F: Field>(
    setup_ctx: &SetupCtx,
    proof_ctx: &ProofCtx<F>,
    air_instance: &mut AirInstance<F>,
    hint_id: usize,
    hint_field_name: &str,
    options: HintFieldOptions,
) -> HintFieldValues<F> {
    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id);

    let public_inputs_ptr = (*proof_ctx.public_inputs.inputs.read().unwrap()).as_ptr() as *mut c_void;
    let challenges_ptr = (*proof_ctx.challenges.challenges.read().unwrap()).as_ptr() as *mut c_void;

    let const_pols_ptr = (*setup.const_pols.values.read().unwrap()).as_ptr() as *mut c_void;
    let const_tree_ptr = (*setup.const_tree.values.read().unwrap()).as_ptr() as *mut c_void;

    let steps_params = StepsParams {
        trace: air_instance.get_trace_ptr() as *mut c_void,
        pols: air_instance.get_buffer_ptr() as *mut c_void,
        public_inputs: public_inputs_ptr,
        challenges: challenges_ptr,
        airgroup_values: air_instance.airgroup_values.as_ptr() as *mut c_void,
        airvalues: air_instance.airvalues.as_ptr() as *mut c_void,
        evals: air_instance.evals.as_ptr() as *mut c_void,
        xdivxsub: std::ptr::null_mut(),
        p_const_pols: const_pols_ptr,
        p_const_tree: const_tree_ptr,
        custom_commits: air_instance.get_custom_commits_ptr(),
    };

    let raw_ptr = get_hint_field_c(
        (&setup.p_setup).into(),
        (&steps_params).into(),
        hint_id as u64,
        hint_field_name,
        (&options).into(),
    );

    unsafe {
        let hint_field = &*(raw_ptr as *mut HintFieldInfoValues<F>);
        let mut hint_field_values = HashMap::with_capacity(hint_field.n_values as usize);

        for v in 0..hint_field.n_values {
            let h = &*(hint_field.hint_field_values.add(v as usize));
            if v == 0 && h.matrix_size > 2 {
                panic!("get_hint_field_m can only be called with a matrix of expressions!",);
            }
            let hint_value = HintCol::from_hint_field(h);
            let mut pos = Vec::new();
            for p in 0..h.matrix_size {
                pos.push(h.pos.wrapping_add(p as usize) as u64);
            }
            hint_field_values.insert(pos, hint_value);
        }

        HintFieldValues { values: hint_field_values }
    }
}

pub fn get_hint_field_constant<F: Field>(
    setup_ctx: &SetupCtx,
    airgroup_id: usize,
    air_id: usize,
    hint_id: usize,
    hint_field_name: &str,
    mut options: HintFieldOptions,
) -> HintFieldValue<F> {
    options.compilation_time = true;

    let setup = setup_ctx.get_setup(airgroup_id, air_id);

    let steps_params = StepsParams::default();

    let raw_ptr = get_hint_field_c(
        (&setup.p_setup).into(),
        (&steps_params).into(),
        hint_id as u64,
        hint_field_name,
        (&options).into(),
    );

    unsafe {
        let hint_field_values = &*(raw_ptr as *mut HintFieldInfoValues<F>);
        let value = &*(hint_field_values.hint_field_values.add(0));
        if value.matrix_size != 0 {
            panic!("get_hint_field can only be called with single expressions, but {} is an array", hint_field_name);
        }
        HintCol::from_hint_field(value)
    }
}

pub fn get_hint_field_constant_a<F: Field>(
    setup_ctx: &SetupCtx,
    airgroup_id: usize,
    air_id: usize,
    hint_id: usize,
    hint_field_name: &str,
    mut options: HintFieldOptions,
) -> Vec<HintFieldValue<F>> {
    options.compilation_time = true;

    let setup = setup_ctx.get_setup(airgroup_id, air_id);

    let steps_params = StepsParams::default();

    let raw_ptr = get_hint_field_c(
        (&setup.p_setup).into(),
        (&steps_params).into(),
        hint_id as u64,
        hint_field_name,
        (&options).into(),
    );

    unsafe {
        let mut hint_field_values = Vec::new();
        let hint_field = &*(raw_ptr as *mut HintFieldInfoValues<F>);
        for v in 0..hint_field.n_values {
            let h = &*(hint_field.hint_field_values.add(v as usize));
            if v == 0 && h.matrix_size != 1 {
                panic!("get_hint_field_m can only be called with an array of expressions!");
            }
            let hint_value = HintCol::from_hint_field(h);
            hint_field_values.push(hint_value);
        }

        hint_field_values
    }
}

pub fn get_hint_field_constant_m<F: Field>(
    setup_ctx: &SetupCtx,
    airgroup_id: usize,
    air_id: usize,
    hint_id: usize,
    hint_field_name: &str,
    mut options: HintFieldOptions,
) -> HintFieldValues<F> {
    options.compilation_time = true;

    let setup = setup_ctx.get_setup(airgroup_id, air_id);

    let steps_params = StepsParams::default();

    let raw_ptr = get_hint_field_c(
        (&setup.p_setup).into(),
        (&steps_params).into(),
        hint_id as u64,
        hint_field_name,
        (&options).into(),
    );

    unsafe {
        let hint_field = &*(raw_ptr as *mut HintFieldInfoValues<F>);
        let mut hint_field_values = HashMap::with_capacity(hint_field.n_values as usize);

        for v in 0..hint_field.n_values {
            let h = &*(hint_field.hint_field_values.add(v as usize));
            if v == 0 && h.matrix_size != 0 {
                panic!(
                    "get_hint_field_m can only be called with arrays of expressions, but {} is a single one",
                    hint_field_name
                );
            }
            let hint_value = HintCol::from_hint_field(h);
            let mut pos = Vec::new();
            for p in 0..h.matrix_size {
                pos.push(h.pos.wrapping_add(p as usize) as u64);
            }
            hint_field_values.insert(pos, hint_value);
        }

        HintFieldValues { values: hint_field_values }
    }
}

pub fn set_hint_field<F: Field>(
    setup_ctx: &SetupCtx,
    air_instance: &mut AirInstance<F>,
    hint_id: u64,
    hint_field_name: &str,
    values: &HintFieldValue<F>,
) {
    let steps_params = StepsParams {
        trace: air_instance.get_trace_ptr() as *mut c_void,
        pols: air_instance.get_buffer_ptr() as *mut c_void,
        public_inputs: std::ptr::null_mut(),
        challenges: std::ptr::null_mut(),
        airgroup_values: std::ptr::null_mut(),
        airvalues: std::ptr::null_mut(),
        evals: std::ptr::null_mut(),
        xdivxsub: std::ptr::null_mut(),
        p_const_pols: std::ptr::null_mut(),
        p_const_tree: std::ptr::null_mut(),
        custom_commits: [std::ptr::null_mut(); 10],
    };

    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id);

    let values_ptr: *mut c_void = match values {
        HintFieldValue::Column(vec) => vec.as_ptr() as *mut c_void,
        HintFieldValue::ColumnExtended(vec) => vec.as_ptr() as *mut c_void,
        _ => panic!("Only column and column extended are accepted"),
    };

    let id = set_hint_field_c((&setup.p_setup).into(), (&steps_params).into(), values_ptr, hint_id, hint_field_name);

    air_instance.set_commit_calculated(id as usize);
}

pub fn set_hint_field_val<F: Field>(
    setup_ctx: &SetupCtx,
    air_instance: &mut AirInstance<F>,
    hint_id: u64,
    hint_field_name: &str,
    value: HintFieldOutput<F>,
) {
    let steps_params = StepsParams {
        trace: std::ptr::null_mut(),
        pols: std::ptr::null_mut(),
        public_inputs: std::ptr::null_mut(),
        challenges: std::ptr::null_mut(),
        airgroup_values: air_instance.airgroup_values.as_mut_ptr() as *mut c_void,
        airvalues: air_instance.airvalues.as_mut_ptr() as *mut c_void,
        evals: std::ptr::null_mut(),
        xdivxsub: std::ptr::null_mut(),
        p_const_pols: std::ptr::null_mut(),
        p_const_tree: std::ptr::null_mut(),
        custom_commits: [std::ptr::null_mut(); 10],
    };

    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id);

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

    let id = set_hint_field_c((&setup.p_setup).into(), (&steps_params).into(), values_ptr, hint_id, hint_field_name);

    air_instance.set_airgroupvalue_calculated(id as usize);
}

pub fn print_expression<F: Field>(
    setup_ctx: &SetupCtx,
    air_instance: &mut AirInstance<F>,
    expr: &HintFieldValue<F>,
    first_print_value: u64,
    last_print_value: u64,
) {
    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id);

    match expr {
        HintFieldValue::Column(vec) => {
            print_expression_c(
                (&setup.p_setup).into(),
                vec.as_ptr() as *mut c_void,
                1,
                first_print_value,
                last_print_value,
            );
        }
        HintFieldValue::ColumnExtended(vec) => {
            print_expression_c(
                (&setup.p_setup).into(),
                vec.as_ptr() as *mut c_void,
                3,
                first_print_value,
                last_print_value,
            );
        }
        HintFieldValue::Field(val) => {
            println!("Field value: {:?}", val);
        }
        HintFieldValue::FieldExtended(val) => {
            println!("FieldExtended values: {:?}", val);
        }
        HintFieldValue::String(_str) => panic!(),
    }
}

pub fn print_row<F: Field>(setup_ctx: &SetupCtx, air_instance: &AirInstance<F>, stage: u64, row: u64) {
    let setup = setup_ctx.get_setup(air_instance.airgroup_id, air_instance.air_id);

    let buffer = match stage == 1 {
        true => air_instance.get_trace_ptr() as *mut c_void,
        false => air_instance.get_buffer_ptr() as *mut c_void,
    };

    print_row_c((&setup.p_setup).into(), buffer, stage, row);
}
