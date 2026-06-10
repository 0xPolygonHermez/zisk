//! Dashboard JSON validation logic.

use std::collections::BTreeSet;
use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

use crate::contract::Contract;
use crate::rules::{
    BANNED_DASHBOARD_URL_FRAGMENTS, BANNED_PROMQL_FRAGMENTS, EXPECTED_DASHBOARD_UID,
    HISTOGRAM_SNAPSHOT_PANELS, POSTGRES_HISTORY_PANELS, PROGRAM_PANELS, PROMETHEUS_TOP_ROW,
    PROMETHEUS_TREND_PANELS, PROMQL_FUNCTIONS, PROOF_RUN_SUMMARY_CARDS, REQUIRED_METRIC_PANELS,
    REQUIRED_PANEL_TITLES,
};

// --- regex singletons ------------------------------------------------------

fn histogram_quantile_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?s)histogram_quantile\s*\([^,]+,\s*.*coordinator_job_duration_seconds_bucket")
            .unwrap()
    })
}

fn job_duration_quantile_selector_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| {
        Regex::new(r"(?s)coordinator_job_duration_seconds\s*\{[^}]*\bquantile\s*=").unwrap()
    })
}

fn sql_alias_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r#"(?i)\bas\s+"([^"]+)""#).unwrap())
}

// --- helpers ---------------------------------------------------------------

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == ':'
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == ':'
}

/// Extract metric-like identifiers from PromQL expressions.
fn promql_metric_names(expr: &str) -> BTreeSet<String> {
    let bytes = expr.as_bytes();
    let len = bytes.len();
    let mut out: BTreeSet<String> = BTreeSet::new();
    let functions: HashSet<&'static str> = PROMQL_FUNCTIONS.iter().copied().collect();

    let mut i = 0usize;
    while i < len {
        let c = bytes[i] as char;
        if !is_ident_start(c) {
            i += 1;
            continue;
        }
        if i > 0 {
            let prev = bytes[i - 1] as char;
            if is_ident_char(prev) {
                i += 1;
                continue;
            }
        }
        let start = i;
        while i < len && is_ident_char(bytes[i] as char) {
            i += 1;
        }
        let end = i;
        let mut j = end;
        while j < len {
            let cc = bytes[j] as char;
            if cc == ' ' || cc == '\t' || cc == '\n' || cc == '\r' || cc == 0x0C as char {
                j += 1;
            } else {
                break;
            }
        }
        let matched = if j == len {
            true
        } else {
            let nc = bytes[j] as char;
            nc == '{' || nc == '['
        };
        if matched {
            let ident = &expr[start..end];
            if !functions.contains(ident) {
                out.insert(ident.to_string());
            }
        }
    }
    out
}

/// Walk every panel in the dashboard, including nested rows.
fn walk_panels(dashboard: &Value) -> Vec<&Value> {
    let mut panels: Vec<&Value> = Vec::new();
    let mut stack: Vec<&Value> = Vec::new();
    if let Some(arr) = dashboard.get("panels").and_then(|v| v.as_array()) {
        for p in arr {
            stack.push(p);
        }
    }
    while let Some(p) = stack.pop() {
        panels.push(p);
        if let Some(arr) = p.get("panels").and_then(|v| v.as_array()) {
            for child in arr {
                stack.push(child);
            }
        }
    }
    panels
}

/// Recursive walk yielding every string value in the JSON tree.
fn dashboard_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => out.push(s.clone()),
        Value::Array(arr) => {
            for child in arr {
                dashboard_strings(child, out);
            }
        }
        Value::Object(obj) => {
            for (_, child) in obj {
                dashboard_strings(child, out);
            }
        }
        _ => {}
    }
}

/// Returns the URL path (no query, no fragment).
fn url_path(url: &str) -> &str {
    let p = url.split_once('?').map(|(p, _)| p).unwrap_or(url);
    p.split_once('#').map(|(p, _)| p).unwrap_or(p)
}

#[derive(Clone, Copy)]
struct GridRect {
    x: i64,
    y: i64,
    w: i64,
    h: i64,
}

fn grid_rect(panel: &Value) -> Option<GridRect> {
    let grid = panel.get("gridPos")?;
    Some(GridRect {
        x: grid.get("x")?.as_i64()?,
        y: grid.get("y")?.as_i64()?,
        w: grid.get("w")?.as_i64()?,
        h: grid.get("h")?.as_i64()?,
    })
}

fn rects_overlap(left: GridRect, right: GridRect) -> bool {
    left.x < right.x + right.w
        && right.x < left.x + left.w
        && left.y < right.y + right.h
        && right.y < left.y + left.h
}

fn validate_top_level_grid(dashboard: &Value, errors: &mut Vec<String>) {
    let Some(panels) = dashboard.get("panels").and_then(Value::as_array) else {
        return;
    };
    for (left_idx, left) in panels.iter().enumerate() {
        let Some(left_rect) = grid_rect(left) else {
            errors.push(format!("{}: missing gridPos", panel_title_or_id(left)));
            continue;
        };
        for right in panels.iter().skip(left_idx + 1) {
            let Some(right_rect) = grid_rect(right) else {
                continue;
            };
            if rects_overlap(left_rect, right_rect) {
                errors.push(format!(
                    "{} overlaps {}",
                    panel_title_or_id(left),
                    panel_title_or_id(right)
                ));
            }
        }
    }
}

