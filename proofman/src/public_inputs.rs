use std::fmt;

pub trait PublicInputs<T>: fmt::Debug + Send + Sync {
    fn to_vec(&self) -> Vec<T>;
}
