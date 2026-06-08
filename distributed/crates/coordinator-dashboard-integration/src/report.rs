//! PASS/FAIL report rendering for the integration harness.

use std::fmt::Write as _;

use crate::assert::Check;

#[derive(Debug, Clone)]
pub struct Stage {
    pub elapsed_seconds: f64,
    pub header: String,
    pub checks: Vec<Check>,
}

#[derive(Debug, Default)]
pub struct Report {
    pub stages: Vec<Stage>,
}

impl Report {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, stage: Stage) {
        self.stages.push(stage);
    }

    pub fn total_checks(&self) -> usize {
        self.stages.iter().map(|stage| stage.checks.len()).sum()
    }

    pub fn passed_checks(&self) -> usize {
        self.stages
            .iter()
            .flat_map(|stage| stage.checks.iter())
            .filter(|check| check.passed)
            .count()
    }

    pub fn is_pass(&self) -> bool {
        self.total_checks() > 0 && self.passed_checks() == self.total_checks()
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        for stage in &self.stages {
            let _ = writeln!(&mut out, "[T+{:.1}s] {}", stage.elapsed_seconds, stage.header);
            for check in &stage.checks {
                let mark = if check.passed { "PASS" } else { "FAIL" };
                let _ = writeln!(&mut out, "  [{mark}] {}", check.label);
                if let Some(detail) = check.detail.as_ref().filter(|_| !check.passed) {
                    let _ = writeln!(&mut out, "         detail: {detail}");
                }
            }
        }
        let total = self.total_checks();
        let passed = self.passed_checks();
        if total == 0 {
            let _ = writeln!(&mut out, "NO ASSERTIONS RUN");
        } else if passed == total {
            let _ = writeln!(&mut out, "ALL PASS ({passed}/{total})");
        } else {
            let _ = writeln!(&mut out, "FAILED ({passed}/{total} passed)");
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_summarises_pass_when_all_checks_pass() {
        let mut report = Report::new();
        report.push(Stage {
            elapsed_seconds: 0.0,
            header: "Observed: phase=Contributions".to_owned(),
            checks: vec![Check::pass("active_jobs=1"), Check::pass("workers_running=1")],
        });
        report.push(Stage {
            elapsed_seconds: 12.3,
            header: "Transition Contributions -> Prove".to_owned(),
            checks: vec![Check::pass("phase_code 2 -> 3")],
        });

        assert!(report.is_pass());
        let rendered = report.render();
        assert!(rendered.contains("[T+0.0s]"));
        assert!(rendered.contains("[T+12.3s]"));
        assert!(rendered.contains("ALL PASS (3/3)"));
    }

    #[test]
    fn report_summarises_fail_when_any_check_fails() {
        let mut report = Report::new();
        report.push(Stage {
            elapsed_seconds: 1.0,
            header: "Terminal".to_owned(),
            checks: vec![Check::pass("a"), Check::fail("b", "expected 1 got 2"), Check::pass("c")],
        });
        assert!(!report.is_pass());
        let rendered = report.render();
        assert!(rendered.contains("FAILED (2/3 passed)"));
        assert!(rendered.contains("detail: expected 1 got 2"));
    }

    #[test]
    fn empty_report_is_not_a_pass() {
        let report = Report::new();
        assert!(!report.is_pass());
        let rendered = report.render();
        assert!(rendered.contains("NO ASSERTIONS RUN"));
    }
}
