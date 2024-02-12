use serde::Deserialize;
use std::any::Any;
use proofman::config::MetaConfiguration;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct MetaConfig {
    hello: String,
}

impl MetaConfiguration for MetaConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }
}
