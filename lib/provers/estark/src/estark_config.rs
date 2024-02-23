use serde::Deserialize;
use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct EStarkConfig {
    pub variant: String,
    pub settings: HashMap<String, EStarkSettings>,
    pub verifier: Option<EStarkVerifier>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct EStarkSettings {
    pub stark_info: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct EStarkVerifier {
    settings: HashMap<String, VerifierSettings>,
}

#[derive(Debug, Deserialize)]
pub struct VerifierSettings {}
