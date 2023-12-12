use std::fmt;

use math::FieldElement;

pub trait PublicInput<T: FieldElement>: fmt::Debug + Send + Sync {
    fn to_elements(&self) -> Vec<T>;
}