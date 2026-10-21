//! The Prometheus text exposition format, written by hand for what the `metrics` facade cannot
//! express: series read from state when `/metrics` is scraped, samples carrying their own
//! timestamp, and the process's own `process_*` metrics.

use std::fmt::Write;
use std::sync::{LazyLock, Mutex};

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

/// Append a family's `# HELP` and `# TYPE` lines; `kind` is `gauge` or `counter`.
pub fn family(out: &mut String, name: &str, kind: &str, help: &str) {
    let _ = writeln!(out, "# HELP {name} {help}");
    let _ = writeln!(out, "# TYPE {name} {kind}");
}

/// Append one sample, with an explicit timestamp in milliseconds when `at_ms` is given.
pub fn sample(
    out: &mut String,
    name: &str,
    labels: &[(&str, &str)],
    value: f64,
    at_ms: Option<i64>,
) {
    out.push_str(name);
    if !labels.is_empty() {
        out.push('{');
        for (i, (key, value)) in labels.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(out, "{key}=\"");
            escape_label(out, value);
            out.push('"');
        }
        out.push('}');
    }
    let _ = write!(out, " {value}");
    if let Some(at) = at_ms {
        let _ = write!(out, " {at}");
    }
    out.push('\n');
}

/// A label value in the text format: `\`, `"` and line breaks escaped.
fn escape_label(out: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
}

/// Append this process's standard metrics: `process_resident_memory_bytes`,
/// `process_cpu_seconds_total` and `process_start_time_seconds`.
pub fn process_metrics(out: &mut String) {
    static SYSTEM: LazyLock<Mutex<System>> = LazyLock::new(|| Mutex::new(System::new()));

    let Ok(pid) = sysinfo::get_current_pid() else { return };
    let mut system = SYSTEM.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        false,
        ProcessRefreshKind::nothing().with_memory().with_cpu(),
    );
    let Some(process) = system.process(pid) else { return };
    family(out, "process_resident_memory_bytes", "gauge", "Resident memory size in bytes.");
    sample(out, "process_resident_memory_bytes", &[], process.memory() as f64, None);
    family(
        out,
        "process_cpu_seconds_total",
        "counter",
        "Total user and system CPU time spent in seconds.",
    );
    sample(
        out,
        "process_cpu_seconds_total",
        &[],
        process.accumulated_cpu_time() as f64 / 1000.0,
        None,
    );
    family(
        out,
        "process_start_time_seconds",
        "gauge",
        "Start time of the process since unix epoch in seconds.",
    );
    sample(out, "process_start_time_seconds", &[], process.start_time() as f64, None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_values_are_escaped_and_timestamps_follow_the_value() {
        let mut out = String::new();
        sample(&mut out, "m", &[("a", "x\"y\\z\nw")], 1.5, Some(42));
        assert_eq!(out, "m{a=\"x\\\"y\\\\z\\nw\"} 1.5 42\n");
    }

    #[test]
    fn a_sample_without_labels_has_no_braces() {
        let mut out = String::new();
        sample(&mut out, "m", &[], 3.0, None);
        assert_eq!(out, "m 3\n");
    }

    #[test]
    fn the_process_reports_its_memory_cpu_and_start() {
        let mut out = String::new();
        process_metrics(&mut out);
        for name in [
            "process_resident_memory_bytes ",
            "process_cpu_seconds_total ",
            "process_start_time_seconds ",
        ] {
            assert!(out.lines().any(|l| l.starts_with(name)), "{name} missing in {out}");
        }
    }
}
