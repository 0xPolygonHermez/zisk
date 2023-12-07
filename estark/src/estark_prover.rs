use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct ESTARKProverSettings {
    #[serde(rename = "nBits")]
    n_bits: u32,
    #[serde(rename = "nBitsExt")]
    n_bits_ext: u32,
    #[serde(rename = "nQueries")]
    n_queries: u32,
    #[serde(rename = "verificationHashType")]
    verification_hash_type: String,
    steps: Vec<NBits>
}

#[derive(Serialize, Deserialize, Debug)]
struct NBits {
    #[serde(rename = "nBits")]
    n_bits: u32,
}

impl ESTARKProverSettings {
    pub fn new(json: String) -> Self {
        let data = serde_json::from_str(&json);

        match data {
            Ok(data) => data,
            Err(e) => panic!("Error parsing settings file: {}", e),
        }
    }
}