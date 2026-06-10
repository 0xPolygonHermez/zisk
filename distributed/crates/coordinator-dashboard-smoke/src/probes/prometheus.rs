//! Prometheus target and core-metric smoke probe.

use anyhow::Result;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::http::request_json;

pub fn run(client: &Client, prometheus_url: &str, coordinator_id: &str) -> Result<Vec<String>> {
    let mut errors = Vec::new();
    let coord_filter = format!("{{coordinator_id=~\"{coordinator_id}\"}}");

    let targets_url = format!("{prometheus_url}/api/v1/targets?state=active");
    let targets: TargetsResponse = request_json(client, &targets_url, &Default::default())?;
    for target in targets.data.active_targets {
        let job = target.labels.get("job").and_then(Value::as_str).unwrap_or("");
        if job.starts_with("zisk-workers") {
            errors.push(
                "Prometheus still has a v0.17-style worker scrape target. v0.18 exposes \
                 coordinator-level pool metrics only; remove zisk-workers scrape_configs."
                    .to_owned(),
            );
        }
        if job.starts_with("zisk-coordinator") && target.health.as_deref() != Some("up") {
            let scrape_url = target.scrape_url.as_deref().unwrap_or("-");
            let health = target.health.as_deref().unwrap_or("unknown");
            let last_error = target.last_error.as_deref().unwrap_or("");
            errors.push(format!(
                "Prometheus coordinator target {scrape_url} is {health}: {last_error}",
            ));
        }
    }

    let coordinator_info =
        prometheus_query(client, prometheus_url, &format!("coordinator_info{coord_filter}"))?;
    if coordinator_info.is_empty() {
        errors.push(
            "Prometheus has no coordinator_info series for the selected coordinator".to_owned(),
        );
    }

    let last_success = prometheus_query(
        client,
        prometheus_url,
        &format!("coordinator_last_successful_job_timestamp_seconds{coord_filter}"),
    )?;
    if last_success.is_empty() {
        errors.push(
            "Prometheus has no coordinator_last_successful_job_timestamp_seconds series; \
             restart the coordinator onto the latest v0.18 observability binary"
                .to_owned(),
        );
    }

    let connected = prometheus_query(
        client,
        prometheus_url,
        &format!("sum(coordinator_workers_connected{coord_filter}) or vector(0)"),
    )?;
    let active = prometheus_query(
        client,
        prometheus_url,
        &format!("sum(coordinator_active_jobs{coord_filter}) or vector(0)"),
    )?;

    println!(
        "prometheus: coordinator_info={count} workers={workers} active_jobs={active}",
        count = coordinator_info.len(),
        workers = scalar_value(&connected).unwrap_or_else(|| "?".to_owned()),
        active = scalar_value(&active).unwrap_or_else(|| "?".to_owned()),
    );

    Ok(errors)
}

fn prometheus_query(client: &Client, prometheus_url: &str, query: &str) -> Result<Vec<PromSeries>> {
    let encoded = urlencoding::encode(query);
    let url = format!("{prometheus_url}/api/v1/query?query={encoded}");
    let payload: PromResponse = request_json(client, &url, &Default::default())?;
    if payload.status != "success" {
        return Err(anyhow::anyhow!(
            "Prometheus query failed: {query}: {raw}",
            raw = serde_json::to_string(&payload).unwrap_or_default()
        ));
    }
    Ok(payload.data.result)
}

fn scalar_value(series: &[PromSeries]) -> Option<String> {
    let first = series.first()?;
    let value = first.value.as_ref()?;
    value.get(1)?.as_str().map(|s| s.to_owned())
}

#[derive(Debug, Deserialize)]
struct TargetsResponse {
    #[serde(default)]
    data: TargetsData,
}

#[derive(Debug, Default, Deserialize)]
struct TargetsData {
    #[serde(rename = "activeTargets", default)]
    active_targets: Vec<Target>,
}

#[derive(Debug, Deserialize)]
struct Target {
    #[serde(default)]
    labels: serde_json::Map<String, Value>,
    #[serde(default)]
    health: Option<String>,
    #[serde(default, rename = "scrapeUrl")]
    scrape_url: Option<String>,
    #[serde(default, rename = "lastError")]
    last_error: Option<String>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct PromResponse {
    status: String,
    #[serde(default)]
    data: PromData,
}

#[derive(Debug, Default, Deserialize, serde::Serialize)]
struct PromData {
    #[serde(default)]
    result: Vec<PromSeries>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct PromSeries {
    /// Metric labels; values are not inspected by this smoke probe.
    #[serde(default)]
    metric: serde_json::Map<String, Value>,
    /// Instant-vector value: `[<timestamp:f64>, "<sample:string>"]`.
    #[serde(default)]
    value: Option<Vec<Value>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scalar_value_extracts_string_sample() {
        let series = vec![PromSeries {
            metric: Default::default(),
            value: Some(vec![Value::from(1700000000.0), Value::from("3")]),
        }];
        assert_eq!(scalar_value(&series).as_deref(), Some("3"));
    }

    #[test]
    fn scalar_value_returns_none_when_empty() {
        assert!(scalar_value(&[]).is_none());
    }

    #[test]
    fn targets_response_parses_small_payload() {
        // Mirrors a compact Prometheus reply with optional fields omitted.
        let raw = serde_json::json!({
            "data": {
                "activeTargets": [
                    {
                        "labels": {"job": "zisk-coordinator"},
                        "health": "up",
                        "scrapeUrl": "http://coord:9090/metrics"
                    },
                    {
                        "labels": {"job": "zisk-workers"},
                        "health": "down",
                        "scrapeUrl": "http://w:9100/metrics",
                        "lastError": "refused"
                    }
                ]
            }
        });
        let parsed: TargetsResponse = serde_json::from_value(raw).unwrap();
        assert_eq!(parsed.data.active_targets.len(), 2);
        assert_eq!(
            parsed.data.active_targets[0].labels.get("job").unwrap().as_str(),
            Some("zisk-coordinator")
        );
        assert_eq!(parsed.data.active_targets[1].last_error.as_deref(), Some("refused"));
    }
}
