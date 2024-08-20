use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct StarkProverSettings {
    #[serde(default = "default_string", rename = "currentPath")]
    pub current_path: String,
    #[serde(default = "default_string", rename = "starkInfoFilename")]
    pub stark_info_filename: String,
    #[serde(default = "default_string", rename = "verifierFilename")]
    pub verkey_filename: String,
    #[serde(default = "default_string", rename = "cHelpers")]
    pub chelpers_filename: String,
}

fn default_string() -> String {
    "".to_owned()
}

impl StarkProverSettings {
    //TODO! Remove filename here, it's used while developing
    pub fn from_json(config_json: &str, filename: &str) -> StarkProverSettings {
        let mut config: StarkProverSettings = serde_json::from_str(config_json).expect("Failed to parse JSON");

        // TODO! Remove this line, the path is stored here to be used by the executor
        config.current_path = filename.to_string();

        config
    }
}
