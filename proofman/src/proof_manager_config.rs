use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::fs;
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

// TODO! Config can be removed?????
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

#[derive(Debug, Deserialize)]
pub struct ConfigJson<E: ExecutorsConfiguration, P: ProverConfiguration, M: MetaConfiguration> {
    name: String,
    pilout: String,
    executors: Option<ExecutorsInput<E>>,
    prover: Option<ProverInput<P>>,
    meta: Option<MetaInput<M>>,
    pub debug: Option<bool>,
    pub only_check: Option<bool>,
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
pub struct ProofManConfig<E: ExecutorsConfiguration, P: ProverConfiguration, M: MetaConfiguration> {
    pub name: String,
    pub pilout: String,
    pub executors: Option<E>,
    pub prover: Option<P>,
    pub meta: Option<M>,
    pub debug: bool,
    pub only_check: bool,
}

impl<E, P, M> ProofManConfig<E, P, M>
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

    pub fn parse_input_json(input_json: &str) -> ProofManConfig<E, P, M> {
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
            Some(prover) => match prover {
                ProverInput::String(filename) => {
                    let file_contents = fs::read_to_string(&filename).expect("Failed to read file");
                    Some(serde_json::from_str(file_contents.as_str()).expect("Failed to parse JSON"))
                }
                ProverInput::Struct(prover) => Some(prover),
            },
            None => None,
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

        ProofManConfig {
            name: parsed_json.name,
            pilout: parsed_json.pilout,
            executors: executors_config,
            prover: prover_config,
            meta: meta_config,
            debug: parsed_json.debug.unwrap_or(false),
            only_check: parsed_json.only_check.unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Define some mock configurations for testing
    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct MockExecutorsConfig {
        mock_key: String,
    }

    impl ExecutorsConfiguration for MockExecutorsConfig {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct MockProverConfig {
        mock_key: String,
    }

    impl ProverConfiguration for MockProverConfig {
        fn variant(&self) -> &str {
            "mock"
        }

        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    #[allow(dead_code)]
    #[derive(Debug, Deserialize)]
    struct MockMetaConfig {
        mock_key: String,
    }

    impl MetaConfiguration for MockMetaConfig {
        fn as_any(&self) -> &dyn Any {
            self
        }
    }

    // Test deserialization and parsing of JSON input
    #[test]
    fn test_parse_input_json() {
        let input_json = r#"
            {
                "name": "TestConfig",
                "pilout": "test_pilout",
                "executors": {
                    "mock_key": "mock_value"
                },
                "prover": {
                    "mock_key": "mock_value"
                },
                "meta": {
                    "mock_key": "mock_value"
                },
                "debug": true,
                "only_check": false
            }
        "#;

        let config: ProofManConfig<MockExecutorsConfig, MockProverConfig, MockMetaConfig> =
            ProofManConfig::parse_input_json(input_json);

        assert_eq!(config.name, "TestConfig");
        assert_eq!(config.pilout, "test_pilout");
        assert!(config.debug);
        assert!(!config.only_check);

        assert!(config.executors.is_some());
        assert!(config.prover.is_some());
        assert!(config.meta.is_some());
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

        let config: ProofManConfig<MockExecutorsConfig, MockProverConfig, MockMetaConfig> =
            ProofManConfig::parse_input_json(input_json);

        assert_eq!(config.name, "TestConfig");
        assert_eq!(config.pilout, "test_pilout");
        assert!(!config.debug); // Default value for debug
        assert!(!config.only_check); // Default value for only_check

        assert!(config.executors.is_none());
        assert!(config.prover.is_none());
        assert!(config.meta.is_none());
    }
}
