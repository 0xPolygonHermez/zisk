use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use std::fs;

use crate::ProofType;

#[derive(Clone, Deserialize)]
pub struct ProofValueMap {
    pub name: String,
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub stage: u64,
}
#[derive(Clone, Deserialize)]
pub struct PublicMap {
    pub name: String,
    #[serde(default)]
    pub stage: u64,
    #[serde(default)]
    pub lengths: Vec<u64>,
}

#[derive(Clone, Deserialize)]
pub struct GlobalInfo {
    pub folder_path: String,
    pub name: String,
    pub airs: Vec<Vec<GlobalInfoAir>>,
    pub air_groups: Vec<String>,

    #[serde(rename = "aggTypes")]
    pub agg_types: Vec<Vec<GlobalInfoAggType>>,

    #[serde(rename = "stepsFRI")]
    pub steps_fri: Vec<GlobalInfoStepsFRI>,

    #[serde(rename = "nPublics")]
    pub n_publics: usize,
    #[serde(rename = "numChallenges")]
    pub n_challenges: Vec<usize>,

    #[serde(rename = "proofValuesMap")]
    pub proof_values_map: Option<Vec<ProofValueMap>>,

    #[serde(rename = "publicsMap")]
    pub publics_map: Option<Vec<PublicMap>>,
}

#[derive(Clone, Deserialize)]
pub struct GlobalInfoAir {
    pub name: String,

    #[serde(rename = "hasCompressor", default)]
    pub has_compressor: Option<bool>,

    pub num_rows: usize,
}

#[derive(Clone, Deserialize)]
pub struct GlobalInfoAggType {
    #[serde(rename = "aggType")]
    pub agg_type: usize,
}

#[derive(Clone, Deserialize)]
pub struct GlobalInfoStepsFRI {
    #[serde(rename = "nBits")]
    pub n_bits: usize,
}

impl GlobalInfo {
    pub fn new(proving_key_path: &Path) -> Self {
        log::debug!("glblinfo: ··· Loading GlobalInfo JSON {}", proving_key_path.display());

        let global_info = Self::from_file(&proving_key_path.display().to_string());

        global_info
    }

    pub fn from_file(folder_path: &String) -> Self {
        let file_path = folder_path.to_string() + "/pilout.globalInfo.json";
        let global_info_json =
            fs::read_to_string(&file_path).unwrap_or_else(|_| panic!("Failed to read file {}", file_path));

        let mut global_info_value: Value = serde_json::from_str(&global_info_json)
            .unwrap_or_else(|err| panic!("Failed to parse JSON file: {}: {}", file_path, err));

        // Add the folder_path to the JSON object
        if let Some(obj) = global_info_value.as_object_mut() {
            obj.insert("folder_path".to_string(), Value::String(folder_path.to_string()));
        } else {
            panic!("JSON is not an object: {}", file_path);
        }

        // Serialize the updated JSON object back to a string
        let updated_global_info_json = serde_json::to_string(&global_info_value)
            .unwrap_or_else(|err| panic!("Failed to serialize updated JSON: {}", err));

        // Deserialize the updated JSON string into the `GlobalInfo` struct
        let global_info: GlobalInfo = serde_json::from_str(&updated_global_info_json)
            .unwrap_or_else(|err| panic!("Failed to parse updated JSON file: {}: {}", file_path, err));
        global_info
    }

    pub fn get_proving_key_path(&self) -> PathBuf {
        PathBuf::from(self.folder_path.to_string())
    }

    pub fn get_setup_path(&self, template: &str) -> PathBuf {
        let vadcop_final_setup_folder = format!("{}/{}/{}/{}", self.folder_path, self.name, template, template);
        PathBuf::from(vadcop_final_setup_folder)
    }

    pub fn get_air_setup_path(&self, airgroup_id: usize, air_id: usize, proof_type: &ProofType) -> PathBuf {
        let type_str = match proof_type {
            ProofType::Basic => "air",
            ProofType::Compressor => "compressor",
            ProofType::Recursive1 => "recursive1",
            ProofType::Recursive2 => "recursive2",
            _ => panic!(),
        };

        let air_setup_folder = match proof_type {
            ProofType::Recursive2 => {
                format!("{}/{}/{}/recursive2/recursive2", self.folder_path, self.name, self.air_groups[airgroup_id])
            }
            ProofType::Compressor | ProofType::Recursive1 => {
                format!(
                    "{}/{}/{}/airs/{}/{}/{}",
                    self.folder_path,
                    self.name,
                    self.air_groups[airgroup_id],
                    self.airs[airgroup_id][air_id].name,
                    type_str,
                    type_str,
                )
            }
            ProofType::Basic => {
                format!(
                    "{}/{}/{}/airs/{}/{}/{}",
                    self.folder_path,
                    self.name,
                    self.air_groups[airgroup_id],
                    self.airs[airgroup_id][air_id].name,
                    type_str,
                    self.get_air_name(airgroup_id, air_id),
                )
            }
            _ => panic!(),
        };

        PathBuf::from(air_setup_folder)
    }

    pub fn get_air_group_name(&self, airgroup_id: usize) -> &str {
        &self.air_groups[airgroup_id]
    }

    pub fn get_airgroup_id(&self, air_group_name: &str) -> usize {
        self.air_groups
            .iter()
            .position(|name| name == air_group_name)
            .unwrap_or_else(|| panic!("Air group '{}' not found", air_group_name))
    }

    pub fn get_air_name(&self, airgroup_id: usize, air_id: usize) -> &str {
        &self.airs[airgroup_id][air_id].name
    }

    pub fn get_air_has_compressor(&self, airgroup_id: usize, air_id: usize) -> bool {
        self.airs[airgroup_id][air_id].has_compressor.unwrap_or(false)
    }
}
