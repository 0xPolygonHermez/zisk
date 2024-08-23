use std::path::PathBuf;

// use serde_json::Value as JsonValue;
use proofman_util::{timer_start, timer_stop_and_log};
use log::debug;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct GlobalInfo {
    pub name: String,
    pub airs: Vec<Vec<GlobalInfoAir>>,
    pub subproofs: Vec<String>,

    #[serde(rename = "aggTypes")]
    pub agg_types: Vec<Vec<GlobalInfoAggType>>,

    #[serde(rename = "stepsFRI")]
    pub steps_fri: Vec<GlobalInfoStepsFRI>,

    #[serde(rename = "nPublics")]
    pub n_publics: usize,
    #[serde(rename = "numChallenges")]
    pub n_challenges: Vec<usize>,
}

#[derive(Deserialize)]
pub struct GlobalInfoAir {
    pub name: String,
    //#[serde(rename = "hasCompressor")]
    //pub has_compressor: bool,
}

#[derive(Deserialize)]
pub struct GlobalInfoAggType {
    #[serde(rename = "aggType")]
    pub agg_type: usize,
}

#[derive(Deserialize)]
pub struct GlobalInfoStepsFRI {
    #[serde(rename = "nBits")]
    pub n_bits: usize,
}

impl GlobalInfo {
    pub fn from_file(global_info_path: &PathBuf) -> Self {
        let global_info_json = std::fs::read_to_string(global_info_path)
            .unwrap_or_else(|_| panic!("Failed to read file {}", global_info_path.display()));

        GlobalInfo::from_json(&global_info_json)
    }

    pub fn from_json(global_info_json: &str) -> Self {
        timer_start!(GLOBAL_INFO_LOAD);

        debug!("glblinfo: ··· Loading GlobalInfo JSON");
        let global_info: GlobalInfo = serde_json::from_str(global_info_json).expect("Failed to parse JSON file");

        timer_stop_and_log!(GLOBAL_INFO_LOAD);

        global_info
    }

    pub fn get_air_setup_path(&self, air_group_id: usize, air_id: usize) -> PathBuf {
        let air_setup_folder =
            format!("build/{}/airs/{}/air", self.subproofs[air_group_id], self.airs[air_group_id][air_id].name);

        PathBuf::from(air_setup_folder)
    }

    pub fn get_air_group_name(&self, air_group_id: usize) -> &str {
        &self.subproofs[air_group_id]
    }

    pub fn get_air_name(&self, air_group_id: usize, air_id: usize) -> &str {
        &self.airs[air_group_id][air_id].name
    }
}
