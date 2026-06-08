//! Grafana dashboard probe for refresh, required titles, variables, and datasources.

use anyhow::Result;
use reqwest::blocking::Client;
use serde_json::Value;

use crate::contract::REQUIRED_PANEL_TITLES;
use crate::http::{request_value_with_auth, BasicAuth};

const LIVE_JSON_PANELS: &[(&str, &str)] = &[
    ("Current Proof Phase (now)", "/api/v1/jobs/current"),
    ("Current Proof Duration (now)", "/api/v1/jobs/current"),
    ("Current Phase Age (now)", "/api/v1/jobs/current"),
    ("Progress Update Age (now)", "/api/v1/jobs/current"),
    ("Workers Assigned (now)", "/api/v1/jobs/current"),
    ("Worker Roster", "/api/v1/workers"),
];

const POSTGRES_HISTORY_PANELS: &[&str] = &[
    "Proof Phase Progress (latest jobs)",
    "Average Proof Duration (last 10 successes)",
    "Proof Success Rate (24h)",
    "Proof Failures by Reason (selected range)",
    "Proof Duration Stats (24h)",
    "Recent Proof History",
    "Proof Duration Distribution (recent jobs)",
    "Proof Duration Quantiles (24h)",
    "Proof Duration by Cost (all proofs)",
    "Program Performance Summary (24h)",
    "Worker Error Events",
];

