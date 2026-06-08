//! Scenario definitions for the dashboard integration harness.

use std::fmt;
use std::str::FromStr;

use anyhow::{anyhow, Error};

/// Coordinator-canonical `phase_code` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PhaseCode {
    Idle = 0,
    Queued = 1,
    Contributions = 2,
    Prove = 3,
    Aggregate = 4,
    Execution = 5,
    Unknown = 255,
}

impl PhaseCode {
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Idle,
            1 => Self::Queued,
            2 => Self::Contributions,
            3 => Self::Prove,
            4 => Self::Aggregate,
            5 => Self::Execution,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Queued => "Queued",
            Self::Contributions => "Contributions",
            Self::Prove => "Prove",
            Self::Aggregate => "Aggregate",
            Self::Execution => "Execution",
            Self::Unknown => "Unknown",
        }
    }

    pub fn code(self) -> u8 {
        match self {
            Self::Idle => 0,
            Self::Queued => 1,
            Self::Contributions => 2,
            Self::Prove => 3,
            Self::Aggregate => 4,
            Self::Execution => 5,
            Self::Unknown => 255,
        }
    }
}

impl fmt::Display for PhaseCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenarioKind {
    CleanProofSuccess,
}

impl ScenarioKind {
    #[allow(dead_code)]
    pub fn slug(self) -> &'static str {
        match self {
            Self::CleanProofSuccess => "clean-proof-success",
        }
    }
}

impl FromStr for ScenarioKind {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "clean-proof-success" => Ok(Self::CleanProofSuccess),
            other => Err(anyhow!("unknown scenario '{other}'; supported: clean-proof-success",)),
        }
    }
}

/// Spec for the `clean-proof-success` flow.
#[derive(Debug, Clone)]
pub struct CleanProofSuccessSpec {
    pub expected_phases: Vec<PhaseCode>,
    pub phase_duration_counts: Vec<PhaseCode>,
    pub heartbeat_lag_max_seconds: f64,
    pub job_kind: String,
}

impl Default for CleanProofSuccessSpec {
    fn default() -> Self {
        Self {
            expected_phases: vec![PhaseCode::Contributions, PhaseCode::Prove, PhaseCode::Aggregate],
            phase_duration_counts: vec![
                PhaseCode::Contributions,
                PhaseCode::Prove,
                PhaseCode::Aggregate,
            ],
            heartbeat_lag_max_seconds: 30.0,
            job_kind: "prove".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenario_kind_round_trips_via_string() {
        let kind: ScenarioKind = "clean-proof-success".parse().unwrap();
        assert_eq!(kind, ScenarioKind::CleanProofSuccess);
        assert_eq!(kind.slug(), "clean-proof-success");
    }

    #[test]
    fn unknown_scenario_returns_error_listing_supported_ones() {
        let error = "bogus".parse::<ScenarioKind>().unwrap_err().to_string();
        assert!(error.contains("clean-proof-success"), "error must list valid scenarios: {error}");
    }

    #[test]
    fn phase_code_round_trips_through_u8_for_known_codes() {
        for phase in [
            PhaseCode::Idle,
            PhaseCode::Queued,
            PhaseCode::Contributions,
            PhaseCode::Prove,
            PhaseCode::Aggregate,
            PhaseCode::Execution,
        ] {
            assert_eq!(PhaseCode::from_u8(phase.code()), phase);
        }
        assert_eq!(PhaseCode::from_u8(255), PhaseCode::Unknown);
        assert_eq!(PhaseCode::from_u8(99), PhaseCode::Unknown);
    }

    #[test]
    fn default_spec_covers_contributions_prove_aggregate() {
        let spec = CleanProofSuccessSpec::default();
        assert_eq!(
            spec.expected_phases,
            vec![PhaseCode::Contributions, PhaseCode::Prove, PhaseCode::Aggregate],
        );
        assert_eq!(spec.phase_duration_counts, spec.expected_phases);
        assert_eq!(spec.heartbeat_lag_max_seconds, 30.0);
        assert_eq!(spec.job_kind, "prove");
    }
}
