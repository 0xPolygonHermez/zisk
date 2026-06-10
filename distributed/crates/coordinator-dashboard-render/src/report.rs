//! PASS/FAIL/SKIP report output for panel render checks.

use crate::assert::{Expectation, OperatorState};
use crate::grafana::PanelResult;

/// Trim notes so each panel report stays one line.
fn truncate(s: &str, limit: usize) -> String {
    let oneline: String = s.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    if oneline.len() <= limit {
        oneline
    } else {
        let mut end = limit;
        while !oneline.is_char_boundary(end) && end > 0 {
            end -= 1;
        }
        format!("{}...", &oneline[..end])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail,
    Skip,
}

impl Verdict {
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Skip => "SKIP",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PanelReport {
    pub title: String,
    pub panel_type: String,
    pub expectation: Expectation,
    pub verdict: Verdict,
    pub rows: usize,
    pub note: Option<String>,
}

impl PanelReport {
    pub fn from_result(
        title: String,
        panel_type: String,
        expectation: Expectation,
        state: OperatorState,
        result: &PanelResult,
        note: Option<String>,
    ) -> Self {
        let must = expectation.must_populate(state);
        let verdict = if result.is_error() {
            // Network / HTTP error is a hard fail when the panel matters.
            if must {
                Verdict::Fail
            } else {
                Verdict::Skip
            }
        } else if result.is_populated() {
            Verdict::Pass
        } else if must {
            Verdict::Fail
        } else {
            Verdict::Skip
        };
        let mut combined_note = note;
        if result.is_error() {
            let summary = result
                .target_errors
                .iter()
                .map(|(idx, msg)| format!("target[{idx}]: {}", truncate(msg, 160)))
                .collect::<Vec<_>>()
                .join("; ");
            combined_note = Some(match combined_note {
                Some(existing) => format!("{existing}; {summary}"),
                None => summary,
            });
        }
        Self { title, panel_type, expectation, verdict, rows: result.rows, note: combined_note }
    }
}

#[derive(Debug, Default)]
pub struct Summary {
    pub total: usize,
    pub pass: usize,
    pub fail: usize,
    pub skip: usize,
    pub failures: Vec<String>,
}

impl Summary {
    pub fn from_reports(reports: &[PanelReport]) -> Self {
        let mut summary = Self { total: reports.len(), ..Self::default() };
        for report in reports {
            match report.verdict {
                Verdict::Pass => summary.pass += 1,
                Verdict::Fail => {
                    summary.fail += 1;
                    summary.failures.push(report.title.clone());
                }
                Verdict::Skip => summary.skip += 1,
            }
        }
        summary
    }
}

pub fn print_panel(report: &PanelReport) {
    let exp = match report.expectation {
        Expectation::AlwaysPopulated => "always",
        Expectation::PopulatedDuringRun => "during-run",
        Expectation::OptionallyPopulated => "optional",
    };
    let extra = match &report.note {
        Some(note) => format!(" ({note})"),
        None => String::new(),
    };
    println!(
        "{verdict} [{exp}] rows={rows} type={ptype} {title}{extra}",
        verdict = report.verdict.label(),
        rows = report.rows,
        ptype = report.panel_type,
        title = report.title,
    );
}

pub fn print_summary(summary: &Summary, state: OperatorState) {
    eprintln!(
        "render-dashboard: state={state} panels={total} pass={pass} fail={fail} skip={skip}",
        total = summary.total,
        pass = summary.pass,
        fail = summary.fail,
        skip = summary.skip,
    );
    if !summary.failures.is_empty() {
        eprintln!("render-dashboard: FAIL panels:");
        for name in &summary.failures {
            eprintln!("- {name}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty() -> PanelResult {
        PanelResult { rows: 0, http_status: 200, target_errors: Vec::new() }
    }

    fn populated(rows: usize) -> PanelResult {
        PanelResult { rows, http_status: 200, target_errors: Vec::new() }
    }

    fn errored() -> PanelResult {
        PanelResult { rows: 0, http_status: 500, target_errors: vec![(0, "boom".to_owned())] }
    }

    #[test]
    fn always_populated_panel_fails_when_empty() {
        let r = PanelReport::from_result(
            "Recent Proof History".to_owned(),
            "table".to_owned(),
            Expectation::AlwaysPopulated,
            OperatorState::Idle,
            &empty(),
            None,
        );
        assert_eq!(r.verdict, Verdict::Fail);
    }

    #[test]
    fn during_run_panel_skips_when_idle() {
        let r = PanelReport::from_result(
            "Current Proof Phase (now)".to_owned(),
            "stat".to_owned(),
            Expectation::PopulatedDuringRun,
            OperatorState::Idle,
            &empty(),
            None,
        );
        assert_eq!(r.verdict, Verdict::Skip);
    }

    #[test]
    fn during_run_panel_fails_when_running_but_empty() {
        let r = PanelReport::from_result(
            "Current Proof Phase (now)".to_owned(),
            "stat".to_owned(),
            Expectation::PopulatedDuringRun,
            OperatorState::Running,
            &empty(),
            None,
        );
        assert_eq!(r.verdict, Verdict::Fail);
    }

    #[test]
    fn populated_result_always_passes() {
        let r = PanelReport::from_result(
            "Coordinator Availability (now)".to_owned(),
            "stat".to_owned(),
            Expectation::AlwaysPopulated,
            OperatorState::Idle,
            &populated(3),
            None,
        );
        assert_eq!(r.verdict, Verdict::Pass);
        assert_eq!(r.rows, 3);
    }

    #[test]
    fn error_on_required_panel_is_fail() {
        let r = PanelReport::from_result(
            "Coordinator Availability (now)".to_owned(),
            "stat".to_owned(),
            Expectation::AlwaysPopulated,
            OperatorState::Idle,
            &errored(),
            None,
        );
        assert_eq!(r.verdict, Verdict::Fail);
        assert!(r.note.as_deref().unwrap_or_default().contains("boom"));
    }

    #[test]
    fn summary_counts_verdicts_and_collects_failures() {
        let reports = vec![
            PanelReport {
                title: "A".to_owned(),
                panel_type: "stat".into(),
                expectation: Expectation::AlwaysPopulated,
                verdict: Verdict::Pass,
                rows: 1,
                note: None,
            },
            PanelReport {
                title: "B".to_owned(),
                panel_type: "stat".into(),
                expectation: Expectation::AlwaysPopulated,
                verdict: Verdict::Fail,
                rows: 0,
                note: None,
            },
            PanelReport {
                title: "C".to_owned(),
                panel_type: "stat".into(),
                expectation: Expectation::OptionallyPopulated,
                verdict: Verdict::Skip,
                rows: 0,
                note: None,
            },
        ];
        let s = Summary::from_reports(&reports);
        assert_eq!(s.total, 3);
        assert_eq!(s.pass, 1);
        assert_eq!(s.fail, 1);
        assert_eq!(s.skip, 1);
        assert_eq!(s.failures, vec!["B".to_owned()]);
    }
}
