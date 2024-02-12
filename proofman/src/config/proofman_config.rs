use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::fs;

use crate::config::{ExecutorsConfiguration, ProverConfiguration, MetaConfiguration};

#[derive(Debug, Deserialize)]
pub struct ConfigJson<E: ExecutorsConfiguration, P: ProverConfiguration, M: MetaConfiguration> {
    name: String,
    pilout: String,
    executors: Option<ExecutorsInput<E>>,
    prover: ProverInput<P>,
    meta: Option<MetaInput<M>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ExecutorsInput<E: ExecutorsConfiguration> {
    String(String),
    Struct(E),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ProverInput<P: ProverConfiguration> {
    String(String),
    Struct(P),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MetaInput<M: MetaConfiguration> {
    String(String),
    Struct(M),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct Config<E: ExecutorsConfiguration, P: ProverConfiguration, M: MetaConfiguration> {
    name: String,
    pilout: String,
    executors: Option<E>,
    prover: P,
    meta: Option<M>,
}

impl<E, P, M> Config<E, P, M>
where
    E: ExecutorsConfiguration + DeserializeOwned,
    P: ProverConfiguration + DeserializeOwned,
    M: MetaConfiguration + DeserializeOwned,
{
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_pilout(&self) -> &str {
        &self.pilout
    }

    pub fn parse_input_json(input_json: &str) -> Config<E, P, M> {
        let parsed_json: ConfigJson<E, P, M> = serde_json::from_str(input_json).expect("Failed to parse JSON");

        let executors_config = match parsed_json.executors {
            Some(executors) => match executors {
                ExecutorsInput::String(filename) => {
                    let file_contents = fs::read_to_string(&filename).expect("Failed to read file");
                    Some(serde_json::from_str(file_contents.as_str()).expect("Failed to parse JSON"))
                }
                ExecutorsInput::Struct(executors) => Some(executors),
            },
            None => None,
        };

        let prover_config = match parsed_json.prover {
            ProverInput::String(filename) => {
                let file_contents = fs::read_to_string(&filename).expect("Failed to read file");
                serde_json::from_str(file_contents.as_str()).expect("Failed to parse JSON")
            }
            ProverInput::Struct(prover) => prover,
        };

        let meta_config = match parsed_json.meta {
            Some(meta) => match meta {
                MetaInput::String(filename) => {
                    let file_contents = fs::read_to_string(&filename).expect("Failed to read file");
                    Some(serde_json::from_str(file_contents.as_str()).expect("Failed to parse JSON"))
                }
                MetaInput::Struct(meta) => Some(meta),
            },
            None => None,
        };

        Config {
            name: parsed_json.name,
            pilout: parsed_json.pilout,
            executors: executors_config,
            prover: prover_config,
            meta: meta_config,
        }
    }
}
