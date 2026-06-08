//! Grafana dashboard assertions used by the live integration harness.

use anyhow::{anyhow, Context, Result};
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde_json::{json, Map, Value};

pub struct DashboardProbe<'a> {
    client: &'a Client,
    grafana_url: String,
    auth: BasicAuth,
    dashboard: Value,
}

#[derive(Debug, Clone)]
struct BasicAuth {
    user: String,
    password: String,
}

impl BasicAuth {
    fn new(user: &str, password: &str) -> Self {
        Self { user: user.to_owned(), password: password.to_owned() }
    }

    fn apply(&self, request: RequestBuilder) -> RequestBuilder {
        request.basic_auth(&self.user, Some(&self.password))
    }
}

#[derive(Debug, Clone, Copy)]
struct DashboardVars<'a> {
    coordinator: Option<&'a str>,
    program: Option<&'a str>,
    job_id: Option<&'a str>,
}

impl<'a> DashboardProbe<'a> {
    pub fn new(
        client: &'a Client,
        grafana_url: &str,
        grafana_user: &str,
        grafana_password: &str,
        dashboard_uid: &str,
    ) -> Result<Self> {
        let auth = BasicAuth::new(grafana_user, grafana_password);
        let grafana_url = trim_trailing_slash(grafana_url);
        let dashboard = fetch_dashboard(client, &grafana_url, dashboard_uid, &auth)?;
        Ok(Self { client, grafana_url, auth, dashboard })
    }

    pub fn recent_history_contains_job(&self, job_id: &str, program: &str) -> Result<bool> {
        let rows = self.query_panel_rows(
            "Recent Proof History",
            DashboardVars { coordinator: None, program: Some(program), job_id: Some(job_id) },
        )?;
        Ok(rows.iter().any(|row| field_as_str(row, "Job ID") == Some(job_id)))
    }

    pub fn program_jobs_count(&self, program: &str) -> Result<Option<u64>> {
        let rows = self.query_panel_rows(
            "Program Performance Summary (24h)",
            DashboardVars { coordinator: None, program: Some(program), job_id: None },
        )?;
        Ok(rows
            .iter()
            .find(|row| field_as_str(row, "Program") == Some(program))
            .and_then(|row| field_as_u64(row, "Jobs")))
    }

    fn query_panel_rows(
        &self,
        title: &str,
        vars: DashboardVars<'_>,
    ) -> Result<Vec<Map<String, Value>>> {
        let panel = find_panel(&self.dashboard, title)
            .ok_or_else(|| anyhow!("Grafana dashboard has no panel titled `{title}`"))?;
        let targets = panel
            .get("targets")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("Grafana panel `{title}` has no targets array"))?;

        let mut rows = Vec::new();
        for target in targets {
            let payload = self.query_target(target, vars)?;
            rows.extend(rows_from_query_payload(&payload));
        }
        Ok(rows)
    }

    fn query_target(&self, target: &Value, vars: DashboardVars<'_>) -> Result<Value> {
        let body = wrap_target(target, vars)?;
        let url = format!("{}/api/ds/query", self.grafana_url);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .auth
            .apply(self.client.post(&url))
            .headers(headers)
            .body(serde_json::to_vec(&body)?)
            .send()
            .with_context(|| format!("POST {url} failed"))?;
        let status = response.status();
        if !status.is_success() {
            let body =
                response.text().unwrap_or_else(|error| format!("<body read failed: {error}>"));
            return Err(anyhow!("{url} returned HTTP {}: {body}", status.as_u16()));
        }
        response.json().with_context(|| format!("{url} returned non-JSON body"))
    }
}

fn fetch_dashboard(
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
        let body = response.text().unwrap_or_else(|error| format!("<body read failed: {error}>"));
        return Err(anyhow!("{url} returned HTTP {}: {body}", status.as_u16()));
    }
    let payload: Value =
        response.json().with_context(|| format!("{url} returned non-JSON body"))?;
    payload
        .get("dashboard")
        .cloned()
        .ok_or_else(|| anyhow!("{url} response had no `dashboard` object"))
}

fn find_panel<'a>(dashboard: &'a Value, title: &str) -> Option<&'a Value> {
    fn walk<'a>(panel: &'a Value, title: &str) -> Option<&'a Value> {
        if panel.get("title").and_then(Value::as_str) == Some(title) {
            return Some(panel);
        }
        let children = panel.get("panels").and_then(Value::as_array)?;
        children.iter().find_map(|child| walk(child, title))
    }

    dashboard
        .get("panels")
        .and_then(Value::as_array)
        .and_then(|panels| panels.iter().find_map(|panel| walk(panel, title)))
}

