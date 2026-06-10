//! Grafana dashboard fetch and `/api/ds/query` helpers.

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::{json, Value};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Query result rolled up across all targets in one panel.
#[derive(Debug, Clone)]
pub struct PanelResult {
    pub rows: usize,
    /// Last successful target HTTP status.
    #[allow(dead_code)]
    pub http_status: u16,
    /// Per-target errors, keyed by target index.
    pub target_errors: Vec<(usize, String)>,
}

impl PanelResult {
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.rows == 0 && self.target_errors.is_empty()
    }

    pub fn is_error(&self) -> bool {
        !self.target_errors.is_empty()
    }

    pub fn is_populated(&self) -> bool {
        self.rows > 0
    }
}

pub fn build_client() -> Result<Client> {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|error| anyhow!("failed to build HTTP client: {error}"))
}

#[derive(Debug, Clone)]
pub struct BasicAuth {
    user: String,
    password: String,
}

impl BasicAuth {
    pub fn new(user: &str, password: &str) -> Self {
        Self { user: user.to_owned(), password: password.to_owned() }
    }

    fn apply(&self, request: RequestBuilder) -> RequestBuilder {
        request.basic_auth(&self.user, Some(&self.password))
    }
}

/// Fetch the dashboard JSON by UID.
pub fn fetch_dashboard(
    client: &Client,
    grafana_url: &str,
    uid: &str,
    auth: &BasicAuth,
) -> Result<Value> {
    let encoded_uid = urlencoding::encode(uid);
    let url = format!("{grafana_url}/api/dashboards/uid/{encoded_uid}");
    let response =
        auth.apply(client.get(&url)).send().with_context(|| format!("GET {url} failed"))?;
    let status = response.status();
    if !status.is_success() {
        let body =
            response.text().unwrap_or_else(|error| format!("<could not read body: {error}>"));
        return Err(anyhow!("{url} returned HTTP {}: {body}", status.as_u16()));
    }
    let payload: Value =
        response.json().with_context(|| format!("{url} returned non-JSON body"))?;
    payload
        .get("dashboard")
        .cloned()
        .ok_or_else(|| anyhow!("{url} response had no `dashboard` object"))
}

/// Execute panel targets through `/api/ds/query`.
pub fn query_panel(
    client: &Client,
    grafana_url: &str,
    auth: &BasicAuth,
    targets: &[Value],
) -> PanelResult {
    if targets.is_empty() {
        return PanelResult { rows: 0, http_status: 0, target_errors: Vec::new() };
    }

    let mut max_rows = 0usize;
    let mut last_status = 0u16;
    let mut errors: Vec<(usize, String)> = Vec::new();

    for (idx, target) in targets.iter().enumerate() {
        match query_one(client, grafana_url, auth, target) {
            Ok((rows, status)) => {
                last_status = status;
                if rows > max_rows {
                    max_rows = rows;
                }
            }
            Err(error) => {
                errors.push((idx, error.to_string()));
            }
        }
    }

    PanelResult { rows: max_rows, http_status: last_status, target_errors: errors }
}

fn query_one(
    client: &Client,
    grafana_url: &str,
    auth: &BasicAuth,
    target: &Value,
) -> Result<(usize, u16)> {
    let body = wrap_target(target)?;
    let url = format!("{grafana_url}/api/ds/query");

    let mut req_headers = HeaderMap::new();
    req_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let response = auth
        .apply(client.post(&url))
        .headers(req_headers)
        .body(serde_json::to_vec(&body)?)
        .send()
        .with_context(|| format!("POST {url} failed"))?;

    let status = response.status();
    let status_code = status.as_u16();
    if !status.is_success() {
        let body =
            response.text().unwrap_or_else(|error| format!("<could not read body: {error}>"));
        return Err(anyhow!("HTTP {status_code}: {body}"));
    }

    let payload: Value =
        response.json().with_context(|| format!("{url} returned non-JSON body"))?;
    let rows = count_rows(&payload);
    Ok((rows, status_code))
}

/// Build one `/api/ds/query` request body and substitute dashboard variables.
fn wrap_target(target: &Value) -> Result<Value> {
    let mut query = substitute_template_vars(target);
    if query.get("refId").and_then(Value::as_str).unwrap_or_default().is_empty() {
        query
            .as_object_mut()
            .ok_or_else(|| anyhow!("target was not a JSON object: {target}"))?
            .insert("refId".to_owned(), Value::String("A".to_owned()));
    }
    Ok(json!({
        "queries": [query],
        "from": "now-2h",
        "to": "now",
    }))
}

