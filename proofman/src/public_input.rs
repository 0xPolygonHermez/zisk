use std::fmt;

pub trait PublicInput<T>: fmt::Debug + Send + Sync {
    fn to_elements(&self) -> Vec<T>;
}