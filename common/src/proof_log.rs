use serde::Serialize;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct ProofLog {
    cycles: u64,
    id: String,
    time: f64,
}

impl ProofLog {
    pub fn new(cycles: u64, id: String, time: f64) -> Self {
        ProofLog { cycles, id, time }
    }

    pub fn write_json_log(file_path: &PathBuf, entries: &ProofLog) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_string_pretty(entries)?;
        let mut file = File::create(file_path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
}
