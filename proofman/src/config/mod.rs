pub mod proofman_config;


use std::any::Any;

pub trait ExecutorsConfiguration: Any {
    fn as_any(&self) -> &dyn Any;
}

pub trait MetaConfiguration: Any {
    fn as_any(&self) -> &dyn Any;
}

pub trait ProverConfiguration: Any {
    fn variant(&self) -> &str;
    fn as_any(&self) -> &dyn Any;
}

// TODO! This can be removed?????
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
