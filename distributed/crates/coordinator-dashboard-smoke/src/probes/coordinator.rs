//! Coordinator API probe for live-job and worker-roster fields.

use anyhow::Result;
use reqwest::blocking::Client;
use reqwest::header::HeaderMap;
use serde_json::Value;

use crate::http::request_value;

const REQUIRED_CURRENT_JOB_FIELDS: &[&str] = &[
    "status",
    "phase",
    "phase_code",
    "coordinator_id",
    "job_id",
    "state",
    "age_seconds",
    "phase_age_seconds",
    "update_age_seconds",
    "workers_count",
];

const REQUIRED_WORKER_FIELDS: &[&str] = &[
    "worker_id",
    "status",
    "program",
    "job_id",
    "job_label",
    "phase",
    "assigned_seconds",
    "updated_at",
];

pub fn run(
    client: &Client,
    coordinator_api_url: &str,
    auth_headers: &HeaderMap,
) -> Result<Vec<String>> {
    let mut errors = Vec::new();

    let current_url = format!("{coordinator_api_url}/api/v1/jobs/current");
    let current_raw: Value = request_value(client, &current_url, auth_headers)?;
    match current_raw.get("data").and_then(Value::as_array) {
        Some(rows) if rows.len() == 1 => {
            let row = &rows[0];
            let missing = missing_fields(row, REQUIRED_CURRENT_JOB_FIELDS);
            if !missing.is_empty() {
                errors.push(format!(
                    "coordinator current-job API is missing dashboard fields {missing:?}; \
                     restart or rebuild the coordinator from this branch",
                ));
            }
            let phase_ok =
                row.get("phase").and_then(Value::as_str).is_some_and(|phase| !phase.is_empty());
            if !phase_ok {
                errors.push(
                    "coordinator current-job API row must include a non-empty phase".to_owned(),
                );
            }
            if !row.get("phase_code").is_some_and(Value::is_number) {
                errors.push(
                    "coordinator current-job API row must include numeric phase_code".to_owned(),
                );
            }
            let phase = row.get("phase").and_then(Value::as_str).unwrap_or("-");
            println!("coordinator-api: current_phase={phase}");
        }
        _ => {
            errors.push("coordinator current-job API must return exactly one data row".to_owned());
        }
    }

    let workers_url = format!("{coordinator_api_url}/api/v1/workers");
    let workers_raw: Value = request_value(client, &workers_url, auth_headers)?;
    let worker_rows = match workers_raw.get("data").and_then(Value::as_array) {
        Some(rows) => rows,
        None => {
            errors.push("coordinator workers API did not return a data array".to_owned());
            return Ok(errors);
        }
    };
    for row in worker_rows {
        let missing = missing_fields(row, REQUIRED_WORKER_FIELDS);
        if !missing.is_empty() {
            errors.push(format!("coordinator workers API is missing dashboard fields {missing:?}"));
        }
    }
    println!("coordinator-api: workers={}", worker_rows.len());

    Ok(errors)
}

fn missing_fields(row: &Value, required: &[&str]) -> Vec<String> {
    let obj = match row.as_object() {
        Some(obj) => obj,
        None => return required.iter().map(|s| (*s).to_owned()).collect(),
    };
    let mut missing: Vec<String> = required
        .iter()
        .filter(|field| !obj.contains_key(**field))
        .map(|field| (*field).to_owned())
        .collect();
    missing.sort();
    missing
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_fields_returns_sorted_diff() {
        let row = json!({"job_id": "x", "state": "Completed"});
        let missing = missing_fields(&row, &["state", "job_id", "job_label", "coordinator_id"]);
        assert_eq!(missing, vec!["coordinator_id".to_owned(), "job_label".to_owned()]);
    }

    #[test]
    fn missing_fields_treats_non_object_as_all_missing() {
        let row = json!("not-an-object");
        let missing = missing_fields(&row, &["a", "b"]);
        assert_eq!(missing, vec!["a".to_owned(), "b".to_owned()]);
    }

    #[test]
    fn current_job_required_fields_include_dashboard_live_columns() {
        assert!(REQUIRED_CURRENT_JOB_FIELDS.contains(&"phase_code"));
        assert!(REQUIRED_CURRENT_JOB_FIELDS.contains(&"workers_count"));
    }
}