fn wrap_target(target: &Value, vars: DashboardVars<'_>) -> Result<Value> {
    let mut query = substitute_template_vars(target, vars);
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

fn substitute_template_vars(target: &Value, vars: DashboardVars<'_>) -> Value {
    fn rewrite(s: &str, vars: DashboardVars<'_>) -> String {
        let coordinator = vars.coordinator.unwrap_or(".*");
        let program = vars.program.unwrap_or(".*");
        let job_id = vars.job_id.unwrap_or("");
        let coordinator_single = vars.coordinator.map(sql_quote).unwrap_or_else(|| "''".to_owned());
        let program_single = vars.program.map(sql_quote).unwrap_or_else(|| "''".to_owned());
        let job_id_single = vars.job_id.map(sql_quote).unwrap_or_else(|| "''".to_owned());
        let replacements = [
            ("$__timeFilter(sort_at)", "sort_at >= NOW() - INTERVAL '2 hours'".to_owned()),
            ("$__rate_interval", "30s".to_owned()),
            ("$__interval", "30s".to_owned()),
            ("$__range", "1h".to_owned()),
            ("${coordinator:singlequote}", coordinator_single),
            ("${program:singlequote}", program_single),
            ("${job_id:singlequote}", job_id_single),
            ("$coordinator", coordinator.to_owned()),
            ("$program", program.to_owned()),
            ("$job_id", job_id.to_owned()),
        ];

        let mut out = s.to_owned();
        for (needle, replacement) in replacements {
            if out.contains(needle) {
                out = out.replace(needle, &replacement);
            }
        }
        out
    }

    fn walk(value: &Value, vars: DashboardVars<'_>) -> Value {
        match value {
            Value::String(s) => Value::String(rewrite(s, vars)),
            Value::Array(values) => {
                Value::Array(values.iter().map(|value| walk(value, vars)).collect())
            }
            Value::Object(map) => Value::Object(
                map.iter().map(|(key, value)| (key.clone(), walk(value, vars))).collect(),
            ),
            other => other.clone(),
        }
    }

    walk(target, vars)
}

fn rows_from_query_payload(payload: &Value) -> Vec<Map<String, Value>> {
    let Some(results) = payload.get("results").and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for result in results.values() {
        let Some(frames) = result.get("frames").and_then(Value::as_array) else {
            continue;
        };
        for frame in frames {
            let field_names = frame
                .get("schema")
                .and_then(|schema| schema.get("fields"))
                .and_then(Value::as_array)
                .map(|fields| {
                    fields
                        .iter()
                        .map(|field| {
                            field.get("name").and_then(Value::as_str).unwrap_or_default().to_owned()
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let columns = frame
                .get("data")
                .and_then(|data| data.get("values"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let row_count =
                columns.iter().filter_map(Value::as_array).map(Vec::len).max().unwrap_or_default();
            for row_idx in 0..row_count {
                let mut row = Map::new();
                for (col_idx, name) in field_names.iter().enumerate() {
                    if name.is_empty() {
                        continue;
                    }
                    let value = columns
                        .get(col_idx)
                        .and_then(Value::as_array)
                        .and_then(|values| values.get(row_idx))
                        .cloned()
                        .unwrap_or(Value::Null);
                    row.insert(name.clone(), value);
                }
                rows.push(row);
            }
        }
    }
    rows
}

fn field_as_str<'a>(row: &'a Map<String, Value>, field: &str) -> Option<&'a str> {
    row.get(field).and_then(Value::as_str)
}

fn field_as_u64(row: &Map<String, Value>, field: &str) -> Option<u64> {
    match row.get(field)? {
        Value::Number(number) => number.as_u64().or_else(|| number.as_f64().map(|v| v as u64)),
        Value::String(value) => value.parse().ok(),
        _ => None,
    }
}

fn sql_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn trim_trailing_slash(value: &str) -> String {
    value.trim_end_matches('/').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn substitute_template_vars_quotes_sql_filters() {
        let target = json!({
            "rawSql": "job_id = ${job_id:singlequote} AND program = ANY(ARRAY[${program:singlequote}]::text[])",
            "expr": "rate(x{program=~\"$program\"}[$__rate_interval])",
        });
        let vars = DashboardVars { coordinator: None, program: Some("alpha"), job_id: Some("abc") };
        let out = substitute_template_vars(&target, vars);
        let sql = out.get("rawSql").and_then(Value::as_str).unwrap();
        let expr = out.get("expr").and_then(Value::as_str).unwrap();
        assert!(sql.contains("job_id = 'abc'"), "sql={sql}");
        assert!(sql.contains("ARRAY['alpha']"), "sql={sql}");
        assert!(expr.contains("program=~\"alpha\""), "expr={expr}");
        assert!(expr.contains("[30s]"), "expr={expr}");
    }

    #[test]
    fn rows_from_query_payload_uses_schema_field_names() {
        let payload = json!({
            "results": {
                "A": {
                    "frames": [{
                        "schema": {"fields": [{"name": "Job ID"}, {"name": "Jobs"}]},
                        "data": {"values": [["abc", "def"], [1, 2]]}
                    }]
                }
            }
        });
        let rows = rows_from_query_payload(&payload);
        assert_eq!(rows.len(), 2);
        assert_eq!(field_as_str(&rows[0], "Job ID"), Some("abc"));
        assert_eq!(field_as_u64(&rows[1], "Jobs"), Some(2));
    }

    #[test]
    fn find_panel_walks_nested_rows() {
        let dashboard = json!({
            "panels": [{
                "title": "Row",
                "panels": [{"title": "Recent Proof History", "targets": []}]
            }]
        });
        assert!(find_panel(&dashboard, "Recent Proof History").is_some());
        assert!(find_panel(&dashboard, "Missing").is_none());
    }
}