fn substitute_template_vars(target: &Value) -> Value {
    fn rewrite(s: &str) -> String {
        // Longer keys first so `$__range` does not rewrite `$__rate_interval`.
        let replacements: &[(&str, &str)] = &[
            ("$__timeFilter(sort_at)", "sort_at >= NOW() - INTERVAL '2 hours'"),
            ("$__rate_interval", "30s"),
            ("$__range", "1h"),
            ("${coordinator:singlequote}", "''"),
            ("${program:singlequote}", "''"),
            ("${job_id:singlequote}", "''"),
            ("$coordinator", ".*"),
            ("$program", ".*"),
            ("$job_id", ""),
        ];
        let mut out = s.to_owned();
        for (needle, repl) in replacements {
            if out.contains(needle) {
                out = out.replace(needle, repl);
            }
        }
        out
    }

    fn walk(value: &Value) -> Value {
        match value {
            Value::String(s) => Value::String(rewrite(s)),
            Value::Array(arr) => Value::Array(arr.iter().map(walk).collect()),
            Value::Object(map) => {
                let mut new_map = serde_json::Map::with_capacity(map.len());
                for (k, v) in map {
                    new_map.insert(k.clone(), walk(v));
                }
                Value::Object(new_map)
            }
            other => other.clone(),
        }
    }

    walk(target)
}

/// Count the largest frame column returned by `/api/ds/query`.
fn count_rows(payload: &Value) -> usize {
    let Some(results) = payload.get("results").and_then(Value::as_object) else {
        return 0;
    };
    let mut max_rows = 0usize;
    for (_ref_id, result) in results {
        let Some(frames) = result.get("frames").and_then(Value::as_array) else { continue };
        for frame in frames {
            let Some(values) =
                frame.get("data").and_then(|d| d.get("values")).and_then(Value::as_array)
            else {
                continue;
            };
            for column in values {
                if let Some(rows) = column.as_array() {
                    if rows.len() > max_rows {
                        max_rows = rows.len();
                    }
                }
            }
        }
    }
    max_rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn basic_auth_keeps_user_and_secret_separate() {
        let auth_value = ["grafana", "test", "value"].join("-");
        let auth = BasicAuth::new("user", &auth_value);
        assert_eq!(auth.user, "user");
        assert_eq!(auth.password, auth_value);
    }

    #[test]
    fn count_rows_returns_largest_column() {
        let payload = json!({
            "results": {
                "A": {
                    "frames": [
                        {"data": {"values": [[1, 2, 3], [10, 20, 30]]}}
                    ]
                }
            }
        });
        assert_eq!(count_rows(&payload), 3);
    }

    #[test]
    fn count_rows_zero_when_frame_empty() {
        let payload = json!({
            "results": {
                "A": {"frames": [{"data": {"values": []}}]}
            }
        });
        assert_eq!(count_rows(&payload), 0);
    }

    #[test]
    fn count_rows_zero_when_results_missing() {
        let payload = json!({"foo": "bar"});
        assert_eq!(count_rows(&payload), 0);
    }

    #[test]
    fn wrap_target_injects_default_ref_id() {
        let target = json!({
            "datasource": {"type": "prometheus", "uid": "prometheus"},
            "expr": "up"
        });
        let body = wrap_target(&target).expect("wrap");
        let queries = body.get("queries").and_then(Value::as_array).expect("queries");
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].get("refId").and_then(Value::as_str), Some("A"));
    }

    #[test]
    fn wrap_target_preserves_existing_ref_id() {
        let target = json!({"refId": "Q42", "expr": "up"});
        let body = wrap_target(&target).expect("wrap");
        assert_eq!(body["queries"][0].get("refId").and_then(Value::as_str), Some("Q42"),);
    }

    #[test]
    fn substitute_template_vars_handles_all_known_placeholders() {
        let target = json!({
            "expr": "rate(coordinator_jobs_total{coordinator_id=~\"$coordinator\"}[$__rate_interval]) * $__range",
            "url": "/api/v1/jobs/current?program=$program&job_id=$job_id",
        });
        let rewritten = substitute_template_vars(&target);
        let expr = rewritten.get("expr").and_then(Value::as_str).unwrap();
        let url = rewritten.get("url").and_then(Value::as_str).unwrap();
        assert!(expr.contains("coordinator_id=~\".*\""), "got {expr}");
        assert!(expr.contains("[30s]"), "got {expr}");
        assert!(expr.contains("* 1h"), "got {expr}");
        assert_eq!(url, "/api/v1/jobs/current?program=.*&job_id=");
    }
}