pub fn run(
    client: &Client,
    grafana_url: &str,
    dashboard_uid: &str,
    expected_refresh: &str,
    auth: &BasicAuth,
) -> Result<Vec<String>> {
    let mut errors = Vec::new();

    let encoded_uid = urlencoding::encode(dashboard_uid);
    let url = format!("{grafana_url}/api/dashboards/uid/{encoded_uid}");
    let payload: Value = request_value_with_auth(client, &url, auth)?;
    let dashboard = payload.get("dashboard").cloned().unwrap_or(Value::Null);

    let refresh = dashboard.get("refresh").cloned().unwrap_or(Value::Null);
    let refresh_matches = refresh.as_str() == Some(expected_refresh);
    if !refresh_matches {
        errors.push(format!(
            "Grafana dashboard refresh is {refresh_repr:?}, expected {expected_refresh:?}; \
             set GF_DASHBOARDS_MIN_REFRESH_INTERVAL=1s and re-import/restart Grafana",
            refresh_repr = display_refresh(&refresh),
        ));
    }

    let mut panel_titles: Vec<String> = Vec::new();
    collect_panel_titles(&dashboard, &mut panel_titles);
    let missing = missing_panel_titles(&panel_titles);
    if !missing.is_empty() {
        errors.push(format!("Grafana dashboard is missing expected panels: {missing:?}"));
    }

    let templating = dashboard
        .get("templating")
        .and_then(|t| t.get("list"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let has_job_id = templating.iter().any(|var| match_template(var, "job_id", "textbox"));
    if !has_job_id {
        errors.push("Grafana dashboard is missing the job_id textbox variable".to_owned());
    }
    let has_program = templating.iter().any(|var| match_template(var, "program", "query"));
    if !has_program {
        errors.push("Grafana dashboard is missing the program query variable".to_owned());
    }

    errors.extend(validate_datasource_split(&dashboard));

    let uid = dashboard.get("uid").and_then(Value::as_str).unwrap_or("-");
    let version = payload
        .get("meta")
        .and_then(|meta| meta.get("version"))
        .map(format_version)
        .unwrap_or_else(|| "-".to_owned());
    let refresh_display =
        refresh.as_str().map(|s| s.to_owned()).unwrap_or_else(|| display_refresh(&refresh));
    println!("grafana: uid={uid} version={version} refresh={refresh_display}");

    Ok(errors)
}

fn collect_panel_titles(dashboard: &Value, out: &mut Vec<String>) {
    let mut stack: Vec<Value> = match dashboard.get("panels").and_then(Value::as_array) {
        Some(array) => array.clone(),
        None => return,
    };
    while let Some(panel) = stack.pop() {
        if let Some(title) = panel.get("title").and_then(Value::as_str) {
            out.push(title.to_owned());
        }
        if let Some(children) = panel.get("panels").and_then(Value::as_array) {
            stack.extend(children.iter().cloned());
        }
    }
}

fn walk_dashboard_panels(dashboard: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    let mut stack: Vec<Value> = match dashboard.get("panels").and_then(Value::as_array) {
        Some(array) => array.clone(),
        None => return out,
    };
    while let Some(panel) = stack.pop() {
        if let Some(children) = panel.get("panels").and_then(Value::as_array) {
            stack.extend(children.iter().cloned());
        }
        out.push(panel);
    }
    out
}

fn validate_datasource_split(dashboard: &Value) -> Vec<String> {
    let panels = walk_dashboard_panels(dashboard);
    let mut errors = Vec::new();

    for (title, expected_url) in LIVE_JSON_PANELS {
        let Some(panel) =
            panels.iter().find(|panel| panel.get("title").and_then(Value::as_str) == Some(*title))
        else {
            continue;
        };
        if !panel_uses_datasource(panel, "zisk-json") {
            errors.push(format!("{title}: live panel must use zisk-json"));
        }
        let urls = target_strings(panel, "url");
        if !urls.iter().any(|url| url.contains(expected_url)) {
            errors.push(format!("{title}: live panel must read {expected_url}"));
        }
    }

    for title in POSTGRES_HISTORY_PANELS {
        let Some(panel) =
            panels.iter().find(|panel| panel.get("title").and_then(Value::as_str) == Some(*title))
        else {
            continue;
        };
        if !panel_uses_datasource(panel, "zisk-postgres") {
            errors.push(format!("{title}: history panel must use zisk-postgres"));
        }
        if target_strings(panel, "rawSql").is_empty() {
            errors.push(format!("{title}: history panel must use rawSql"));
        }
        if !target_strings(panel, "url").is_empty() {
            errors.push(format!("{title}: history panel must not call coordinator JSON"));
        }
    }

    errors
}

fn panel_uses_datasource(panel: &Value, uid: &str) -> bool {
    panel.get("datasource").and_then(|datasource| datasource.get("uid")).and_then(Value::as_str)
        == Some(uid)
        || panel.get("targets").and_then(Value::as_array).into_iter().flatten().any(|target| {
            target
                .get("datasource")
                .and_then(|datasource| datasource.get("uid"))
                .and_then(Value::as_str)
                == Some(uid)
        })
}

fn target_strings(panel: &Value, key: &str) -> Vec<String> {
    panel
        .get("targets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|target| target.get(key).and_then(Value::as_str))
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn missing_panel_titles(panel_titles: &[String]) -> Vec<String> {
    let mut missing: Vec<String> = REQUIRED_PANEL_TITLES
        .iter()
        .filter(|title| !panel_titles.iter().any(|seen| seen == **title))
        .map(|title| (*title).to_owned())
        .collect();
    missing.sort();
    missing
}

fn match_template(var: &Value, name: &str, expected_type: &str) -> bool {
    let var_name = var.get("name").and_then(Value::as_str);
    let var_type = var.get("type").and_then(Value::as_str);
    var_name == Some(name) && var_type == Some(expected_type)
}

fn format_version(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Null => "-".to_owned(),
        other => other.to_string(),
    }
}

fn display_refresh(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_owned(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn collect_panel_titles_walks_nested_rows() {
        let dashboard = json!({
            "panels": [
                {"title": "Top"},
                {
                    "title": "Row",
                    "panels": [
                        {"title": "Nested-A"},
                        {"title": "Nested-B", "panels": [{"title": "Deep"}]},
                    ]
                }
            ]
        });
        let mut titles = Vec::new();
        collect_panel_titles(&dashboard, &mut titles);
        titles.sort();
        assert_eq!(titles, vec!["Deep", "Nested-A", "Nested-B", "Row", "Top"]);
    }

    #[test]
    fn match_template_requires_both_name_and_type() {
        let var = json!({"name": "job_id", "type": "textbox"});
        assert!(match_template(&var, "job_id", "textbox"));
        assert!(!match_template(&var, "job_id", "query"));
        assert!(!match_template(&var, "program", "textbox"));
    }

    #[test]
    fn datasource_split_accepts_live_json_and_postgres_history() {
        let dashboard = json!({
            "panels": [
                {
                    "title": "Current Proof Phase (now)",
                    "datasource": {"uid": "zisk-json"},
                    "targets": [{"datasource": {"uid": "zisk-json"}, "url": "/api/v1/jobs/current"}]
                },
                {
                    "title": "Recent Proof History",
                    "datasource": {"uid": "zisk-postgres"},
                    "targets": [{"datasource": {"uid": "zisk-postgres"}, "rawSql": "SELECT 1"}]
                }
            ]
        });
        let errors = validate_datasource_split(&dashboard);
        assert!(errors.is_empty(), "errors: {errors:?}");
    }

    #[test]
    fn datasource_split_rejects_history_json_regression() {
        let dashboard = json!({
            "panels": [
                {
                    "title": "Recent Proof History",
                    "datasource": {"uid": "zisk-json"},
                    "targets": [{"datasource": {"uid": "zisk-json"}, "url": "/api/v1/jobs/recent"}]
                }
            ]
        });
        let errors = validate_datasource_split(&dashboard);
        assert!(errors.iter().any(|error| error.contains("zisk-postgres")), "errors: {errors:?}");
        assert!(
            errors.iter().any(|error| error.contains("must not call coordinator JSON")),
            "errors: {errors:?}"
        );
    }

    #[test]
    fn missing_panel_titles_reports_sorted_gaps() {
        let seen =
            vec!["Coordinator Availability (now)".to_owned(), "Workers Connected (now)".to_owned()];
        let missing = missing_panel_titles(&seen);
        assert!(!missing.is_empty());
        let mut sorted = missing.clone();
        sorted.sort();
        assert_eq!(missing, sorted);
        assert!(!missing.iter().any(|m| m == "Coordinator Availability (now)"));
    }
}
