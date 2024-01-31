use std::any::Any;

pub trait Config: Any + Send + Sync {
    fn get_filename(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
}

pub struct ConfigNull {}

impl Config for ConfigNull {
    fn get_filename(&self) -> &str {
        ""
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
