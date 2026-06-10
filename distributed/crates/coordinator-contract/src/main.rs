//! Extract dashboard-facing metrics and routes from coordinator source.

use std::{env, fs, path::PathBuf, process::ExitCode};

use anyhow::{Context, Result};
use serde::Serialize;

const ROUTER_MATCH_MARKER: &str = "match request.path.as_ref()";

#[derive(Debug, Serialize)]
struct Contract {
    metrics: Vec<String>,
    routes: Vec<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("extract-coordinator-contract: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let mut source: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--source" => {
                source = Some(PathBuf::from(iter.next().context("--source needs a path")?))
            }
            "--output" => {
                output = Some(PathBuf::from(iter.next().context("--output needs a path")?))
            }
            "-h" | "--help" => {
                println!(
                    "usage: extract-coordinator-contract [--source PATH] [--output PATH]\n  --source defaults to ../coordinator-server/src/metrics.rs relative to the crate\n  --output defaults to stdout"
                );
                return Ok(());
            }
            other => anyhow::bail!("unknown argument: {other}"),
        }
    }

    let source = source.unwrap_or_else(default_source);
    let text = fs::read_to_string(&source)
        .with_context(|| format!("read coordinator-server source at {}", source.display()))?;

    let mut metrics = extract_metrics(&text);
    metrics.sort();
    metrics.dedup();

    let mut routes = extract_routes(&text);
    routes.sort();
    routes.dedup();

    let contract = Contract { metrics, routes };
    let json = serde_json::to_string_pretty(&contract).context("serialize contract as JSON")?;

    match output {
        Some(path) => fs::write(&path, json + "\n")
            .with_context(|| format!("write contract to {}", path.display()))?,
        None => println!("{json}"),
    }
    Ok(())
}

/// Extract metric names and histogram companion series.
fn extract_metrics(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (kind, marker) in [
        ("gauge", "metrics::describe_gauge!"),
        ("counter", "metrics::describe_counter!"),
        ("histogram", "metrics::describe_histogram!"),
    ] {
        for hit in find_first_string_arg(source, marker) {
            out.push(hit.clone());
            if kind == "histogram" {
                out.push(format!("{hit}_bucket"));
                out.push(format!("{hit}_count"));
                out.push(format!("{hit}_sum"));
            }
        }
    }
    out
}

/// Extract route literals from the router match arm only.
fn extract_routes(source: &str) -> Vec<String> {
    let Some(match_start) = source.find(ROUTER_MATCH_MARKER) else {
        return Vec::new();
    };
    let after_match = &source[match_start..];
    let Some(block_start) = after_match.find('{') else {
        return Vec::new();
    };
    let block_text = &after_match[block_start + 1..];
    let block = balanced_block(block_text);

    let mut out = Vec::new();
    let mut cursor = block;
    while let Some(quote) = cursor.find('"') {
        let after = &cursor[quote + 1..];
        let Some(end) = after.find('"') else { break };
        let literal = &after[..end];
        let rest = after[end + 1..].trim_start();
        if literal.starts_with('/') && rest.starts_with("=>") {
            out.push(literal.to_owned());
        }
        cursor = &after[end + 1..];
    }
    out
}

/// Return the current block body while respecting nested braces.
fn balanced_block(text: &str) -> &str {
    let mut depth = 1usize;
    for (idx, ch) in text.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &text[..idx];
                }
            }
            _ => {}
        }
    }
    text
}

/// Return the first quoted string argument after each marker call.
fn find_first_string_arg(source: &str, marker: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = source;
    while let Some(start) = cursor.find(marker) {
        let after_marker = &cursor[start + marker.len()..];
        let Some(paren) = after_marker.find('(') else { break };
        let body = &after_marker[paren + 1..];
        if let Some(value) = first_quoted(body) {
            out.push(value);
        }
        cursor = &after_marker[paren + 1..];
    }
    out
}

fn first_quoted(text: &str) -> Option<String> {
    let start = text.find('"')?;
    let rest = &text[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

fn default_source() -> PathBuf {
    let manifest =
        env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    manifest.join("..").join("coordinator-server").join("src").join("metrics.rs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_metrics_picks_up_describe_calls_and_expands_histograms() {
        let source = r#"
            fn register_descriptions() {
                metrics::describe_gauge!("coordinator_active_jobs", "desc");
                metrics::describe_counter!(
                    "coordinator_jobs_total",
                    "desc with newlines"
                );
                metrics::describe_histogram!("coordinator_job_duration_seconds", "desc");
            }
        "#;
        let metrics = extract_metrics(source);
        assert!(metrics.contains(&"coordinator_active_jobs".to_owned()));
        assert!(metrics.contains(&"coordinator_jobs_total".to_owned()));
        assert!(metrics.contains(&"coordinator_job_duration_seconds".to_owned()));
        assert!(metrics.contains(&"coordinator_job_duration_seconds_bucket".to_owned()));
        assert!(metrics.contains(&"coordinator_job_duration_seconds_count".to_owned()));
        assert!(metrics.contains(&"coordinator_job_duration_seconds_sum".to_owned()));
    }

    #[test]
    fn extract_routes_only_scans_the_router_match() {
        let source = r#"
            const FOO: &str = "/api/v1/should-not-leak";

            async fn handle_request(...) {
                match request.path.as_ref() {
                    "/metrics" => metrics_response(),
                    "/api/v1/jobs/current" => current_job_response(...).await,
                    "/api/v1/workers" => workers_response(...).await,
                    path if path.starts_with("/api/v1/jobs/") => job_response(path, history).await,
                    _ => problem_response(404, ...),
                }
            }
        "#;
        let routes = extract_routes(source);
        assert!(routes.contains(&"/metrics".to_owned()));
        assert!(routes.contains(&"/api/v1/jobs/current".to_owned()));
        assert!(routes.contains(&"/api/v1/workers".to_owned()));
        assert!(!routes.contains(&"/api/v1/should-not-leak".to_owned()));
    }

    #[test]
    fn balanced_block_respects_nested_braces() {
        let body = "let x = { 1 + 2 }; \"y\" => 3, } trailing";
        assert_eq!(balanced_block(body), "let x = { 1 + 2 }; \"y\" => 3, ");
    }
}
