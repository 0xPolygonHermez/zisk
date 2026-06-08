//! Poll coordinator API, worker roster, and metrics.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::Client;
use reqwest::header::HeaderMap;
use serde::Deserialize;
use serde_json::Value;

use crate::scenarios::PhaseCode;

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub elapsed_seconds: f64,
    pub current: CurrentJobRow,
    pub workers: Vec<WorkerRow>,
    pub metrics: MetricSamples,
}

/// Dashboard-facing fields from `/api/v1/jobs/current`.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CurrentJobRow {
    pub status: String,
    pub phase: String,
    pub phase_code: u8,
    #[serde(default)]
    pub coordinator_id: Option<String>,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub age_seconds: Option<u64>,
    #[serde(default)]
    pub phase_age_seconds: Option<u64>,
    #[serde(default)]
    pub update_age_seconds: Option<u64>,
    #[serde(default)]
    pub workers_count: Option<usize>,
}

impl CurrentJobRow {
    pub fn phase_code_enum(&self) -> PhaseCode {
        PhaseCode::from_u8(self.phase_code)
    }
}

/// Dashboard-facing fields from `/api/v1/workers`.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct WorkerRow {
    pub worker_id: String,
    pub status: String,
    #[serde(default)]
    pub program: Option<String>,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub phase: Option<String>,
    pub heartbeat_age_seconds: u64,
}

#[derive(Debug, Default, Clone)]
pub struct MetricSamples {
    pub samples: Vec<Sample>,
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub name: String,
    pub labels: HashMap<String, String>,
    pub value: f64,
}

impl MetricSamples {
    pub fn find(&self, name: &str, labels: &[(&str, &str)]) -> Option<&Sample> {
        let mut hits = self.iter(name, labels);
        let first = hits.next()?;
        if hits.next().is_some() {
            return None;
        }
        Some(first)
    }

    pub fn sum(&self, name: &str, labels: &[(&str, &str)]) -> f64 {
        self.iter(name, labels).map(|sample| sample.value).sum()
    }

    pub fn iter<'s>(
        &'s self,
        name: &str,
        labels: &[(&str, &str)],
    ) -> impl Iterator<Item = &'s Sample> + 's {
        let name = name.to_owned();
        let labels: Vec<(String, String)> =
            labels.iter().map(|(key, value)| ((*key).to_owned(), (*value).to_owned())).collect();
        self.samples.iter().filter(move |sample| {
            sample.name == name
                && labels.iter().all(|(key, expected)| {
                    sample.labels.get(key.as_str()).map(String::as_str) == Some(expected.as_str())
                })
        })
    }

    pub fn value_or_zero(&self, name: &str, labels: &[(&str, &str)]) -> f64 {
        self.find(name, labels).map(|sample| sample.value).unwrap_or(0.0)
    }
}

pub struct Recorder<'a> {
    client: &'a Client,
    headers: &'a HeaderMap,
    coordinator_api: &'a str,
    started: Instant,
}

impl<'a> Recorder<'a> {
    pub fn new(client: &'a Client, headers: &'a HeaderMap, coordinator_api: &'a str) -> Self {
        Self { client, headers, coordinator_api, started: Instant::now() }
    }

    pub fn snapshot(&self) -> Result<Snapshot> {
        let current = self.fetch_current()?;
        let workers = self.fetch_workers()?;
        let metrics = self.fetch_metrics()?;
        Ok(Snapshot {
            elapsed_seconds: self.started.elapsed().as_secs_f64(),
            current,
            workers,
            metrics,
        })
    }

    pub fn poll_until(
        &self,
        interval: Duration,
        timeout: Duration,
        mut predicate: impl FnMut(&Snapshot) -> bool,
    ) -> Result<Snapshot> {
        let deadline = Instant::now() + timeout;
        loop {
            let snapshot = self.snapshot()?;
            if predicate(&snapshot) {
                return Ok(snapshot);
            }
            if Instant::now() >= deadline {
                return Err(anyhow!(
                    "timed out after {:?} waiting for predicate to hold (last phase={}, status={})",
                    timeout,
                    snapshot.current.phase,
                    snapshot.current.status,
                ));
            }
            std::thread::sleep(interval);
        }
    }

    fn fetch_current(&self) -> Result<CurrentJobRow> {
        let url = format!("{}/api/v1/jobs/current", self.coordinator_api);
        let raw: Value = request_value(self.client, &url, self.headers)?;
        let rows = raw
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("/api/v1/jobs/current did not return a `data` array"))?;
        let row = rows
            .first()
            .ok_or_else(|| anyhow!("/api/v1/jobs/current returned an empty `data` array"))?;
        serde_json::from_value(row.clone()).context("decode current-job row")
    }

    fn fetch_workers(&self) -> Result<Vec<WorkerRow>> {
        let url = format!("{}/api/v1/workers", self.coordinator_api);
        let raw: Value = request_value(self.client, &url, self.headers)?;
        let rows = raw
            .get("data")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("/api/v1/workers did not return a `data` array"))?;
        rows.iter()
            .map(|row| serde_json::from_value::<WorkerRow>(row.clone()).map_err(Into::into))
            .collect()
    }

    fn fetch_metrics(&self) -> Result<MetricSamples> {
        let url = format!("{}/metrics", self.coordinator_api);
        let body = self
            .client
            .get(&url)
            .headers(self.headers.clone())
            .send()
            .map_err(|error| anyhow!("{url} failed: {error}"))?
            .error_for_status()
            .map_err(|error| anyhow!("{url} returned non-success: {error}"))?
            .text()
            .map_err(|error| anyhow!("{url} body read failed: {error}"))?;
        Ok(parse_prom_text(&body))
    }
}

