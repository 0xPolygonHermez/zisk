use std::{any::Any, fmt::Debug};

pub trait Config: Any + Send + Sync + Debug {
    fn get_filename(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
}

#[derive(Debug)]
pub struct ConfigNull {}

impl Config for ConfigNull {
    fn get_filename(&self) -> &str {
        ""
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
