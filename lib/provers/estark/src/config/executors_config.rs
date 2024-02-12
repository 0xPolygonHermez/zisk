use serde::Deserialize;
use std::any::Any;
use std::collections::HashMap;
use proofman::config::ExecutorsConfiguration;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct ExecutorsConfig {
    settings: HashMap<String, ExecutorsSettings>,
}

#[derive(Debug, Deserialize)]
pub struct ExecutorsSettings {}

impl ExecutorsConfiguration for ExecutorsConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