// --- panel accessors -------------------------------------------------------

fn panel_title(panel: &Value) -> Option<&str> {
    panel.get("title").and_then(|v| v.as_str())
}

fn panel_title_or_id(panel: &Value) -> String {
    if let Some(t) = panel_title(panel) {
        return t.to_string();
    }
    let id_str = match panel.get("id") {
        Some(Value::Number(n)) => {
            if let Some(u) = n.as_u64() {
                u.to_string()
            } else if let Some(i) = n.as_i64() {
                i.to_string()
            } else if let Some(f) = n.as_f64() {
                f.to_string()
            } else {
                "?".to_string()
            }
        }
        Some(Value::String(s)) => s.clone(),
        _ => "?".to_string(),
    };
    format!("panel {}", id_str)
}

fn panel_targets(panel: &Value) -> Vec<&Value> {
    panel.get("targets").and_then(|v| v.as_array()).map(|a| a.iter().collect()).unwrap_or_default()
}

fn datasource_uid(node: &Value) -> Option<&str> {
    node.get("datasource").and_then(|v| v.get("uid")).and_then(|v| v.as_str())
}

fn options_str<'a>(panel: &'a Value, key: &str) -> Option<&'a str> {
    panel.get("options").and_then(|v| v.get(key)).and_then(|v| v.as_str())
}

fn options_u64(panel: &Value, key: &str) -> Option<u64> {
    panel.get("options").and_then(|v| v.get(key)).and_then(|v| v.as_u64())
}

fn target_str<'a>(t: &'a Value, key: &str) -> &'a str {
    t.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

fn target_bool(t: &Value, key: &str) -> bool {
    t.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn find_top_panel<'a>(dashboard: &'a Value, title: &str) -> Option<&'a Value> {
    let arr = dashboard.get("panels")?.as_array()?;
    arr.iter().find(|p| panel_title(p) == Some(title))
}