pub fn parse_prom_text(text: &str) -> MetricSamples {
    let mut samples = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some(sample) = parse_prom_line(line) else { continue };
        samples.push(sample);
    }
    MetricSamples { samples }
}

fn parse_prom_line(line: &str) -> Option<Sample> {
    let (head, tail) = if let Some(brace_open) = line.find('{') {
        let brace_close = line[brace_open..].find('}')?;
        let labels_str = &line[brace_open + 1..brace_open + brace_close];
        let name = line[..brace_open].trim();
        let after_braces = line[brace_open + brace_close + 1..].trim();
        (Header { name: name.to_owned(), labels: parse_labels(labels_str) }, after_braces)
    } else {
        let mut parts = line.splitn(2, char::is_whitespace);
        let name = parts.next()?.trim().to_owned();
        let rest = parts.next()?.trim();
        (Header { name, labels: HashMap::new() }, rest)
    };
    let value_token = tail.split_whitespace().next()?;
    let value: f64 = value_token.parse().ok()?;
    Some(Sample { name: head.name, labels: head.labels, value })
}

struct Header {
    name: String,
    labels: HashMap<String, String>,
}

fn parse_labels(raw: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    // Handles Prometheus label escapes for quoted values.
    let bytes = raw.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() {
        while idx < bytes.len() && (bytes[idx] == b',' || bytes[idx].is_ascii_whitespace()) {
            idx += 1;
        }
        let key_start = idx;
        while idx < bytes.len() && bytes[idx] != b'=' {
            idx += 1;
        }
        if idx >= bytes.len() {
            break;
        }
        let key = std::str::from_utf8(&bytes[key_start..idx]).unwrap_or("").trim().to_owned();
        idx += 1; // skip '='
        if idx >= bytes.len() || bytes[idx] != b'"' {
            continue;
        }
        idx += 1; // skip opening quote
        let mut value = String::new();
        while idx < bytes.len() {
            match bytes[idx] {
                b'\\' if idx + 1 < bytes.len() => {
                    match bytes[idx + 1] {
                        b'"' => value.push('"'),
                        b'\\' => value.push('\\'),
                        b'n' => value.push('\n'),
                        other => value.push(other as char),
                    }
                    idx += 2;
                }
                b'"' => {
                    idx += 1;
                    break;
                }
                other => {
                    value.push(other as char);
                    idx += 1;
                }
            }
        }
        if !key.is_empty() {
            out.insert(key, value);
        }
    }
    out
}

fn request_value(client: &Client, url: &str, headers: &HeaderMap) -> Result<Value> {
    let response = client
        .get(url)
        .headers(headers.clone())
        .send()
        .map_err(|error| anyhow!("{url} failed: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().unwrap_or_else(|e| format!("<body read failed: {e}>"));
        return Err(anyhow!("{url} returned HTTP {}: {body}", status.as_u16()));
    }
    response.json::<Value>().map_err(|error| anyhow!("{url} returned non-JSON body: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_counter_with_labels() {
        let text = r#"
# HELP coordinator_jobs_total Jobs ...
# TYPE coordinator_jobs_total counter
coordinator_jobs_total{coordinator_id="coord-a",kind="prove",outcome="success",program="fib"} 3
"#;
        let parsed = parse_prom_text(text);
        let sample = parsed
            .find(
                "coordinator_jobs_total",
                &[("kind", "prove"), ("outcome", "success"), ("program", "fib")],
            )
            .expect("series present");
        assert!((sample.value - 3.0).abs() < f64::EPSILON);
        assert_eq!(sample.labels.get("coordinator_id").map(String::as_str), Some("coord-a"));
    }

    #[test]
    fn parses_labelless_gauge() {
        let text = "coordinator_db_write_queue_depth 0\n";
        let parsed = parse_prom_text(text);
        let sample = parsed.find("coordinator_db_write_queue_depth", &[]).expect("series");
        assert!((sample.value - 0.0).abs() < f64::EPSILON);
        assert!(sample.labels.is_empty());
    }

    #[test]
    fn missing_metric_returns_zero_when_treated_as_counter() {
        let parsed = parse_prom_text("# nothing here\n");
        assert!((parsed.value_or_zero("nope", &[]) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn sum_aggregates_across_label_combos() {
        let text = r#"
coordinator_workers_by_status{status="running"} 2
coordinator_workers_by_status{status="idle"} 1
coordinator_workers_by_status{status="errored"} 0
"#;
        let parsed = parse_prom_text(text);
        assert!((parsed.sum("coordinator_workers_by_status", &[]) - 3.0).abs() < f64::EPSILON);
        assert!(
            (parsed.sum("coordinator_workers_by_status", &[("status", "running")]) - 2.0).abs()
                < f64::EPSILON,
        );
    }

    #[test]
    fn label_value_with_quotes_is_preserved() {
        let text = r#"coordinator_info{version="v0.18-\"rc1\"",environment="dev"} 1"#;
        let parsed = parse_prom_text(text);
        let sample =
            parsed.find("coordinator_info", &[("environment", "dev")]).expect("series present");
        assert_eq!(sample.labels.get("version").map(String::as_str), Some("v0.18-\"rc1\""));
    }
}
