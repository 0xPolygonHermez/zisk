use std::fmt;

pub trait PublicInputs<T>: fmt::Debug + Send + Sync {
    fn to_elements(&self) -> Vec<T>;
}
