use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AiroutSetings {
    path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WitnessCalculatorSettings {
    path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProofManProverSettings {
    airout: AiroutSetings,
    witness_calculators: Vec<WitnessCalculatorSettings>,
//    prover: ProverSettings,
}

impl ProofManProverSettings {
    pub fn new(json: String) -> Self {
        let data = serde_json::from_str(&json);

        match data {
            Ok(data) => data,
            Err(e) => panic!("Error parsing settings file: {}", e),
        }
    }
}