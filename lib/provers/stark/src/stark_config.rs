use serde::Deserialize;
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct StarkConfig {
    pub variant: String,
    pub settings: HashMap<String, StarkSettings>,
    pub verifier: Option<StarkVerifier>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct StarkSettings {
    pub stark_info: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct StarkVerifier {
    settings: HashMap<String, VerifierSettings>,
}

#[derive(Debug, Deserialize)]
pub struct VerifierSettings {}
