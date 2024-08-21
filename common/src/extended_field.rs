use p3_field::Field;
use core::array;
use core::iter::{Product, Sum};
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ExtensionField<F> {
    value: [F; 3],
}

impl<F: Field> ExtensionField<F> {
    pub fn zero() -> Self {
        Self { value: field_to_array::<F>(F::zero()) }
    }
    pub fn one() -> Self {
        Self { value: field_to_array::<F>(F::one()) }
    }
    pub fn two() -> Self {
        Self { value: field_to_array::<F>(F::two()) }
    }
    pub fn neg_one() -> Self {
        Self { value: field_to_array::<F>(F::neg_one()) }
    }

    #[inline(always)]
    pub fn square(&self) -> Self {
        Self { value: cubic_square(&self.value).to_vec().try_into().unwrap() }
    }

    pub fn from_array(arr: &[F]) -> Self {
        // Ensure the array has the correct size
        assert!(arr.len() == 3, "Array must have length 3");

        let mut value: [F; 3] = Default::default();
        value.copy_from_slice(arr);

        Self { value }
    }
}

impl<F: Field> Add for ExtensionField<F> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self {
        let mut res = self.value;
        for (r, rhs_val) in res.iter_mut().zip(rhs.value) {
            *r += rhs_val;
        }
        Self { value: res }
    }
}

impl<F: Field> Add<F> for ExtensionField<F> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: F) -> Self {
        let mut res = self.value;
        res[0] += rhs;
        Self { value: res }
    }
}

impl<F: Field> AddAssign for ExtensionField<F> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<F: Field> AddAssign<F> for ExtensionField<F> {
    fn add_assign(&mut self, rhs: F) {
        *self = *self + rhs;
    }
}

impl<F: Field> Sum for ExtensionField<F> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        let zero = Self { value: field_to_array::<F>(F::zero()) };
        iter.fold(zero, |acc, x| acc + x)
    }
}

impl<F: Field> Sub for ExtensionField<F> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self {
        let mut res = self.value;
        for (r, rhs_val) in res.iter_mut().zip(rhs.value) {
            *r -= rhs_val;
        }
        Self { value: res }
    }
}

impl<F: Field> Sub<F> for ExtensionField<F> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: F) -> Self {
        let mut res = self.value;
        res[0] -= rhs;
        Self { value: res }
    }
}

impl<F: Field> SubAssign for ExtensionField<F> {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<F: Field> SubAssign<F> for ExtensionField<F> {
    #[inline]
    fn sub_assign(&mut self, rhs: F) {
        *self = *self - rhs;
    }
}

impl<F: Field> Mul for ExtensionField<F> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self {
        let a = self.value;
        let b = rhs.value;
        Self { value: cubic_mul(&a, &b).to_vec().try_into().unwrap() }
    }
}

impl<F: Field> Mul<F> for ExtensionField<F> {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: F) -> Self {
        Self { value: self.value.map(|x| x * rhs) }
    }
}

impl<F: Field> Product for ExtensionField<F> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        let one = Self { value: field_to_array::<F>(F::one()) };
        iter.fold(one, |acc, x| acc * x)
    }
}

impl<F: Field> MulAssign for ExtensionField<F> {
    #[inline]
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<F: Field> MulAssign<F> for ExtensionField<F> {
    fn mul_assign(&mut self, rhs: F) {
        *self = *self * rhs;
    }
}

impl<F: Field> Neg for ExtensionField<F> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self {
        Self { value: self.value.map(F::neg) }
    }
}

impl<F: Field> Div for ExtensionField<F> {
    type Output = Self;

    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        let a = self.value;
        let b_inv = cubic_inv(&rhs.value);
        Self { value: cubic_mul(&a, &b_inv).to_vec().try_into().unwrap() }
    }
}

impl<F: Field> DivAssign for ExtensionField<F> {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

/// Extend a field `F` element `x` to an array of length 3
/// by filling zeros.
pub fn field_to_array<F: Field>(x: F) -> [F; 3] {
    let mut arr = array::from_fn(|_| F::zero());
    arr[0] = x;
    arr
}

#[inline]
fn cubic_square<F: Field>(a: &[F]) -> [F; 3] {
    let c0 = a[0].square() + (a[2] * a[1]).double();
    let c1 = a[2].square() + (a[0] * a[1]).double() + (a[1] * a[2]).double();
    let c2 = a[1].square() + (a[0] * a[2]).double() + a[2].square();

    [c0, c1, c2]
}

#[inline]
fn cubic_mul<F: Field>(a: &[F], b: &[F]) -> [F; 3] {
    let c0 = a[0] * b[0] + a[2] * b[1] + a[1] * b[2];
    let c1 = a[1] * b[0] + a[0] * b[1] + a[2] * b[1] + a[1] * b[2] + a[2] * b[2];
    let c2 = a[2] * b[0] + a[1] * b[1] + a[0] * b[2] + a[2] * b[2];

    [c0, c1, c2]
}

fn cubic_inv<F: Field>(a: &[F]) -> [F; 3] {
    let aa = a[0].square();
    let ac = a[0] * a[2];
    let ba = a[1] * a[0];
    let bb = a[1].square();
    let bc = a[1] * a[2];
    let cc = a[2].square();

    let aaa = aa * a[0];
    let aac = aa * a[2];
    let abc = ba * a[2];
    let abb = ba * a[1];
    let acc = ac * a[2];
    let bbb = bb * a[1];
    let bcc = bc * a[2];
    let ccc = cc * a[2];

    let t = abc + abc + abc + abb - aaa - aac - aac - acc - bbb + bcc - ccc;

    let i0 = (bc + bb - aa - ac - ac - cc) * t.inverse();
    let i1 = (ba - cc) * t.inverse();
    let i2 = (ac + cc - bb) * t.inverse();

    [i0, i1, i2]
}
