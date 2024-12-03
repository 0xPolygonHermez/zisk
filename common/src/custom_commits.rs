use proofman_starks_lib_c::{extend_and_merkelize_custom_commit_c, fri_proof_new_c, starks_new_c};
use p3_goldilocks::Goldilocks;
use serde_json::Value;
use std::collections::HashMap;
use std::os::raw::c_void;
use std::path::PathBuf;

use crate::Setup;

pub fn parse_cached_buffers(s: &str) -> Result<HashMap<String, PathBuf>, String> {
    let json_data: Value = serde_json::from_str(s).map_err(|e| format!("Invalid JSON: {}", e))?;
    let mut map = HashMap::new();

    if let Some(obj) = json_data.as_object() {
        for (key, value) in obj {
            if let Some(path_str) = value.as_str() {
                map.insert(key.clone(), PathBuf::from(path_str));
            } else {
                return Err(format!("Expected string for path in key '{}'", key));
            }
        }
    } else {
        return Err("Expected JSON object for cached_buffers".to_string());
    }

    Ok(map)
}

pub fn get_custom_commit_trace(commit_id: u64, step: u64, setup: &Setup, buffer: Vec<Goldilocks>, buffer_str: &str) {
    extend_and_merkelize_custom_commit_c(
        starks_new_c((&setup.p_setup).into(), std::ptr::null_mut()),
        commit_id,
        step,
        buffer.as_ptr() as *mut c_void,
        fri_proof_new_c((&setup.p_setup).into()),
        std::ptr::null_mut(),
        buffer_str,
    );
}
