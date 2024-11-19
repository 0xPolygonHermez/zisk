use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

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
