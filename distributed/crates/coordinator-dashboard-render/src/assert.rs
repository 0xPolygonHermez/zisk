//! Expected panel population rules for the render checker.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorState {
    Idle,
    Running,
    Terminal,
}

impl OperatorState {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "idle" => Some(Self::Idle),
            "running" => Some(Self::Running),
            "terminal" => Some(Self::Terminal),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Running => "running",
            Self::Terminal => "terminal",
        }
    }
}

impl fmt::Display for OperatorState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expectation {
    AlwaysPopulated,
    PopulatedDuringRun,
    OptionallyPopulated,
}

impl Expectation {
    pub fn must_populate(self, state: OperatorState) -> bool {
        match (self, state) {
            (Self::AlwaysPopulated, _) => true,
            (Self::PopulatedDuringRun, OperatorState::Running) => true,
            (Self::PopulatedDuringRun, _) => false,
            (Self::OptionallyPopulated, _) => false,
        }
    }
}

/// Exact panel-title expectations; missing titles default to optional.
pub const PANEL_STATES: &[(&str, Expectation)] = &[
    ("Coordinator Availability (now)", Expectation::AlwaysPopulated),
    ("Coordinator Availability Timeline", Expectation::AlwaysPopulated),
    ("Workers Connected (now)", Expectation::AlwaysPopulated),
    ("Worker Heartbeat Lag by Worker", Expectation::OptionallyPopulated),
    ("Worker Roster", Expectation::OptionallyPopulated),
    ("Worker Assignments by Worker", Expectation::OptionallyPopulated),
    ("Worker Error Events", Expectation::OptionallyPopulated),
    ("Recent Proof History", Expectation::AlwaysPopulated),
    ("Proof Phase Progress (latest jobs)", Expectation::AlwaysPopulated),
    ("Average Proof Duration (last 10 successes)", Expectation::OptionallyPopulated),
    ("Last Successful Proof Age (now)", Expectation::OptionallyPopulated),
    ("Proof Success Rate (24h)", Expectation::OptionallyPopulated),
    ("Current Proof Phase (now)", Expectation::PopulatedDuringRun),
    ("Current Proof Duration (now)", Expectation::PopulatedDuringRun),
    ("Current Phase Age (now)", Expectation::PopulatedDuringRun),
    ("Progress Update Age (now)", Expectation::PopulatedDuringRun),
    ("Workers Assigned (now)", Expectation::PopulatedDuringRun),
    ("Active Proofs (now)", Expectation::PopulatedDuringRun),
    ("Proof Failure Rate by Kind (5m)", Expectation::OptionallyPopulated),
    ("Proof Failures by Reason (selected range)", Expectation::OptionallyPopulated),
    ("Coordinator Restarts (selected range)", Expectation::OptionallyPopulated),
    ("Proof Duration Distribution (recent jobs)", Expectation::OptionallyPopulated),
    ("Proof Duration Quantiles (24h)", Expectation::OptionallyPopulated),
    ("Proof Duration by Cost (all proofs)", Expectation::OptionallyPopulated),
    ("Stage Utilization by Phase (15m)", Expectation::OptionallyPopulated),
    ("Executed Cycles Rate by Program (15m)", Expectation::OptionallyPopulated),
    ("Proof Duration p95 by Program (15m)", Expectation::OptionallyPopulated),
    ("Phase Duration p95 by Program and Phase (15m)", Expectation::OptionallyPopulated),
    ("Program Performance Summary (24h)", Expectation::OptionallyPopulated),
    ("Proof Duration Stats (24h)", Expectation::OptionallyPopulated),
    ("gRPC Request Rate by Status", Expectation::OptionallyPopulated),
    ("History Writer Queue and Drops", Expectation::OptionallyPopulated),
    ("History DB Latency p95 by Operation", Expectation::OptionallyPopulated),
];

pub fn expectation_for(title: &str) -> Expectation {
    PANEL_STATES
        .iter()
        .find(|(name, _)| *name == title)
        .map(|(_, exp)| *exp)
        .unwrap_or(Expectation::OptionallyPopulated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_state_round_trips() {
        for (literal, expected) in [
            ("idle", OperatorState::Idle),
            ("running", OperatorState::Running),
            ("terminal", OperatorState::Terminal),
        ] {
            let parsed = OperatorState::parse(literal).expect("known state");
            assert_eq!(parsed, expected);
            assert_eq!(parsed.as_str(), literal);
        }
        assert!(OperatorState::parse("nonsense").is_none());
    }

    #[test]
    fn must_populate_resolves_per_state() {
        for state in [OperatorState::Idle, OperatorState::Running, OperatorState::Terminal] {
            assert!(Expectation::AlwaysPopulated.must_populate(state));
        }
        assert!(Expectation::PopulatedDuringRun.must_populate(OperatorState::Running));
        assert!(!Expectation::PopulatedDuringRun.must_populate(OperatorState::Idle));
        assert!(!Expectation::PopulatedDuringRun.must_populate(OperatorState::Terminal));
        for state in [OperatorState::Idle, OperatorState::Running, OperatorState::Terminal] {
            assert!(!Expectation::OptionallyPopulated.must_populate(state));
        }
    }

    #[test]
    fn assertion_table_covers_known_panels() {
        let always = [
            "Coordinator Availability (now)",
            "Recent Proof History",
            "Proof Phase Progress (latest jobs)",
        ];
        for title in always {
            assert_eq!(
                expectation_for(title),
                Expectation::AlwaysPopulated,
                "{title} must be in PANEL_STATES as AlwaysPopulated",
            );
        }
        let during_run =
            ["Current Proof Phase (now)", "Current Proof Duration (now)", "Workers Assigned (now)"];
        for title in during_run {
            assert_eq!(
                expectation_for(title),
                Expectation::PopulatedDuringRun,
                "{title} must be in PANEL_STATES as PopulatedDuringRun",
            );
        }
        assert_eq!(
            expectation_for("Some Brand New Panel That Does Not Exist"),
            Expectation::OptionallyPopulated,
        );
    }

    #[test]
    fn assertion_table_has_no_duplicate_titles() {
        let mut seen = std::collections::HashSet::new();
        for (title, _) in PANEL_STATES {
            assert!(seen.insert(*title), "duplicate entry in PANEL_STATES: {title}");
        }
    }
}