fn find_panel<'a>(panels: &'a [&'a Value], title: &str) -> Option<&'a Value> {
    panels.iter().copied().find(|p| panel_title(p) == Some(title))
}

fn target_columns(target: &Value) -> Vec<&Value> {
    target.get("columns").and_then(|v| v.as_array()).map(|a| a.iter().collect()).unwrap_or_default()
}

fn raw_sqls(panel: &Value) -> Vec<&str> {
    panel_targets(panel).iter().map(|t| target_str(t, "rawSql")).filter(|s| !s.is_empty()).collect()
}

fn target_datasource_uid(target: &Value) -> Option<&str> {
    target.get("datasource").and_then(|v| v.get("uid")).and_then(|v| v.as_str())
}

fn panel_uses_datasource(panel: &Value, uid: &str) -> bool {
    datasource_uid(panel) == Some(uid)
        || panel_targets(panel).iter().any(|target| target_datasource_uid(target) == Some(uid))
}

fn sql_contains(panel: &Value, needle: &str) -> bool {
    raw_sqls(panel).iter().any(|sql| sql.contains(needle))
}

fn sql_contains_any(panel: &Value, needles: &[&str]) -> bool {
    needles.iter().any(|needle| sql_contains(panel, needle))
}

fn require_postgres_history_panel(title: &str, panel: &Value, errors: &mut Vec<String>) {
    if !panel_uses_datasource(panel, "zisk-postgres") {
        errors.push(format!("{}: history/reporting panel must use zisk-postgres", title));
    }
    let urls: Vec<String> =
        panel_targets(panel).iter().map(|t| target_str(t, "url").to_string()).collect();
    if urls.iter().any(|url| !url.is_empty()) {
        errors.push(format!("{}: history/reporting panel must not call coordinator JSON", title));
    }
    if raw_sqls(panel).is_empty() {
        errors.push(format!("{}: history/reporting panel must use rawSql", title));
    }
}

fn collect_column_field(panel: &Value, field: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for t in panel_targets(panel) {
        for c in target_columns(t) {
            if let Some(s) = c.get(field).and_then(|v| v.as_str()) {
                out.insert(s.to_string());
            }
        }
        if field == "text" {
            let sql = target_str(t, "rawSql");
            for captures in sql_alias_re().captures_iter(sql) {
                if let Some(alias) = captures.get(1) {
                    out.insert(alias.as_str().to_string());
                }
            }
        }
    }
    out
}

// --- per-target validation -------------------------------------------------

fn validate_promql_contract(
    title: &str,
    expr: &str,
    contract: &Contract,
    errors: &mut Vec<String>,
) {
    if job_duration_quantile_selector_re().is_match(expr) {
        errors.push(format!(
            "{}: coordinator_job_duration_seconds is histogram-only; \
             use histogram_quantile over coordinator_job_duration_seconds_bucket instead of quantile selectors",
            title
        ));
    }
    for metric in promql_metric_names(expr) {
        if metric.starts_with("coordinator_") && !contract.metrics.contains(&metric) {
            errors.push(format!("{}: unknown coordinator metric '{}'", title, metric));
        }
    }
}

fn validate_json_api_url(title: &str, url: &str, contract: &Contract, errors: &mut Vec<String>) {
    if !url.starts_with("/api/") {
        return;
    }
    let path = url_path(url);
    if path.starts_with("/api/v1/jobs/") && !contract.routes.contains(path) {
        let job_id = path.trim_start_matches("/api/v1/jobs/");
        let looks_like_uuid =
            job_id.len() == 36 && job_id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
        if looks_like_uuid && contract.routes.contains("/api/v1/jobs/{job_id}") {
            return;
        }
    }
    if !contract.routes.contains(path) {
        errors.push(format!(
            "{}: JSON API endpoint '{}' is not implemented by the coordinator",
            title, path
        ));
    }
}

// --- main entry ------------------------------------------------------------

pub fn validate(dashboard: &Value, contract: &Contract) -> Vec<String> {
    let mut errors: Vec<String> = Vec::new();
    let panels = walk_panels(dashboard);

    // refresh
    if dashboard.get("refresh").and_then(|v| v.as_str()) != Some("1s") {
        errors.push("dashboard source refresh must remain 1s".to_string());
    }

    // uid
    if dashboard.get("uid").and_then(|v| v.as_str()) != Some(EXPECTED_DASHBOARD_UID) {
        errors.push(format!("dashboard uid must remain '{}'", EXPECTED_DASHBOARD_UID));
    }

    validate_top_level_grid(dashboard, &mut errors);

    // templating variables
    let templating_list: Vec<&Value> = dashboard
        .get("templating")
        .and_then(|v| v.get("list"))
        .and_then(|v| v.as_array())
        .map(|a| a.iter().collect())
        .unwrap_or_default();

    let find_var = |name: &str| -> Option<&Value> {
        templating_list
            .iter()
            .copied()
            .find(|v| v.get("name").and_then(|n| n.as_str()) == Some(name))
    };

    match find_var("job_id") {
        None => errors.push(
            "dashboard must expose a job_id textbox variable for consumer-scoped inspection"
                .to_string(),
        ),
        Some(v) => {
            if v.get("type").and_then(|t| t.as_str()) != Some("textbox") {
                errors.push(
                    "job_id dashboard variable must be a textbox so operators can paste their UUID"
                        .to_string(),
                );
            }
        }
    }

    match find_var("program") {
        None => errors.push(
            "dashboard must expose a program variable using the program alias label".to_string(),
        ),
        Some(v) => {
            if v.get("type").and_then(|t| t.as_str()) != Some("query") {
                errors.push(
                    "program dashboard variable must be a Prometheus query variable".to_string(),
                );
            } else {
                let query_str = match v.get("query") {
                    None => String::new(),
                    Some(Value::String(s)) => s.clone(),
                    Some(other) => other.to_string(),
                };
                if !query_str.contains("program") {
                    errors.push(
                        "program dashboard variable must query the program label, not raw hash_id"
                            .to_string(),
                    );
                }
            }
        }
    }

    // required panel titles
    let panel_title_set: BTreeSet<String> =
        panels.iter().filter_map(|p| panel_title(p).map(|s| s.to_string())).collect();
    let required: BTreeSet<String> = REQUIRED_PANEL_TITLES.iter().map(|s| s.to_string()).collect();
    let mut missing: Vec<&String> = required.difference(&panel_title_set).collect();
    missing.sort();
    for title in missing {
        errors.push(format!("missing required operator panel: {}", title));
    }

    // banned dashboard URL fragments (recursive string walk)
    let mut all_strings: Vec<String> = Vec::new();
    dashboard_strings(dashboard, &mut all_strings);
    for (fragment, reason) in BANNED_DASHBOARD_URL_FRAGMENTS {
        if all_strings.iter().any(|s| s.contains(fragment)) {
            errors
                .push(format!("dashboard contains banned URL fragment '{}': {}", fragment, reason));
        }
    }

    // per-panel checks
    for panel in &panels {
        let title = panel_title_or_id(panel);
        let title_str = title.as_str();

        if title_str.contains(" - ") {
            errors.push(format!(
                "{}: panel titles must use descriptive scope parentheses instead of `X - Y` naming",
                title
            ));
        }

        // Non-row panels must be documented for the generated runbook.
        let panel_type = panel.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if panel_type != "row" {
            let description = panel.get("description").and_then(|v| v.as_str()).unwrap_or("");
            let len = description.chars().count();
            if len < 60 {
                errors.push(format!(
                    "{}: panel must include an operator-facing description of at least 60 chars (currently {} chars)",
                    title, len
                ));
            }
        }

        if PROMETHEUS_TOP_ROW.contains(&title_str) {
            if datasource_uid(panel) != Some("prometheus") {
                errors.push(format!(
                    "{}: top-row Prometheus status panel must use Prometheus",
                    title
                ));
            }
            if options_str(panel, "graphMode") != Some("none") {
                errors.push(format!("{}: top-row status panel must not render a sparkline", title));
            }
            for target in panel_targets(panel) {
                if !target_bool(target, "instant") {
                    errors.push(format!(
                        "{}: top-row Prometheus status panel must use instant queries",
                        title
                    ));
                }
            }
        }

        if POSTGRES_HISTORY_PANELS.contains(&title_str) {
            require_postgres_history_panel(&title, panel, &mut errors);
        }

        for target in panel_targets(panel) {
            let expr = target_str(target, "expr");
            let url = target_str(target, "url");
            for (fragment, reason) in BANNED_PROMQL_FRAGMENTS {
                if expr.contains(fragment) {
                    errors.push(format!(
                        "{}: banned PromQL fragment '{}': {}",
                        title, fragment, reason
                    ));
                }
            }
            for (fragment, reason) in BANNED_DASHBOARD_URL_FRAGMENTS {
                if url.contains(fragment) {
                    errors
                        .push(format!("{}: banned URL fragment '{}': {}", title, fragment, reason));
                }
            }
            if !expr.is_empty() {
                validate_promql_contract(&title, expr, contract, &mut errors);
            }
            if !url.is_empty() && !url.starts_with('/') {
                errors.push(format!("{}: Infinity URL must be relative, got '{}'", title, url));
            }
            if !url.is_empty() {
                validate_json_api_url(&title, url, contract, &mut errors);
            }
            if expr.contains("coordinator_requests_total") && expr.contains("outcome") {
                errors.push(format!(
                    "{}: v0.18 coordinator_requests_total uses status, not outcome",
                    title
                ));
            }
            if PROGRAM_PANELS.contains(&title_str)
                && !url.is_empty()
                && !url.contains("program=$program")
            {
                errors.push(format!("{}: JSON API panel must honor the program variable", title));
            }
        }
    }

    // "Current Proof" groups the live run cards.
    let summary = find_top_panel(dashboard, "Current Proof");
    match summary {
        None => errors.push("missing Current Proof row".to_string()),
        Some(panel) => {
            let ty = panel.get("type").and_then(|v| v.as_str());
            let collapsed = panel.get("collapsed").and_then(|v| v.as_bool());
            if ty != Some("row") || collapsed != Some(false) {
                errors.push(
                    "Current Proof must be an expanded row grouping compact stat cards".to_string(),
                );
            }
        }
    }

    for (title, selector) in PROOF_RUN_SUMMARY_CARDS {
        let card = find_panel(&panels, title);
        let card = match card {
            None => {
                errors.push(format!("missing current proof card: {}", title));
                continue;
            }
            Some(c) => c,
        };
        if card.get("type").and_then(|v| v.as_str()) != Some("stat") {
            errors.push(format!("{}: current proof panel must be a stat panel", title));
        }
        if datasource_uid(card) != Some("zisk-json") {
            errors.push(format!("{}: current proof card must use the coordinator JSON API", title));
        }
        let urls: Vec<String> =
            panel_targets(card).iter().map(|t| target_str(t, "url").to_string()).collect();
        if !urls.iter().any(|u| u.contains("/api/v1/jobs/current")) {
            errors.push(format!("{}: current proof card must use /api/v1/jobs/current", title));
        }
        if urls.iter().any(|u| u.contains("job_id=$job_id")) {
            errors.push(format!(
                "{}: current proof card must not pass job_id; non-UUID textbox input breaks live cards",
                title
            ));
        }
        let selectors: BTreeSet<String> = collect_column_field(card, "selector");
        if !selectors.contains(*selector) {
            errors.push(format!("{}: current proof card must read {}", title, selector));
        }
    }

    // Success rate is the restart-proof SLO card for Reliability.
    if let Some(sr) = find_panel(&panels, "Proof Success Rate (24h)") {
        if sr.get("type").and_then(|v| v.as_str()) != Some("stat") {
            errors
                .push("Proof Success Rate (24h) must remain a single-value stat panel".to_string());
        }
        if !panel_uses_datasource(sr, "zisk-postgres") {
            errors.push(
                "Proof Success Rate (24h) must read Postgres history so it is restart-proof"
                    .to_string(),
            );
        }
        if !sql_contains(sr, "INTERVAL '86400 seconds'") {
            errors.push(
                "Proof Success Rate (24h) must constrain the rollup window to the last 24 hours"
                    .to_string(),
            );
        }
        let unit =
            sr.get("fieldConfig").and_then(|v| v.get("defaults")).and_then(|v| v.get("unit"));
        if unit.and_then(|v| v.as_str()) != Some("percentunit") {
            errors.push(
                "Proof Success Rate (24h) must render as a percent so the SLO band is readable"
                    .to_string(),
            );
        }
    }

    // Average Proof Duration (last 10 successes)
    if let Some(avg) = find_panel(&panels, "Average Proof Duration (last 10 successes)") {
        if avg.get("type").and_then(|v| v.as_str()) != Some("stat") {
            errors.push(
                "Average Proof Duration (last 10 successes) must remain a single-value stat panel"
                    .to_string(),
            );
        }
        if !panel_uses_datasource(avg, "zisk-postgres") {
            errors.push(
                "Average Proof Duration (last 10 successes) must read Postgres history".to_string(),
            );
        }
        if !sql_contains(avg, "LIMIT 10") || !sql_contains(avg, "ORDER BY sort_at DESC") {
            errors.push(
                "Average Proof Duration (last 10 successes) must average the newest 10 matching proofs".to_string(),
            );
        }
        if !sql_contains(avg, "outcome = 'success'") || !sql_contains(avg, "AVG(duration_ms)") {
            errors.push(
                "Average Proof Duration (last 10 successes) must average successful proof durations".to_string(),
            );
        }
        let unit =
            avg.get("fieldConfig").and_then(|v| v.get("defaults")).and_then(|v| v.get("unit"));
        if unit.and_then(|v| v.as_str()) != Some("ms") {
            errors.push(
                "Average Proof Duration (last 10 successes) must render with the ms duration unit"
                    .to_string(),
            );
        }
    }

    // Active Proofs (now)
    if let Some(active_jobs) = find_panel(&panels, "Active Proofs (now)") {
        let targets = panel_targets(active_jobs);
        let exprs: Vec<String> =
            targets.iter().map(|t| target_str(t, "expr").to_string()).collect();
        if !exprs.iter().any(|e| e.contains("coordinator_active_jobs") && !e.contains("kind=\"")) {
            errors.push(
                "Active Proofs (now) must include all active job kinds instead of hiding execute jobs"
                    .to_string(),
            );
        }
    }

    // Proof Failure Rate by Kind (5m)
    if let Some(fr) = find_panel(&panels, "Proof Failure Rate by Kind (5m)") {
        let targets = panel_targets(fr);
        let exprs: Vec<String> =
            targets.iter().map(|t| target_str(t, "expr").to_string()).collect();
        let legends: Vec<String> =
            targets.iter().map(|t| target_str(t, "legendFormat").to_string()).collect();
        if !exprs
            .iter()
            .any(|e| e.contains("sum by (kind)") && e.contains("coordinator_jobs_total"))
        {
            errors.push("Proof Failure Rate by Kind (5m) must preserve kind labels".to_string());
        }
        if !legends.iter().any(|l| l.contains("{{kind}}")) {
            errors.push("Proof Failure Rate by Kind (5m) legend must show kind".to_string());
        }
    }

    // Failure reasons are restart-proof history, not restart-scoped counters.
    if let Some(fr) = find_panel(&panels, "Proof Failures by Reason (selected range)") {
        if fr.get("type").and_then(|v| v.as_str()) != Some("bargauge") {
            errors.push(
                "Proof Failures by Reason (selected range) must remain a taxonomy bar gauge"
                    .to_string(),
            );
        }
        if !panel_uses_datasource(fr, "zisk-postgres") {
            errors.push(
                "Proof Failures by Reason (selected range) must use Postgres proof history"
                    .to_string(),
            );
        }
        let renders_all_rows = fr
            .get("options")
            .and_then(|v| v.get("reduceOptions"))
            .and_then(|v| v.get("values"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !renders_all_rows {
            errors.push(
                "Proof Failures by Reason (selected range) must render all reason rows".to_string(),
            );
        }
        if sql_contains(fr, "coordinator_job_failures_total") {
            errors.push(
                "Proof Failures by Reason (selected range) must not use restart-scoped Prometheus counters"
                    .to_string(),
            );
        }
        for required in
            &["failure_reason", "$__timeFilter(sort_at)", "GROUP BY reason", "COUNT(*)::bigint"]
        {
            if !sql_contains(fr, required) {
                errors.push(format!(
                    "Proof Failures by Reason (selected range) must include {}",
                    required
                ));
            }
        }
    }

    // Phase Share by Proof (last 5 terminal jobs)
    if let Some(timeline) = find_panel(&panels, "Phase Share by Proof (last 5 terminal jobs)") {
        if timeline.get("type").and_then(|v| v.as_str()) != Some("barchart") {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must remain a horizontal stacked phase-duration chart"
                    .to_string(),
            );
        }
        if !panel_uses_datasource(timeline, "zisk-postgres") {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must use Postgres history".to_string(),
            );
        }
        if !sql_contains(timeline, "LIMIT 5") {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must default to the last 5 jobs"
                    .to_string(),
            );
        }
        if sql_contains(timeline, "state NOT IN ('Completed', 'Failed', 'Cancelled')") {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must show recent history, not only active jobs"
                    .to_string(),
            );
        }
        if !sql_contains(timeline, "outcome IN ('success', 'failure', 'cancelled')") {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must use terminal jobs so open phases do not stretch the chart"
                    .to_string(),
            );
        }
        if !sql_contains(timeline, "${job_id:singlequote}") {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must honor the job_id variable"
                    .to_string(),
            );
        }
        if !sql_contains(timeline, "${program:singlequote}") {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must honor the program variable"
                    .to_string(),
            );
        }
        let columns_text: BTreeSet<String> = collect_column_field(timeline, "text");
        let mut selectors_by_text: std::collections::BTreeMap<String, String> =
            std::collections::BTreeMap::new();
        for t in panel_targets(timeline) {
            for c in target_columns(t) {
                if let (Some(txt), Some(sel)) = (
                    c.get("text").and_then(|v| v.as_str()),
                    c.get("selector").and_then(|v| v.as_str()),
                ) {
                    selectors_by_text.insert(txt.to_string(), sel.to_string());
                }
            }
        }
        for column in &["Job", "Contribution", "Prove", "Aggregate/Wrap", "Execution"] {
            if !columns_text.contains(*column) {
                errors.push(format!(
                    "Phase Share by Proof (last 5 terminal jobs) missing column: {}",
                    column
                ));
            }
        }
        if selectors_by_text.get("Job").map(|s| s.as_str()) != Some("job_label")
            && !(sql_contains(timeline, "job_label") && sql_contains(timeline, "AS \"Job\""))
        {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must use job_label, not raw lane/hash data, for readable lanes"
                    .to_string(),
            );
        }
        if !sql_contains(timeline, "/ NULLIF(") {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must normalize phase shares so outlier durations remain readable"
                    .to_string(),
            );
        }
        let opts = timeline.get("options");
        let orientation = opts.and_then(|v| v.get("orientation")).and_then(|v| v.as_str());
        let stacking = opts.and_then(|v| v.get("stacking")).and_then(|v| v.as_str());
        let x_field = opts.and_then(|v| v.get("xField")).and_then(|v| v.as_str());
        let show_value = opts.and_then(|v| v.get("showValue")).and_then(|v| v.as_str());
        if orientation != Some("horizontal") {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must use horizontal orientation"
                    .to_string(),
            );
        }
        if stacking != Some("normal") {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must stack normalized phase-share segments"
                    .to_string(),
            );
        }
        let unit =
            timeline.get("fieldConfig").and_then(|v| v.get("defaults")).and_then(|v| v.get("unit"));
        if unit.and_then(|v| v.as_str()) != Some("percentunit") {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must render normalized phase shares as percentages"
                    .to_string(),
            );
        }
        if x_field != Some("Job") {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must use Job as the category axis"
                    .to_string(),
            );
        }
        if show_value != Some("never") {
            errors.push(
                "Phase Share by Proof (last 5 terminal jobs) must hide inline value labels"
                    .to_string(),
            );
        }
    }

    // Recent Proof History
    if let Some(panel) = find_panel(&panels, "Recent Proof History") {
        let columns_text: BTreeSet<String> = collect_column_field(panel, "text");
        if !panel_uses_datasource(panel, "zisk-postgres") {
            errors.push("Recent Proof History must use Postgres history".to_string());
        }
        if !sql_contains(panel, "${job_id:singlequote}") {
            errors.push("Recent Proof History must honor the job_id variable".to_string());
        }
        if !sql_contains(panel, "${program:singlequote}") {
            errors.push("Recent Proof History must honor the program variable".to_string());
        }
        if !columns_text.contains("Failure Reason") {
            errors.push("Recent Proof History must expose failure reasons".to_string());
        }
    }

    // Proof Duration Stats (24h)
    if let Some(panel) = find_panel(&panels, "Proof Duration Stats (24h)") {
        if !panel_uses_datasource(panel, "zisk-postgres") {
            errors.push("Proof Duration Stats (24h) must use Postgres history".to_string());
        }
        if !sql_contains(panel, "${program:singlequote}") {
            errors.push("Proof Duration Stats (24h) must honor the program variable".to_string());
        }
        if !sql_contains(panel, "${job_id:singlequote}") {
            errors.push("Proof Duration Stats (24h) must honor the job_id variable".to_string());
        }
    }

    // Program Performance Summary (24h)
    if let Some(panel) = find_panel(&panels, "Program Performance Summary (24h)") {
        let columns_text: BTreeSet<String> = collect_column_field(panel, "text");
        if !panel_uses_datasource(panel, "zisk-postgres") {
            errors.push("Program Performance Summary (24h) must use Postgres history".to_string());
        }
        if !sql_contains_any(panel, &["FROM terminal", "zisk_dashboard_program_performance"]) {
            errors.push(
                "Program Performance Summary (24h) must aggregate terminal history".to_string(),
            );
        }
        if !columns_text.contains("Program") {
            errors
                .push("Program Performance Summary (24h) must expose Program aliases".to_string());
        }
    }

    // HISTOGRAM_SNAPSHOT_PANELS
    for (title, source_field) in HISTOGRAM_SNAPSHOT_PANELS {
        let panel = match find_panel(&panels, title) {
            None => continue,
            Some(p) => p,
        };
        if panel.get("type").and_then(|v| v.as_str()) != Some("histogram") {
            errors.push(format!("{}: snapshot distribution must use a Grafana histogram", title));
        }
        if !panel_uses_datasource(panel, "zisk-postgres") {
            errors.push(format!("{}: raw-value histogram must use the Postgres datasource", title));
        }
        let w = panel.get("gridPos").and_then(|v| v.get("w")).and_then(|v| v.as_u64()).unwrap_or(0);
        if w < 8 {
            errors.push(format!("{}: histogram panel is too narrow to read", title));
        }
        if options_u64(panel, "bucketCount").unwrap_or(0) < 10 {
            errors.push(format!("{}: histogram must keep at least 10 buckets", title));
        }
        if !sql_contains(panel, "${job_id:singlequote}") {
            errors.push(format!("{}: raw-value histogram must honor the job_id variable", title));
        }
        if !sql_contains(panel, "${program:singlequote}") {
            errors.push(format!("{}: raw-value histogram must honor the program variable", title));
        }
        let columns_text: BTreeSet<String> = collect_column_field(panel, "text");
        if columns_text.is_empty() {
            errors.push(format!("{}: histogram target must expose a raw numeric field", title));
        }
        if !sql_contains(panel, source_field) {
            errors.push(format!("{}: histogram must read {}", title, source_field));
        }
    }

    // Proof Duration by Cost (all proofs)
    if let Some(panel) = find_panel(&panels, "Proof Duration by Cost (all proofs)") {
        if panel.get("type").and_then(|v| v.as_str()) != Some("xychart") {
            errors.push("Proof Duration by Cost (all proofs) must use an XY dot plot".to_string());
        }
        if !panel_uses_datasource(panel, "zisk-postgres") {
            errors
                .push("Proof Duration by Cost (all proofs) must use Postgres history".to_string());
        }
        let xy_mapping_ok = panel
            .get("options")
            .and_then(|v| v.get("series"))
            .and_then(|v| v.as_array())
            .map(|series| {
                series.iter().any(|mapping| {
                    mapping.get("x").and_then(|v| v.as_str()) == Some("Cost (M cycles)")
                        && mapping.get("y").and_then(|v| v.as_str()) == Some("Duration (min)")
                })
            })
            .unwrap_or(false);
        if !xy_mapping_ok {
            errors.push(
                "Proof Duration by Cost (all proofs) must map cost to x and duration to y"
                    .to_string(),
            );
        }
        let columns_text: BTreeSet<String> = collect_column_field(panel, "text");
        for column in &["Cost (M cycles)", "Duration (min)", "Proof"] {
            if !columns_text.contains(*column) {
                errors.push(format!(
                    "Proof Duration by Cost (all proofs) missing column: {}",
                    column
                ));
            }
        }
        if columns_text.contains("Steps") {
            errors.push(
                "Proof Duration by Cost (all proofs) must not expose Steps as a second y-series"
                    .to_string(),
            );
        }
        if !sql_contains(panel, "executed_steps") || !sql_contains(panel, "duration_ms") {
            errors.push(
                "Proof Duration by Cost (all proofs) must sort duration_ms by executed_steps"
                    .to_string(),
            );
        }
        if sql_contains(panel, "Proof Order") {
            errors.push(
                "Proof Duration by Cost (all proofs) must not replace cycles with proof order"
                    .to_string(),
            );
        }
        if sql_contains(panel, "ROW_NUMBER") {
            errors.push(
                "Proof Duration by Cost (all proofs) must not fake x-axis separation for duplicate cycle counts"
                    .to_string(),
            );
        }
        if sql_contains(panel, "$__timeFilter") || sql_contains(panel, "INTERVAL '86400 seconds'") {
            errors.push(
                "Proof Duration by Cost (all proofs) must not apply a time-range filter"
                    .to_string(),
            );
        }
    }

    // PROMETHEUS_TREND_PANELS
    for (title, metric) in PROMETHEUS_TREND_PANELS {
        let panel = match find_panel(&panels, title) {
            None => continue,
            Some(p) => p,
        };
        if panel.get("type").and_then(|v| v.as_str()) != Some("timeseries") {
            errors.push(format!("{}: workload trend must remain a timeseries", title));
        }
        if datasource_uid(panel) != Some("prometheus") {
            errors.push(format!("{}: workload trend must use Prometheus", title));
        }
        for target in panel_targets(panel) {
            let expr = target_str(target, "expr");
            let instant = target_bool(target, "instant");
            if instant {
                errors.push(format!("{}: workload trend target must be a range query", title));
            }
            if !expr.contains(metric) || !expr.contains("increase(") || !expr.contains("sum by") {
                errors.push(format!(
                    "{}: workload trend must query {} with increase() summed by label",
                    title, metric
                ));
            }
            if !expr.contains("program=~\"$program\"") {
                errors.push(format!("{}: workload trend must honor the program variable", title));
            }
            let legend = target_str(target, "legendFormat");
            if *title == "Stage Utilization by Phase (15m)" && !legend.contains("{{phase}}") {
                errors.push(format!("{} legend must use the phase label", title));
            }
            if *title == "Executed Cycles Rate by Program (15m)" && !legend.contains("{{program}}")
            {
                errors.push(format!("{} legend must use the program alias label", title));
            }
        }
    }

    // REQUIRED_METRIC_PANELS sweep (must query, p95 histogram_quantile, legends)
    for (title, metric) in REQUIRED_METRIC_PANELS {
        let panel = match find_panel(&panels, title) {
            None => continue,
            Some(p) => p,
        };
        let exprs: Vec<String> =
            panel_targets(panel).iter().map(|t| target_str(t, "expr").to_string()).collect();
        if !exprs.iter().any(|e| e.contains(metric)) {
            errors.push(format!("{} must query expected metric {}", title, metric));
        }
        if (*title == "Proof Duration p95 by Program (15m)"
            || *title == "Phase Duration p95 by Program and Phase (15m)")
            && !exprs.iter().any(|e| e.contains("histogram_quantile") && e.contains("_bucket"))
        {
            errors.push(format!(
                "{} must compute p95 with histogram_quantile over histogram buckets",
                title
            ));
        }
        if *title == "Proof Duration p95 by Program (15m)"
            && exprs.iter().any(|e| !histogram_quantile_re().is_match(e))
        {
            errors.push(
                "Proof Duration p95 by Program (15m) must use coordinator_job_duration_seconds_bucket, \
                 not quantile selectors on coordinator_job_duration_seconds"
                    .to_string(),
            );
        }
        if *title == "Proof Duration p95 by Program (15m)"
            || *title == "Phase Duration p95 by Program and Phase (15m)"
        {
            let legends: Vec<String> = panel_targets(panel)
                .iter()
                .map(|t| target_str(t, "legendFormat").to_string())
                .collect();
            if !legends.iter().any(|l| l.contains("{{program}}")) {
                errors.push(format!("{} legend must use the program alias label", title));
            }
        }
        if *title == "Worker Heartbeat Lag by Worker" {
            let legends: Vec<String> = panel_targets(panel)
                .iter()
                .map(|t| target_str(t, "legendFormat").to_string())
                .collect();
            if !legends.iter().any(|l| l.contains("{{worker_id}}")) {
                errors.push(
                    "Worker Heartbeat Lag by Worker legend must use the worker_id label"
                        .to_string(),
                );
            }
        }
    }

    // Worker Roster is a fleet view: it must NOT pass program / job_id to
    // coord, because the workers endpoint drops idle workers (program=null)
    // whenever a program filter is set and rejects non-UUID job_id values
    // with HTTP 400; both make the roster render empty between jobs.
    if let Some(panel) = find_panel(&panels, "Worker Roster") {
        let urls: Vec<String> =
            panel_targets(panel).iter().map(|t| target_str(t, "url").to_string()).collect();
        if !urls.iter().any(|u| u.contains("/api/v1/workers")) {
            errors.push("Worker Roster must use the expected /api/v1/workers endpoint".to_string());
        }
        if urls.iter().any(|u| u.contains("program=$program")) {
            errors.push(
                "Worker Roster must not pass $program (coord drops idle workers when filtered)"
                    .to_string(),
            );
        }
        if urls.iter().any(|u| u.contains("job_id=$job_id")) {
            errors.push(
                "Worker Roster must not pass $job_id (coord 400s on non-UUID textbox input)"
                    .to_string(),
            );
        }
    }

    // Worker Error Events
    if let Some(panel) = find_panel(&panels, "Worker Error Events") {
        if !panel_uses_datasource(panel, "zisk-postgres") {
            errors.push("Worker Error Events must use Postgres worker error history".to_string());
        }
        if !sql_contains_any(panel, &["job_history_worker_errors", "zisk_dashboard_worker_errors"])
        {
            errors.push("Worker Error Events must read worker error history".to_string());
        }
        if !sql_contains(panel, "${program:singlequote}") {
            errors.push("Worker Error Events must honor the program variable".to_string());
        }
        if !sql_contains(panel, "${job_id:singlequote}") {
            errors.push("Worker Error Events must honor the job_id variable".to_string());
        }
    }

    // Infrastructure Health (top-level row)
    let infrastructure = find_top_panel(dashboard, "Infrastructure Health");
    match infrastructure {
        None => errors
            .push("Infrastructure Health row must exist for low-level SRE plumbing".to_string()),
        Some(panel) => {
            let ty = panel.get("type").and_then(|v| v.as_str());
            let collapsed = panel.get("collapsed").and_then(|v| v.as_bool());
            if ty != Some("row") || collapsed != Some(true) {
                errors.push("Infrastructure Health row must be collapsed by default".to_string());
            }
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::Contract;

    fn empty_contract() -> Contract {
        Contract::from_sets(
            ["coordinator_active_jobs", "coordinator_info"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            ["/api/v1/workers", "/api/v1/jobs/current"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
        )
    }

    #[test]
    fn contract_loads_metrics_and_routes() {
        let raw = r#"{"metrics":["coordinator_x"],"routes":["/api/v1/foo"]}"#;
        let dir = std::env::temp_dir();
        let path = dir.join("zisk-dashboard-validator-test-contract.json");
        std::fs::write(&path, raw).unwrap();
        let c = Contract::load(&path).unwrap();
        assert!(c.metrics.contains("coordinator_x"));
        assert!(c.routes.contains("/api/v1/foo"));
    }

    #[test]
    fn banned_url_fragment_detected() {
        let dashboard: Value = serde_json::from_str(
            r#"{
                "refresh": "1s",
                "uid": "zisk-dev",
                "templating": {"list": []},
                "panels": [
                    {"title": "x", "targets": [{"url": "http://localhost/foo"}]}
                ]
            }"#,
        )
        .unwrap();
        let errors = validate(&dashboard, &empty_contract());
        assert!(
            errors.iter().any(|e| e.contains("banned URL fragment 'localhost'")),
            "errors: {:?}",
            errors
        );
    }

    #[test]
    fn missing_required_panel_detected() {
        let dashboard: Value = serde_json::from_str(
            r#"{
                "refresh": "1s",
                "uid": "zisk-dev",
                "templating": {"list": []},
                "panels": []
            }"#,
        )
        .unwrap();
        let errors = validate(&dashboard, &empty_contract());
        assert!(
            errors
                .iter()
                .any(|e| e == "missing required operator panel: Coordinator Availability (now)"),
            "errors: {:?}",
            errors
        );
        assert!(
            errors.iter().any(|e| e == "missing required operator panel: Active Proofs (now)"),
            "errors: {:?}",
            errors
        );
    }

    #[test]
    fn promql_metric_names_extracts_metrics() {
        let expr = "sum by (kind) (rate(coordinator_jobs_total{kind=\"prove\"}[5m]))";
        let names = promql_metric_names(expr);
        assert!(names.contains("coordinator_jobs_total"));
        // rate is a function, not a metric.
        assert!(!names.contains("rate"));
    }

    #[test]
    fn top_level_grid_overlap_detected() {
        let dashboard: Value = serde_json::from_str(
            r#"{
                "panels": [
                    {"id": 1, "title": "A", "gridPos": {"x": 0, "y": 0, "w": 12, "h": 4}},
                    {"id": 2, "title": "B", "gridPos": {"x": 6, "y": 2, "w": 12, "h": 4}}
                ]
            }"#,
        )
        .unwrap();
        let mut errors = Vec::new();
        validate_top_level_grid(&dashboard, &mut errors);
        assert_eq!(errors, vec!["A overlaps B"]);
    }
}
