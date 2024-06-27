use serde::Deserialize;
use std::fs;

use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct ConfigJson {
    name: String,
    pilout: String,
    pub debug: Option<bool>,
    pub only_check: Option<bool>,
    executors: Option<ConfigBaseJson>,
    provers: Option<ConfigBaseJson>,
    meta: Option<ConfigBaseJson>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ConfigBaseJson {
    String(String),
    Struct(Value),
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct ProofManConfig {
    pub name: String,
    pub pilout: String,
    pub debug: bool,
    pub only_check: bool,
    pub executors: Option<Value>,
    pub provers: Option<Value>,
    pub meta: Option<Value>,
}

impl ProofManConfig {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_pilout(&self) -> &str {
        &self.pilout
    }

    pub fn parse_input_json(input_json: &str) -> ProofManConfig {
        let parsed_json: ConfigJson = serde_json::from_str(input_json).expect("Failed to parse JSON");

        let executors_config = match parsed_json.executors {
            Some(executors) => match executors {
                ConfigBaseJson::String(filename) => {
                    println!("Reading executors from file: {}", filename);
                    let file_contents = fs::read_to_string(&filename).expect("Failed to read file");
                    Some(serde_json::from_str(file_contents.as_str()).expect("Failed to parse JSON"))
                }
                ConfigBaseJson::Struct(executors) => Some(executors),
            },
            None => None,
        };

        let prover_config = match parsed_json.provers {
            Some(prover) => match prover {
                ConfigBaseJson::String(filename) => {
                    let file_contents = fs::read_to_string(&filename).expect("Failed to read file");
                    Some(serde_json::from_str(file_contents.as_str()).expect("Failed to parse JSON"))
                }
                ConfigBaseJson::Struct(prover) => Some(prover),
            },
            None => None,
        };

        let meta_config = match parsed_json.meta {
            Some(meta) => match meta {
                ConfigBaseJson::String(filename) => {
                    let file_contents = fs::read_to_string(&filename).expect("Failed to read file");
                    Some(serde_json::from_str(file_contents.as_str()).expect("Failed to parse JSON"))
                }
                ConfigBaseJson::Struct(meta) => Some(meta),
            },
            None => None,
        };

        ProofManConfig {
            name: parsed_json.name,
            pilout: parsed_json.pilout,
            debug: parsed_json.debug.unwrap_or(false),
            only_check: parsed_json.only_check.unwrap_or(false),
            executors: executors_config,
            provers: prover_config,
            meta: meta_config,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test deserialization and parsing of JSON input
    #[test]
    fn test_parse_input_json() {
        let input_json = r#"
            {
                "name": "TestConfig",
                "pilout": "test_pilout",
                "debug": true,
                "only_check": false,
                "executors": {
                    "mock_key": "mock_value"
                },
                "provers": {
                    "mock_key": "mock_value"
                }
            }
        "#;

        let config = ProofManConfig::parse_input_json(input_json);

        assert_eq!(config.name, "TestConfig");
        assert_eq!(config.pilout, "test_pilout");
        assert!(config.debug);
        assert!(!config.only_check);

        assert!(config.executors.is_some());
        assert!(config.provers.is_some());
    }

    // Test deserialization and parsing of JSON input with missing fields
    #[test]
    fn test_parse_input_json_missing_fields() {
        let input_json = r#"
            {
                "name": "TestConfig",
                "pilout": "test_pilout"
            }
        "#;

        let config = ProofManConfig::parse_input_json(input_json);

        assert_eq!(config.name, "TestConfig");
        assert_eq!(config.pilout, "test_pilout");
        assert!(!config.debug); // Default value for debug
        assert!(!config.only_check); // Default value for only_check

        assert!(config.executors.is_none());
        assert!(config.provers.is_none());
    }
}
