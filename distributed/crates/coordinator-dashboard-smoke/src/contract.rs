//! Required dashboard titles for the live smoke check.

pub const REQUIRED_PANEL_TITLES: &[&str] = &[
    "Coordinator Availability (now)",
    "Workers Connected (now)",
    "Active Proofs (now)",
    "Average Proof Duration (last 10 successes)",
    "Last Successful Proof Age (now)",
    "Infrastructure Health",
    "Current Proof",
    "Current Proof Phase (now)",
    "Current Proof Duration (now)",
    "Current Phase Age (now)",
    "Progress Update Age (now)",
    "Workers Assigned (now)",
    "Proof Phase Progress (latest jobs)",
    "Reliability",
    "Proof Success Rate (24h)",
    "Proof Duration Stats (24h)",
    "Proof Failure Rate by Kind (5m)",
    "Proof Failures by Reason (selected range)",
    "Proof Performance",
    "Proof Duration Distribution (recent jobs)",
    "Proof Duration Quantiles (24h)",
    "Proof Duration by Cost (all proofs)",
    "Performance Trends",
    "Stage Utilization by Phase (15m)",
    "Executed Cycles Rate by Program (15m)",
    "Proof Duration p95 by Program (15m)",
    "Phase Duration p95 by Program and Phase (15m)",
    "Program Performance Summary (24h)",
    "Worker Fleet",
    "Worker Heartbeat Lag by Worker",
    "Worker Roster",
    "Worker Diagnostics",
    "Worker Assignments by Worker",
    "Worker Error Events",
    "Coordinator Runtime",
    "Coordinator Availability Timeline",
    "Coordinator Restarts (selected range)",
    "Recent Proof History",
    "gRPC Request Rate by Status",
    "History Writer Queue and Drops",
    "History DB Latency p95 by Operation",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_titles_count_matches_dashboard() {
        assert_eq!(REQUIRED_PANEL_TITLES.len(), 41);
    }
}
